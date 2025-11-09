use std::path::PathBuf;

pub fn manifest_path<'a>(segments: impl IntoIterator<Item = &'a str>) -> PathBuf {
    let segments = segments.into_iter();
    let full_path = [env!("CARGO_MANIFEST_DIR")].into_iter().chain(segments);
    full_path.collect()
}

pub fn relative_path<'a>(segments: impl IntoIterator<Item = &'a str>) -> PathBuf {
    segments.into_iter().collect()
}
