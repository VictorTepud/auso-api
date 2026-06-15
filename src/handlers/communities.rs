use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::community::*;

/// POST /api/v1/communities
pub async fn create_community(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<CreateCommunityRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO communities (id, name, description, creator_id, is_private) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&community_id)
    .bind(&body.name)
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(&user_id)
    .bind(body.is_private.unwrap_or(false) as bool)
    .execute(pool.get_ref())
    .await?;

    // El creador se une automáticamente como admin
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO community_members (id, community_id, user_id, role) VALUES (?, ?, ?, 'admin')"
    )
    .bind(&member_id)
    .bind(&community_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    let community = sqlx::query_as::<_, Community>(
        "SELECT * FROM communities WHERE id = ?"
    )
    .bind(&community_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CommunityResponse {
        community,
        members_count: 1,
        is_member: true,
        user_role: Some("admin".to_string()),
    }))
}

/// GET /api/v1/communities
pub async fn list_communities(
    pool: web::Data<SqlitePool>,
    query: web::Query<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let search = query.get("q")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let communities = if search.is_empty() {
        sqlx::query_as::<_, CommunityWithCount>(
            "SELECT c.*, COUNT(cm.id) as members_count FROM communities c LEFT JOIN community_members cm ON c.id = cm.community_id GROUP BY c.id ORDER BY members_count DESC LIMIT 20"
        )
        .fetch_all(pool.get_ref())
        .await?
    } else {
        let pattern = format!("%{}%", search);
        sqlx::query_as::<_, CommunityWithCount>(
            "SELECT c.*, COUNT(cm.id) as members_count FROM communities c LEFT JOIN community_members cm ON c.id = cm.community_id WHERE c.name LIKE ? GROUP BY c.id ORDER BY members_count DESC LIMIT 20"
        )
        .bind(&pattern)
        .fetch_all(pool.get_ref())
        .await?
    };

    Ok(HttpResponse::Ok().json(communities))
}

/// GET /api/v1/communities/{id}
pub async fn get_community(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref()).ok();
    let community_id = community_id.into_inner();

    let community = sqlx::query_as::<_, Community>(
        "SELECT * FROM communities WHERE id = ?"
    )
    .bind(&community_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Comunidad no encontrada"))?;

    let members_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM community_members WHERE community_id = ?"
    )
    .bind(&community_id)
    .fetch_one(pool.get_ref())
    .await?;

    let (is_member, user_role) = if let Some(ref uid) = user_id {
        let member = sqlx::query_as::<_, CommunityMember>(
            "SELECT * FROM community_members WHERE community_id = ? AND user_id = ?"
        )
        .bind(&community_id)
        .bind(uid)
        .fetch_optional(pool.get_ref())
        .await?;
        (member.is_some(), member.map(|m| m.role))
    } else {
        (false, None)
    };

    Ok(HttpResponse::Ok().json(CommunityResponse {
        community,
        members_count,
        is_member,
        user_role,
    }))
}

/// PUT /api/v1/communities/{id}
pub async fn update_community(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
    body: web::Json<UpdateCommunityRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = community_id.into_inner();

    // Verificar permisos (admin o moderador)
    let _member = sqlx::query_as::<_, CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&community_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("No tienes permisos para editar esta comunidad"))?;

    let community = sqlx::query_as::<_, Community>(
        "SELECT * FROM communities WHERE id = ?"
    )
    .bind(&community_id)
    .fetch_one(pool.get_ref())
    .await?;

    let name = body.name.as_deref().unwrap_or(&community.name);
    let description = body.description.as_deref().unwrap_or(&community.description);
    let is_private = body.is_private.unwrap_or(community.is_private);

    sqlx::query(
        "UPDATE communities SET name=?, description=?, is_private=?, updated_at=datetime('now') WHERE id=?"
    )
    .bind(name)
    .bind(description)
    .bind(is_private)
    .bind(&community_id)
    .execute(pool.get_ref())
    .await?;

    let updated = sqlx::query_as::<_, Community>(
        "SELECT * FROM communities WHERE id = ?"
    )
    .bind(&community_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(updated))
}

/// POST /api/v1/communities/{id}/join
pub async fn join_community(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = community_id.into_inner();

    // Verificar que existe
    let _ = sqlx::query_as::<_, Community>(
        "SELECT * FROM communities WHERE id = ?"
    )
    .bind(&community_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Comunidad no encontrada"))?;

    // Verificar si ya es miembro
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM community_members WHERE community_id = ? AND user_id = ?"
    )
    .bind(&community_id)
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await?;

    if existing > 0 {
        return Err(ApiError::conflict("Ya eres miembro de esta comunidad"));
    }

    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO community_members (id, community_id, user_id, role) VALUES (?, ?, ?, 'member')"
    )
    .bind(&member_id)
    .bind(&community_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Te has unido a la comunidad correctamente"
    })))
}

/// POST /api/v1/communities/{id}/leave
pub async fn leave_community(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = community_id.into_inner();

    sqlx::query(
        "DELETE FROM community_members WHERE community_id = ? AND user_id = ?"
    )
    .bind(&community_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Has salido de la comunidad"
    })))
}

/// GET /api/v1/communities/{id}/members
pub async fn get_community_members(
    pool: web::Data<SqlitePool>,
    community_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let community_id = community_id.into_inner();

    let members = sqlx::query_as::<_, CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? ORDER BY joined_at DESC"
    )
    .bind(&community_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(members))
}

/// POST /api/v1/communities/{id}/cover-photo
pub async fn upload_community_cover(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    community_id: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let community_id = community_id.into_inner();

    // Verificar permisos
    let _ = sqlx::query_as::<_, CommunityMember>(
        "SELECT * FROM community_members WHERE community_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&community_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("Sin permisos"))?;

    let uploaded = crate::services::media::save_image(payload, config.get_ref(), &format!("communities/{}", community_id)).await?;

    if let Some(img) = uploaded.first() {
        sqlx::query("UPDATE communities SET cover_photo_url=?, updated_at=datetime('now') WHERE id=?")
            .bind(&img.url)
            .bind(&community_id)
            .execute(pool.get_ref())
            .await?;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Foto de portada actualizada"
    })))
}
