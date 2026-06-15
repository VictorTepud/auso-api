use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub server_host: String,
    pub server_port: u16,
    pub upload_dir: String,
    pub max_image_size_mb: u64,
    pub max_video_size_mb: u64,
    pub max_images_per_pack: u32,
    pub hls_segment_duration: u32,
    pub video_target_height: u32,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./auso.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "auso_secret_change_in_production".to_string()),
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "168".to_string())
                .parse()
                .unwrap_or(168),
            server_host: env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            upload_dir: env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "./uploads".to_string()),
            max_image_size_mb: env::var("MAX_IMAGE_SIZE_MB")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            max_video_size_mb: env::var("MAX_VIDEO_SIZE_MB")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            max_images_per_pack: env::var("MAX_IMAGES_PER_PACK")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .unwrap_or(15),
            hls_segment_duration: env::var("HLS_SEGMENT_DURATION")
                .unwrap_or_else(|_| "6".to_string())
                .parse()
                .unwrap_or(6),
            video_target_height: env::var("VIDEO_TARGET_HEIGHT")
                .unwrap_or_else(|_| "360".to_string())
                .parse()
                .unwrap_or(360),
        }
    }

    pub fn max_image_size_bytes(&self) -> u64 {
        self.max_image_size_mb * 1024 * 1024
    }

    pub fn max_video_size_bytes(&self) -> u64 {
        self.max_video_size_mb * 1024 * 1024
    }
}
