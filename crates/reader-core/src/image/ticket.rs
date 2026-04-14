use std::sync::mpsc::Receiver;

/// Ticket returned by [`super::get_image_url`] while an image re-encode
/// is in flight. The template renderer waits on it at the end of
/// `render_article` so the `/i/{hash}.avif` cache file lands on disk
/// before the HTTP response goes out.
pub struct ImageTicket {
    pub done: Receiver<()>,
}
