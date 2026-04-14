use super::ImageTicket;

/// The outcome of [`super::get_image_url`]: the URL to actually emit in
/// the final `<img src>` (either the re-encoded `/i/{hash}` path or the
/// original remote URL), plus an optional [`ImageTicket`] the template
/// renderer must wait on if re-encoding is in flight.
pub struct ResolvedImage {
    pub url: String,
    pub ticket: Option<ImageTicket>,
}
