use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::group::*;

/// POST /api/v1/groups
pub async fn create_group(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<CreateGroupRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let group_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO groups (id, name, description, creator_id, is_private) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&group_id)
    .bind(&body.name)
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(&user_id)
    .bind(body.is_private.unwrap_or(false) as bool)
    .execute(pool.get_ref())
    .await?;

    // El creador se une automáticamente como admin
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role) VALUES (?, ?, ?, 'admin')"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(GroupResponse {
        group,
        members_count: 1,
        is_member: true,
        user_role: Some("admin".to_string()),
    }))
}

/// GET /api/v1/groups
pub async fn list_groups(
    pool: web::Data<SqlitePool>,
    query: web::Query<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let search = query.get("q")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let groups = if search.is_empty() {
        sqlx::query_as::<_, GroupWithCount>(
            "SELECT g.*, COUNT(gm.id) as members_count FROM groups g LEFT JOIN group_members gm ON g.id = gm.group_id GROUP BY g.id ORDER BY members_count DESC LIMIT 20"
        )
        .fetch_all(pool.get_ref())
        .await?
    } else {
        let pattern = format!("%{}%", search);
        sqlx::query_as::<_, GroupWithCount>(
            "SELECT g.*, COUNT(gm.id) as members_count FROM groups g LEFT JOIN group_members gm ON g.id = gm.group_id WHERE g.name LIKE ? GROUP BY g.id ORDER BY members_count DESC LIMIT 20"
        )
        .bind(&pattern)
        .fetch_all(pool.get_ref())
        .await?
    };

    Ok(HttpResponse::Ok().json(groups))
}

/// GET /api/v1/groups/{id}
pub async fn get_group(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    group_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref()).ok();
    let group_id = group_id.into_inner();

    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Grupo no encontrado"))?;

    let members_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM group_members WHERE group_id = ?"
    )
    .bind(&group_id)
    .fetch_one(pool.get_ref())
    .await?;

    let (is_member, user_role) = if let Some(ref uid) = user_id {
        let member = sqlx::query_as::<_, GroupMember>(
            "SELECT * FROM group_members WHERE group_id = ? AND user_id = ?"
        )
        .bind(&group_id)
        .bind(uid)
        .fetch_optional(pool.get_ref())
        .await?;
        (member.is_some(), member.map(|m| m.role))
    } else {
        (false, None)
    };

    Ok(HttpResponse::Ok().json(GroupResponse {
        group,
        members_count,
        is_member,
        user_role,
    }))
}

/// PUT /api/v1/groups/{id}
pub async fn update_group(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    group_id: web::Path<String>,
    body: web::Json<UpdateGroupRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let group_id = group_id.into_inner();

    let _member = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&group_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("No tienes permisos para editar este grupo"))?;

    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_one(pool.get_ref())
    .await?;

    let name = body.name.as_deref().unwrap_or(&group.name);
    let description = body.description.as_deref().unwrap_or(&group.description);
    let is_private = body.is_private.unwrap_or(group.is_private);

    sqlx::query(
        "UPDATE groups SET name=?, description=?, is_private=?, updated_at=datetime('now') WHERE id=?"
    )
    .bind(name)
    .bind(description)
    .bind(is_private)
    .bind(&group_id)
    .execute(pool.get_ref())
    .await?;

    let updated = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(updated))
}

/// POST /api/v1/groups/{id}/join
pub async fn join_group(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    group_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let group_id = group_id.into_inner();

    let _ = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Grupo no encontrado"))?;

    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await?;

    if existing > 0 {
        return Err(ApiError::conflict("Ya eres miembro de este grupo"));
    }

    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role) VALUES (?, ?, ?, 'member')"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Te has unido al grupo correctamente"
    })))
}

/// POST /api/v1/groups/{id}/leave
pub async fn leave_group(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    group_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let group_id = group_id.into_inner();

    sqlx::query(
        "DELETE FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Has salido del grupo"
    })))
}

/// GET /api/v1/groups/{id}/members
pub async fn get_group_members(
    pool: web::Data<SqlitePool>,
    group_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_id = group_id.into_inner();

    let members = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? ORDER BY joined_at DESC"
    )
    .bind(&group_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(members))
}

/// POST /api/v1/groups/{id}/cover-photo
pub async fn upload_group_cover(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    group_id: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let group_id = group_id.into_inner();

    let _ = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ? AND role IN ('admin', 'moderator')"
    )
    .bind(&group_id)
    .bind(&user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::forbidden("Sin permisos"))?;

    let uploaded = crate::services::media::save_image(payload, config.get_ref(), &format!("groups/{}", group_id)).await?;

    if let Some(img) = uploaded.first() {
        sqlx::query("UPDATE groups SET cover_photo_url=?, updated_at=datetime('now') WHERE id=?")
            .bind(&img.url)
            .bind(&group_id)
            .execute(pool.get_ref())
            .await?;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Foto de portada actualizada"
    })))
}
