use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/dist/"]
struct Assets;

#[must_use]
pub fn get(path: &str) -> Option<(Cow<'static, [u8]>, String)> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();
    Some((file.data, mime))
}
