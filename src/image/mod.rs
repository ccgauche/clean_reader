use std::{path::Path, thread::JoinHandle};

use imgref::ImgVec;
use ravif::{cleared_alpha, encode_rgba, Config};
use rgb::RGBA;

use crate::utils::{self, sha256};

const IMAGE_EXT: &'static [&'static str] = &[".jpg", ".jpeg", ".png"];

pub fn get_image_url(url: &str) -> (String, Option<JoinHandle<()>>) {
    let hash = sha256(url);
    let path = format!("cache/images/{}.avif", &hash[..8]);
    let cache_file = Path::new(&path);
    if cache_file.exists() {
        (format!("/i/{}", &hash[..8]), None)
    } else {
        if IMAGE_EXT.iter().any(|x| url.ends_with(x)) {
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
    let mut img = load_rgba(&image, false)?;
    img = cleared_alpha(img);
    let (out_data, color_size, alpha_size) = encode_rgba(
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

fn load_rgba(mut data: &[u8], premultiplied_alpha: bool) -> anyhow::Result<ImgVec<RGBA<u8>>> {
    use rgb::FromSlice;

    let mut img = if data.get(0..4) == Some(&[0x89, b'P', b'N', b'G']) {
        let img = lodepng::decode32(data)?;
        ImgVec::new(img.buffer, img.width, img.height)
    } else {
        let mut jecoder = jpeg_decoder::Decoder::new(&mut data);
        let pixels = jecoder.decode()?;
        let info = jecoder
            .info()
            .ok_or(anyhow::anyhow!("Error reading JPEG info"))?;
        use jpeg_decoder::PixelFormat::*;
        let buf: Vec<_> = match info.pixel_format {
            L8 => pixels
                .iter()
                .copied()
                .map(|g| RGBA::new(g, g, g, 255))
                .collect(),
            RGB24 => {
                let rgb = pixels.as_rgb();
                rgb.iter().map(|p| p.alpha(255)).collect()
            }
            CMYK32 => {
                return Err(anyhow::anyhow!(
                    "CMYK JPEG is not supported. Please convert to PNG first"
                ))
            }
        };
        ImgVec::new(buf, info.width.into(), info.height.into())
    };
    if premultiplied_alpha {
        img.pixels_mut().for_each(|px| {
            px.r = (px.r as u16 * px.a as u16 / 255) as u8;
            px.g = (px.g as u16 * px.a as u16 / 255) as u8;
            px.b = (px.b as u16 * px.a as u16 / 255) as u8;
        });
    }
    Ok(img)
}
