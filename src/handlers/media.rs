use actix_files::NamedFile;
use actix_web::{web, HttpRequest, HttpResponse};
use std::path::PathBuf;

use crate::config::Config;
use crate::errors::ApiError;

/// GET /uploads/{path:.*}
/// Sirve archivos estáticos subidos (imágenes, thumbnails, etc.)
pub async fn serve_upload(
    config: web::Data<Config>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let relative_path = path.into_inner();

    // Normalizar dobles barras
    let normalized = relative_path.replace("//", "/");

    // Prevenir path traversal
    if normalized.contains("..") {
        return Err(ApiError::bad_request("Ruta no válida"));
    }

    let file_path = PathBuf::from(format!("{}/{}", config.upload_dir, normalized));

    if !file_path.exists() {
        return Err(ApiError::not_found("Archivo no encontrado"));
    }

    let file = NamedFile::open_async(&file_path)
        .await
        .map_err(|e| ApiError::internal(format!("Error abriendo archivo: {}", e)))?;

    Ok(file.into_response(&req))
}
