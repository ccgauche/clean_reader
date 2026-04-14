//! Image decoding, ravif re-encoding, and the
//! `<url> -> (rewritten URL, optional ticket)` resolver.
//!
//! Cross-crate wiring goes through [`register_encoder`]: the image-actor
//! crate boots, spawns its worker, and registers a closure here so
//! reader-core can trigger re-encodes without depending on the actor
//! layer directly.

use std::{io::Cursor, path::Path, path::PathBuf};

use image::io::Reader;
use imgref::ImgVec;
use once_cell::sync::OnceCell;
use ravif::Encoder;
use rgb::RGBA;

use crate::{config::CONFIG, utils::sha256};

use super::{ImageError, ImageTicket, ResolvedImage};

type Result<T> = std::result::Result<T, ImageError>;

/// Image-URL suffixes the re-encoder is willing to attempt. Anything else
/// (SVGs, webm animations, …) is passed through unchanged.
const REENCODABLE_EXTENSIONS: &[&str] = &[".jpg", ".jpeg", ".png", ".webp", ".bmp", ".avif"];

/// Length of the sha256 prefix used in `/i/{…}.avif` cache paths. 8 hex
/// chars = 32 bits of collision space, which is plenty for a single
/// server's image cache and keeps URLs short.
const IMAGE_HASH_PREFIX_LEN: usize = 8;

/// Function signature of a registered image-encoder backend.
///
/// Called with the source URL and the cache-file path we want the `.avif`
/// written to. Returns a ticket the caller can wait on, or `None` if the
/// backend declines (no worker available, bad URL, …).
pub type EncoderFn = Box<dyn Fn(String, PathBuf) -> Option<ImageTicket> + Send + Sync + 'static>;

static ENCODER: OnceCell<EncoderFn> = OnceCell::new();

/// Install the encoder backend. Called once at server startup by the
/// image-actor crate. Silently ignores a second registration.
pub fn register_encoder(encoder: EncoderFn) {
    let _ = ENCODER.set(encoder);
}

/// Resolve an image URL to its final `<img src>` value, launching a
/// re-encode worker if appropriate.
pub fn get_image_url(url: &str) -> ResolvedImage {
    if !CONFIG.recompress_images {
        return ResolvedImage {
            url: url.to_owned(),
            ticket: None,
        };
    }
    let hash = sha256(url);
    let short_hash = &hash[..IMAGE_HASH_PREFIX_LEN];
    let cache_path = format!("{}/images/{}.avif", CONFIG.cache_folder, short_hash);
    if Path::new(&cache_path).exists() {
        return ResolvedImage {
            url: format!("/i/{}", short_hash),
            ticket: None,
        };
    }
    let reencodable = REENCODABLE_EXTENSIONS.iter().any(|ext| url.contains(ext));
    let ticket = reencodable
        .then(|| ENCODER.get())
        .flatten()
        .and_then(|encoder| encoder(url.to_owned(), PathBuf::from(&cache_path)));
    if ticket.is_some() {
        return ResolvedImage {
            url: format!("/i/{}", short_hash),
            ticket,
        };
    }
    ResolvedImage {
        url: url.to_owned(),
        ticket: None,
    }
}

pub fn encode_avif(image: &[u8]) -> Result<Vec<u8>> {
    let img = load_rgba(image)?;
    let result = Encoder::new()
        .with_quality(50.0)
        .with_alpha_quality(50.0)
        .with_speed(5)
        .encode_rgba(img.as_ref())
        .map_err(|e: ravif::Error| ImageError::AvifEncode(e.to_string()))?;
    Ok(result.avif_file)
}

/// Decode `bytes` into a tightly-packed RGBA buffer. Unlike the old
/// implementation this no longer has an always-false
/// `premultiplied_alpha` parameter — removing the dead flag drops the
/// helper's only nesting.
fn load_rgba(bytes: &[u8]) -> Result<ImgVec<RGBA<u8>>> {
    let decoded = Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("Cursor io never fails")
        .decode()?;
    to_rgba(decoded)
}

fn to_rgba(decoded: image::DynamicImage) -> Result<ImgVec<RGBA<u8>>> {
    Ok(match decoded {
        image::DynamicImage::ImageLuma8(img) => gray8_to_rgba(&img),
        image::DynamicImage::ImageLumaA8(img) => graya8_to_rgba(&img),
        image::DynamicImage::ImageRgb8(img) => rgb8_to_rgba(&img),
        image::DynamicImage::ImageRgba8(img) => rgba8_to_rgba(&img),
        image::DynamicImage::ImageLuma16(img) => gray16_to_rgba(&img),
        image::DynamicImage::ImageLumaA16(img) => graya16_to_rgba(&img),
        image::DynamicImage::ImageRgb16(img) => rgb16_to_rgba(&img),
        image::DynamicImage::ImageRgba16(img) => rgba16_to_rgba(&img),
        image::DynamicImage::ImageRgb32F(img) => rgb32f_to_rgba(&img),
        image::DynamicImage::ImageRgba32F(img) => rgba32f_to_rgba(&img),
        _ => return Err(ImageError::UnsupportedFormat),
    })
}

fn gray8_to_rgba(img: &image::GrayImage) -> ImgVec<RGBA<u8>> {
    let pixels = img.pixels().map(|p| RGBA::new(p[0], p[0], p[0], 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn graya8_to_rgba(img: &image::GrayAlphaImage) -> ImgVec<RGBA<u8>> {
    let pixels = img.pixels().map(|p| RGBA::new(p[0], p[0], p[0], 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgb8_to_rgba(img: &image::RgbImage) -> ImgVec<RGBA<u8>> {
    let pixels = img.pixels().map(|p| RGBA::new(p[0], p[1], p[2], 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgba8_to_rgba(img: &image::RgbaImage) -> ImgVec<RGBA<u8>> {
    let pixels = img.pixels().map(|p| RGBA::new(p[0], p[1], p[2], p[3]));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn gray16_to_rgba(img: &image::ImageBuffer<image::Luma<u16>, Vec<u16>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[0] as u8, p[0] as u8, 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn graya16_to_rgba(img: &image::ImageBuffer<image::LumaA<u16>, Vec<u16>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[0] as u8, p[0] as u8, 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgb16_to_rgba(img: &image::ImageBuffer<image::Rgb<u16>, Vec<u16>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[1] as u8, p[2] as u8, 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgba16_to_rgba(img: &image::ImageBuffer<image::Rgba<u16>, Vec<u16>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgb32f_to_rgba(img: &image::ImageBuffer<image::Rgb<f32>, Vec<f32>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[1] as u8, p[2] as u8, 255));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}

fn rgba32f_to_rgba(img: &image::ImageBuffer<image::Rgba<f32>, Vec<f32>>) -> ImgVec<RGBA<u8>> {
    let pixels = img
        .pixels()
        .map(|p| RGBA::new(p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8));
    ImgVec::new(
        pixels.collect(),
        img.width() as usize,
        img.height() as usize,
    )
}
