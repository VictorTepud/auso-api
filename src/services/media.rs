use actix_multipart::Multipart;
use futures_util::StreamExt;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::ApiError;
use crate::models::media::UploadedImage;

/// Guarda imágenes subidas y retorna la información del archivo
pub async fn save_image(
    mut payload: Multipart,
    config: &Config,
    subfolder: &str,
) -> Result<Vec<UploadedImage>, ApiError> {
    let mut uploaded = Vec::new();
    let upload_dir = format!("{}/images/{}", config.upload_dir, subfolder);

    fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Error creando directorio: {}", e)))?;

    while let Some(field) = payload.next().await {
        let field = field.map_err(|e| ApiError::bad_request(format!("Error leyendo multipart: {}", e)))?;
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .and_then(|cd| cd.get_filename())
            .unwrap_or("unknown.jpg")
            .to_string();

        let ext = Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");

        let safe_name = sanitize_filename::sanitize(&filename);
        let unique_name = format!("{}_{}.{}", Uuid::new_v4(), safe_name, ext);
        let filepath = format!("{}/{}", upload_dir, unique_name);

        let mut total_size: u64 = 0;
        let mut file = fs::File::create(&filepath)
            .await
            .map_err(|e| ApiError::internal(format!("Error creando archivo: {}", e)))?;

        let mut field_stream = field;
        while let Some(chunk) = field_stream.next().await {
            let data = chunk.map_err(|e| ApiError::bad_request(format!("Error leyendo datos: {}", e)))?;
            total_size += data.len() as u64;

            if total_size > config.max_image_size_bytes() {
                let _ = fs::remove_file(&filepath).await;
                return Err(ApiError::bad_request(format!(
                    "Imagen excede el tamaño máximo de {}MB",
                    config.max_image_size_mb
                )));
            }

            use tokio::io::AsyncWriteExt;
            file.write_all(&data)
                .await
                .map_err(|e| ApiError::internal(format!("Error escribiendo archivo: {}", e)))?;
        }

        uploaded.push(UploadedImage {
            url: format!("/uploads/images/{}/{}", subfolder, unique_name),
            filename: unique_name,
            size: total_size,
        });
    }

    Ok(uploaded)
}

/// Guarda un video subido temporalmente para procesamiento
pub async fn save_video_temp(
    mut payload: Multipart,
    config: &Config,
) -> Result<(PathBuf, String), ApiError> {
    let temp_dir = format!("{}/videos/temp", config.upload_dir);

    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Error creando directorio temporal: {}", e)))?;

    let video_id = Uuid::new_v4().to_string();
    let mut original_filename = String::from("video.mp4");

    if let Some(field) = payload.next().await {
        let field = field.map_err(|e| ApiError::bad_request(format!("Error leyendo multipart: {}", e)))?;
        let content_disposition = field.content_disposition();

        if let Some(cd) = content_disposition {
            if let Some(fname) = cd.get_filename() {
                original_filename = fname.to_string();
            }
        }

        let ext = Path::new(&original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");

        let temp_path = PathBuf::from(format!("{}/{}.{}", temp_dir, video_id, ext));

        let mut total_size: u64 = 0;
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| ApiError::internal(format!("Error creando archivo temporal: {}", e)))?;

        let mut field_stream = field;
        while let Some(chunk) = field_stream.next().await {
            let data = chunk.map_err(|e| ApiError::bad_request(format!("Error leyendo datos: {}", e)))?;
            total_size += data.len() as u64;

            if total_size > config.max_video_size_bytes() {
                let _ = fs::remove_file(&temp_path).await;
                return Err(ApiError::bad_request(format!(
                    "Video excede el tamaño máximo de {}MB",
                    config.max_video_size_mb
                )));
            }

            use tokio::io::AsyncWriteExt;
            file.write_all(&data)
                .await
                .map_err(|e| ApiError::internal(format!("Error escribiendo archivo: {}", e)))?;
        }

        Ok((temp_path, original_filename))
    } else {
        Err(ApiError::bad_request("No se encontró archivo de video"))
    }
}

/// Guarda una sola imagen de perfil o portada
pub async fn save_profile_image(
    mut payload: Multipart,
    config: &Config,
    image_type: &str, // "profile" o "cover"
) -> Result<UploadedImage, ApiError> {
    let upload_dir = format!("{}/profiles/{}", config.upload_dir, image_type);

    fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Error creando directorio: {}", e)))?;

    if let Some(field) = payload.next().await {
        let field = field.map_err(|e| ApiError::bad_request(format!("Error leyendo multipart: {}", e)))?;
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .and_then(|cd| cd.get_filename())
            .unwrap_or("photo.jpg")
            .to_string();

        let ext = Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}.{}", Uuid::new_v4(), ext);
        let filepath = format!("{}/{}", upload_dir, unique_name);

        let mut total_size: u64 = 0;
        let mut file = fs::File::create(&filepath)
            .await
            .map_err(|e| ApiError::internal(format!("Error creando archivo: {}", e)))?;

        let mut field_stream = field;
        while let Some(chunk) = field_stream.next().await {
            let data = chunk.map_err(|e| ApiError::bad_request(format!("Error leyendo datos: {}", e)))?;
            total_size += data.len() as u64;

            if total_size > config.max_image_size_bytes() {
                let _ = fs::remove_file(&filepath).await;
                return Err(ApiError::bad_request(format!(
                    "Imagen excede el tamaño máximo de {}MB",
                    config.max_image_size_mb
                )));
            }

            use tokio::io::AsyncWriteExt;
            file.write_all(&data)
                .await
                .map_err(|e| ApiError::internal(format!("Error escribiendo archivo: {}", e)))?;
        }

        Ok(UploadedImage {
            url: format!("/uploads/profiles/{}/{}", image_type, unique_name),
            filename: unique_name,
            size: total_size,
        })
    } else {
        Err(ApiError::bad_request("No se encontró archivo de imagen"))
    }
}
