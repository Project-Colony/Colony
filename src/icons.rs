//! Decoding cached PNG app icons into iced image handles.
//!
//! iced is built with `image-without-codecs`, so it renders pre-decoded RGBA
//! but ships no decoder. We decode the (small) PNG ourselves with the `image`
//! crate (png feature only) once, then hand iced an `image::Handle::from_rgba`.
//! Handles are cheap-clone (Arc-backed) and stored in `App::app_icons`, so a
//! card never re-decodes in `view()`.

use iced::widget::image::Handle;

/// Decode PNG bytes into an iced RGBA image handle. Returns `None` on any
/// decode error (unsupported/corrupt file) so the caller falls back to the
/// hexagon tile.
pub fn decode_icon(bytes: &[u8]) -> Option<Handle> {
    let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(Handle::from_rgba(width, height, rgba.into_raw()))
}
