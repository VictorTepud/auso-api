use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedImage {
    pub url: String,
    pub filename: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingResult {
    pub hls_master_playlist_url: String,
    pub hls_directory: String,
    pub duration: f64,
    pub width: i32,
    pub height: i32,
    pub thumbnail_url: Option<String>,
}
