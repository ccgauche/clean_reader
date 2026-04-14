//! Sha256-hex helper. Used by the URL short-id store and the image
//! re-encode cache path builder.

use sha2::{Digest, Sha256};

pub fn sha256(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
