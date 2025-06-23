use std::path::Path;

pub fn is_media_file(file_name: &str) -> bool {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or(String::from(""));
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "mov" | "mp4")
}