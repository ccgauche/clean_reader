//! Image re-encoding helpers shared across crates.
//!
//! reader-core knows how to decode any format `image` supports and hand
//! ravif a tightly-packed RGBA buffer. What it does not know is *where* the
//! re-encode happens — that's the image-actor crate's job. To keep the dep
//! graph acyclic, the image-actor registers a closure here at boot time via
//! [`register_encoder`]; `get_image_url` calls through the registered
//! closure when the template renderer asks for an image.

use std::{io::Cursor, path::Path, path::PathBuf, sync::mpsc::Receiver};

use image::io::Reader;
use imgref::ImgVec;
use once_cell::sync::OnceCell;
use ravif::Encoder;
use rgb::RGBA;

use crate::{
    config::CONFIG,
    error::{Error, Result},
    utils::sha256,
};

const IMAGE_EXT: &[&str] = &[".jpg", ".jpeg", ".png", ".webp", ".bmp", ".avif"];

/// Ticket returned by `get_image_url` while an image re-encode is in flight.
/// The template renderer waits on it at the end of `gen_html_2` to ensure
/// the `/i/{hash}.avif` file is on disk before the response is sent.
pub struct ImageTicket {
    pub done: Receiver<()>,
}

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

pub fn get_image_url(url: &str) -> (String, Option<ImageTicket>) {
    if !CONFIG.recompress_images {
        return (url.to_owned(), None);
    }
    let hash = sha256(url);
    let path = format!("{}/images/{}.avif", CONFIG.cache_folder, &hash[..8]);
    let cache_file = Path::new(&path);
    if cache_file.exists() {
        return (format!("/i/{}", &hash[..8]), None);
    }
    if IMAGE_EXT.iter().any(|x| url.contains(x)) {
        if let Some(encoder) = ENCODER.get() {
            if let Some(ticket) = encoder(url.to_owned(), cache_file.to_owned()) {
                return (format!("/i/{}", &hash[..8]), Some(ticket));
            }
        }
    }
    (url.to_owned(), None)
}

pub fn encode_avif(image: &[u8]) -> Result<Vec<u8>> {
    let img = load_rgba(image, false)?;
    let result = Encoder::new()
        .with_quality(50.0)
        .with_alpha_quality(50.0)
        .with_speed(5)
        .encode_rgba(img.as_ref())
        .map_err(|x: ravif::Error| Error::AvifEncode(x.to_string()))?;
    Ok(result.avif_file)
}

fn load_rgba(data: &[u8], premultiplied_alpha: bool) -> Result<ImgVec<RGBA<u8>>> {
    let mut img = decode(data)?;
    if premultiplied_alpha {
        img.pixels_mut().for_each(|px| {
            px.r = (px.r as u16 * px.a as u16 / 255) as u8;
            px.g = (px.g as u16 * px.a as u16 / 255) as u8;
            px.b = (px.b as u16 * px.a as u16 / 255) as u8;
        });
    }
    Ok(img)
}

fn decode(bytes: &[u8]) -> Result<ImgVec<RGBA<u8>>> {
    let img = Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("Cursor io never fails")
        .decode()?;
    Ok(match img {
        image::DynamicImage::ImageLuma8(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0], x[0], x[0], 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageLumaA8(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0], x[0], x[0], 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgb8(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0], x[1], x[2], 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgba8(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0], x[1], x[2], x[3]))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageLuma16(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[0] as u8, x[0] as u8, 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageLumaA16(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[0] as u8, x[0] as u8, 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgb16(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[1] as u8, x[2] as u8, 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgba16(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[1] as u8, x[2] as u8, x[3] as u8))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgb32F(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[1] as u8, x[2] as u8, 255))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        image::DynamicImage::ImageRgba32F(img) => ImgVec::new(
            img.pixels()
                .map(|x| RGBA::new(x[0] as u8, x[1] as u8, x[2] as u8, x[3] as u8))
                .collect(),
            img.width() as usize,
            img.height() as usize,
        ),
        _ => return Err(Error::UnsupportedImageFormat),
    })
}
