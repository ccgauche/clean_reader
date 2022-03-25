use std::{io::Cursor, path::Path, thread::JoinHandle};

use image::io::Reader;
use imgref::ImgVec;
use ravif::{cleared_alpha, encode_rgba, Config};
use rgb::RGBA;

use crate::{
    config::CONFIG,
    utils::{self, sha256},
};

const IMAGE_EXT: &[&str] = &[".jpg", ".jpeg", ".png", ".webp", ".bmp", ".avif"];

pub fn get_image_url(url: &str) -> (String, Option<JoinHandle<()>>) {
    println!("Compiling image {}", url);
    if !CONFIG.recompress_images {
        return (url.to_owned(), None);
    }
    let hash = sha256(url);
    let path = format!("{}/images/{}.avif", CONFIG.cache_folder, &hash[..8]);
    let cache_file = Path::new(&path);
    if cache_file.exists() {
        (format!("/i/{}", &hash[..8]), None)
    } else {
        if IMAGE_EXT.iter().any(|x| url.contains(x)) {
            let url = url.to_owned();
            let cache_file = cache_file.to_owned();
            let k = std::thread::spawn(move || match utils::http_get_bytes(&url) {
                Ok(e) => match encode(&e) {
                    Ok(e) => {
                        std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
                        std::fs::write(&cache_file, &e).unwrap();
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                },
                Err(e) => {
                    println!("{}", e);
                }
            });
            return (format!("/i/{}", &hash[..8]), Some(k));
        }
        (url.to_owned(), None)
    }
}

fn encode(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut img = load_rgba(image, false)?;
    img = cleared_alpha(img);
    let (out_data, _color_size, _alpha_size) = encode_rgba(
        img.as_ref(),
        &Config {
            quality: 50.0,
            speed: 5,
            alpha_quality: 50.,
            premultiplied_alpha: false,
            color_space: ravif::ColorSpace::YCbCr,
            threads: 0,
        },
    )
    .map_err(|x| anyhow::anyhow!("{}", x.to_string()))?;
    Ok(out_data)
}

fn load_rgba(data: &[u8], premultiplied_alpha: bool) -> anyhow::Result<ImgVec<RGBA<u8>>> {
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

fn decode(bytes: &[u8]) -> anyhow::Result<ImgVec<RGBA<u8>>> {
    println!("decoding image");
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
        _ => unimplemented!(),
    })
}
