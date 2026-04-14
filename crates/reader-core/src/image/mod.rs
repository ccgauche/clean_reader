//! Image re-encoding helpers shared across crates.
//!
//! reader-core knows how to decode any format `image` supports and hand
//! ravif a tightly-packed RGBA buffer. What it does not know is *where*
//! the re-encode happens — that's the image-actor crate's job. To keep
//! the dep graph acyclic, the image-actor registers a closure here at
//! boot time via [`register_encoder`]; `get_image_url` calls through the
//! registered closure when the template renderer asks for an image.

mod encoder;
mod error;
mod resolved;
mod ticket;

pub use encoder::{encode_avif, get_image_url, register_encoder, EncoderFn};
pub use error::ImageError;
pub use resolved::ResolvedImage;
pub use ticket::ImageTicket;
