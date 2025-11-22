use std::path::PathBuf;

use anyhow::Context;
use image::{DynamicImage, ImageReader};

pub fn manifest_path<'a>(segments: impl IntoIterator<Item = &'a str>) -> PathBuf {
    let segments = segments.into_iter();
    let full_path = [env!("CARGO_MANIFEST_DIR")].into_iter().chain(segments);
    full_path.collect()
}

pub fn relative_path<'a>(segments: impl IntoIterator<Item = &'a str>) -> PathBuf {
    segments.into_iter().collect()
}

pub fn load_image(file_name: &str) -> anyhow::Result<DynamicImage> {
    let file_path = manifest_path(["textures", file_name]);
    let image = ImageReader::open(&file_path)
        .with_context(|| format!("failed to open image: {file_path:?}"))?
        .decode()
        .with_context(|| format!("failed to decode image: {file_path:?}"))?;

    Ok(image)
}
