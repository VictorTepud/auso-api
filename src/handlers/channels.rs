use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::channel::*;

/// POST /api/v1/communities/{community_id}/channels
pub async fn create_channel(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
    body: web::Json<CreateChannelRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = community_id.into_inner();

    // Verificar que el usuario es admin/mod de la comunidad
    let _ = sqlx::query_as::<_, crate::models::community::CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&community_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("No tienes permisos para crear canales en esta comunidad"))?;

    let channel_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO channels (id, community_id, name, description) VALUES (?, ?, ?, ?)"
    )
    .bind(&channel_id)
    .bind(&community_id)
    .bind(&body.name)
    .bind(body.description.as_deref().unwrap_or(""))
    .execute(pool.get_ref())
    .await?;

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(channel))
}

/// GET /api/v1/communities/{community_id}/channels
pub async fn list_channels(
    pool: web::Data<SqlitePool>,
    community_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let community_id = community_id.into_inner();

    let channels = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE community_id = ? ORDER BY name"
    )
    .bind(&community_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(channels))
}

/// GET /api/v1/channels/{id}
pub async fn get_channel(
    pool: web::Data<SqlitePool>,
    channel_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let channel_id = channel_id.into_inner();

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Canal no encontrado"))?;

    let posts_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM posts WHERE channel_id = ?"
    )
    .bind(&channel_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(ChannelResponse {
        channel,
        posts_count,
    }))
}

/// PUT /api/v1/channels/{id}
pub async fn update_channel(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    channel_id: web::Path<String>,
    body: web::Json<UpdateChannelRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let channel_id = channel_id.into_inner();

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Canal no encontrado"))?;

    // Verificar permisos en la comunidad
    let _ = sqlx::query_as::<_, crate::models::community::CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&channel.community_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("Sin permisos para editar este canal"))?;

    let name = body.name.as_deref().unwrap_or(&channel.name);
    let description = body.description.as_deref().unwrap_or(&channel.description);

    sqlx::query(
        "UPDATE channels SET name=?, description=?, updated_at=datetime('now') WHERE id=?"
    )
    .bind(name)
    .bind(description)
    .bind(&channel_id)
    .execute(pool.get_ref())
    .await?;

    let updated = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(updated))
}

/// DELETE /api/v1/channels/{id}
pub async fn delete_channel(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    channel_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let channel_id = channel_id.into_inner();

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Canal no encontrado"))?;

    let _ = sqlx::query_as::<_, crate::models::community::CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? AND user_id = ? AND role = 'admin'"
    )
    .bind(&channel.community_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("Solo el admin puede eliminar canales"))?;

    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(&channel_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Canal eliminado correctamente"
    })))
}
