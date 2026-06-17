use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

use crate::config::Config;
use crate::errors::ApiError;
use crate::models::media::VideoProcessingResult;

/// Procesa un video subido: transcodifica a 360p HLS con segmentación
pub async fn process_video(
    input_path: &str,
    video_id: &str,
    config: &Config,
) -> Result<VideoProcessingResult, ApiError> {
    let upload_dir = &config.upload_dir;
    let hls_dir = format!("{}/videos/{}", upload_dir, video_id);

    // Crear directorio HLS
    fs::create_dir_all(&hls_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Error creando directorio HLS: {}", e)))?;

    let master_playlist = format!("{}/master.m3u8", hls_dir);
    let segment_pattern = format!("{}/segment_%05d.ts", hls_dir);
    let thumbnail_path = format!("{}/thumbnail.jpg", hls_dir);

    // Obtener información del video original
    let video_info = get_video_info(input_path)?;

    // Determinar la resolución objetivo
    let target_height = if video_info.height <= config.video_target_height as i32 {
        video_info.height
    } else {
        config.video_target_height as i32
    };

    let target_width = calculate_width(target_height, video_info.width, video_info.height);

    // Generar thumbnail
    let _ = generate_thumbnail(input_path, &thumbnail_path);

    // Transcodificar a HLS
    transcode_to_hls(
        input_path,
        &master_playlist,
        &segment_pattern,
        target_width,
        target_height,
        config.hls_segment_duration,
    )?;

    // Generar URLs relativas (ocultando la ruta real del servidor)
    let hls_master_url = format!("/api/v1/stream/{}/master.m3u8", video_id);
    let thumbnail_url = if Path::new(&thumbnail_path).exists() {
        Some(format!("/api/v1/stream/{}/thumbnail.jpg", video_id))
    } else {
        None
    };

    Ok(VideoProcessingResult {
        hls_master_playlist_url: hls_master_url,
        hls_directory: hls_dir,
        duration: video_info.duration,
        width: target_width,
        height: target_height,
        thumbnail_url,
    })
}

struct VideoInfo {
    width: i32,
    height: i32,
    duration: f64,
}

fn get_video_info(input_path: &str) -> Result<VideoInfo, ApiError> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            input_path,
        ])
        .output()
        .map_err(|e| ApiError::internal(format!("Error ejecutando ffprobe: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| ApiError::internal(format!("Error parseando ffprobe output: {}", e)))?;

    let video_stream = json["streams"]
        .as_array()
        .and_then(|streams| {
            streams.iter().find(|s| s["codec_type"] == "video")
        })
        .ok_or_else(|| ApiError::bad_request("No se encontró stream de video"))?;

    let width = video_stream["width"]
        .as_i64()
        .unwrap_or(640) as i32;
    let height = video_stream["height"]
        .as_i64()
        .unwrap_or(360) as i32;
    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok(VideoInfo { width, height, duration })
}

fn calculate_width(target_height: i32, original_width: i32, original_height: i32) -> i32 {
    // Mantener aspect ratio, asegurar que el ancho sea par (requerido por H.264)
    let width = (original_width as f64 * target_height as f64 / original_height as f64) as i32;
    if width % 2 != 0 { width + 1 } else { width }
}

fn generate_thumbnail(input_path: &str, output_path: &str) -> Result<(), ApiError> {
    let status = Command::new("ffmpeg")
        .args([
            "-i", input_path,
            "-ss", "00:00:01",
            "-vframes", "1",
            "-q:v", "2",
            "-y",
            output_path,
        ])
        .status()
        .map_err(|e| ApiError::internal(format!("Error generando thumbnail: {}", e)))?;

    if !status.success() {
        tracing::warn!("No se pudo generar thumbnail para: {}", input_path);
    }
    Ok(())
}

fn transcode_to_hls(
    input_path: &str,
    master_playlist: &str,
    segment_pattern: &str,
    width: i32,
    height: i32,
    segment_duration: u32,
) -> Result<(), ApiError> {
    let status = Command::new("ffmpeg")
        .args([
            "-i", input_path,
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "23",
            "-maxrate", "2500k",
            "-bufsize", "5000k",
            "-vf", &format!("scale={}:{}", width, height),
            "-c:a", "aac",
            "-b:a", "128k",
            "-ar", "44100",
            "-ac", "2",
            "-f", "hls",
            "-hls_time", &segment_duration.to_string(),
            "-hls_list_size", "0",
            "-hls_segment_filename", segment_pattern,
            "-hls_flags", "independent_segments",
            "-y",
            master_playlist,
        ])
        .status()
        .map_err(|e| ApiError::internal(format!("Error ejecutando ffmpeg: {}", e)))?;

    if !status.success() {
        return Err(ApiError::internal("Error transcodificando video a HLS"));
    }

    Ok(())
}

/// Obtiene la ruta del archivo de segmento o playlist HLS
pub fn get_hls_file_path(video_id: &str, filename: &str, config: &Config) -> PathBuf {
    PathBuf::from(format!("{}/videos/{}/{}", config.upload_dir, video_id, filename))
}
