use actix_web::{web, HttpResponse};

use crate::config::Config;
use crate::errors::ApiError;
use crate::services::video;

/// GET /api/v1/stream/{video_id}/master.m3u8
/// Sirve el playlist maestro HLS del video
pub async fn stream_hls_master(
    config: web::Data<Config>,
    video_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let video_id = video_id.into_inner();
    let path = video::get_hls_file_path(&video_id, "master.m3u8", config.get_ref());

    if !path.exists() {
        return Err(ApiError::not_found("Video no encontrado o aún procesándose"));
    }

    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| ApiError::internal(format!("Error leyendo playlist: {}", e)))?;

    Ok(HttpResponse::Ok()
        .content_type("application/vnd.apple.mpegurl")
        .insert_header(("Cache-Control", "no-cache"))
        .body(data))
}

/// GET /api/v1/stream/{video_id}/{segment}.ts
/// Sirve los segmentos TS del video
pub async fn stream_hls_segment(
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (video_id, segment) = path.into_inner();

    // Validar que el segmento sea un archivo .ts o .m3u8 válido
    if !segment.ends_with(".ts") && !segment.ends_with(".m3u8") {
        return Err(ApiError::bad_request("Tipo de segmento no válido"));
    }

    // Prevenir path traversal
    if segment.contains("..") || segment.contains("/") || segment.contains("\\") {
        return Err(ApiError::bad_request("Segmento no válido"));
    }

    let file_path = video::get_hls_file_path(&video_id, &segment, config.get_ref());

    if !file_path.exists() {
        return Err(ApiError::not_found("Segmento no encontrado"));
    }

    let content_type = if segment.ends_with(".ts") {
        "video/mp2t"
    } else {
        "application/vnd.apple.mpegurl"
    };

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| ApiError::internal(format!("Error leyendo segmento: {}", e)))?;

    let cache = if segment.ends_with(".ts") {
        "max-age=86400"
    } else {
        "no-cache"
    };

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header(("Cache-Control", cache))
        .body(data))
}

/// GET /api/v1/stream/{video_id}/thumbnail.jpg
pub async fn stream_thumbnail(
    config: web::Data<Config>,
    video_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let video_id = video_id.into_inner();
    let path = video::get_hls_file_path(&video_id, "thumbnail.jpg", config.get_ref());

    if !path.exists() {
        return Err(ApiError::not_found("Thumbnail no disponible"));
    }

    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| ApiError::internal(format!("Error leyendo thumbnail: {}", e)))?;

    Ok(HttpResponse::Ok()
        .content_type("image/jpeg")
        .body(data))
}
