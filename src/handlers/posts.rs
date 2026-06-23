use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::post::*;
use crate::models::poll::{PollOptionResponse, PollResponse};
use crate::services::{media, video};

/// POST /api/v1/posts/text
pub async fn create_text_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<CreateTextPostRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO posts (id, user_id, community_id, channel_id, group_id, post_type, content, background_color, background_image_url) VALUES (?, ?, ?, ?, ?, 'text', ?, ?, ?)"
    )
    .bind(&post_id)
    .bind(&user_id)
    .bind(&body.community_id)
    .bind(&body.channel_id)
    .bind(&body.group_id)
    .bind(&body.content)
    .bind(&body.background_color)
    .bind(&body.background_image_url)
    .execute(pool.get_ref())
    .await?;

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_one(pool.get_ref())
        .await?;

    // Extract & persist hashtags from the post content
    let _ = crate::handlers::discovery::persist_hashtags_for_post(
        pool.get_ref(), &post_id, &body.content
    ).await;

    Ok(HttpResponse::Created().json(post))
}

/// POST /api/v1/posts/image
pub async fn create_image_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = Uuid::new_v4().to_string();

    // Primero crear el post
    sqlx::query(
        "INSERT INTO posts (id, user_id, post_type, content) VALUES (?, ?, 'image', '')"
    )
    .bind(&post_id)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    // Guardar las imágenes
    let images = media::save_image(payload, config.get_ref(), &post_id).await?;

    for (idx, img) in images.iter().enumerate() {
        let img_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO post_images (id, post_id, image_url, position) VALUES (?, ?, ?, ?)"
        )
        .bind(&img_id)
        .bind(&post_id)
        .bind(&img.url)
        .bind(idx as i32)
        .execute(pool.get_ref())
        .await?;
    }

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_one(pool.get_ref())
        .await?;

    let post_images = sqlx::query_as::<_, PostImage>(
        "SELECT * FROM post_images WHERE post_id = ? ORDER BY position"
    )
    .bind(&post_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "post": post,
        "images": post_images,
    })))
}

/// POST /api/v1/posts/image-pack
pub async fn create_image_pack_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<CreateImagePackPostRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = Uuid::new_v4().to_string();

    let layout = body.layout_mode.as_deref().unwrap_or("carousel");
    if layout != "carousel" && layout != "grid" {
        return Err(ApiError::bad_request("layout_mode debe ser 'carousel' o 'grid'"));
    }

    let content = body.content.as_deref().unwrap_or("");

    sqlx::query(
        "INSERT INTO posts (id, user_id, community_id, channel_id, group_id, post_type, content, layout_mode) VALUES (?, ?, ?, ?, ?, 'image_pack', ?, ?)"
    )
    .bind(&post_id)
    .bind(&user_id)
    .bind(&body.community_id)
    .bind(&body.channel_id)
    .bind(&body.group_id)
    .bind(content)
    .bind(layout)
    .execute(pool.get_ref())
    .await?;

    // Las imágenes se suben en un endpoint separado después de crear el post
    // POST /api/v1/posts/{id}/images

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_one(pool.get_ref())
        .await?;

    // Extract & persist hashtags from the post content (if any)
    let _ = crate::handlers::discovery::persist_hashtags_for_post(
        pool.get_ref(), &post_id, content
    ).await;

    Ok(HttpResponse::Created().json(post))
}

/// POST /api/v1/posts/{id}/images - Agregar imágenes a un post image_pack
pub async fn add_images_to_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = post_id.into_inner();

    // Verificar que el post pertenece al usuario y es de tipo image_pack
    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ? AND user_id = ?")
        .bind(&post_id)
        .bind(&user_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("Post no encontrado"))?;

    if post.post_type != "image_pack" && post.post_type != "image" {
        return Err(ApiError::bad_request("Este post no soporta imágenes adicionales"));
    }

    // Verificar límite de imágenes
    let current_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM post_images WHERE post_id = ?"
    )
    .bind(&post_id)
    .fetch_one(pool.get_ref())
    .await?;

    if current_count >= config.max_images_per_pack as i64 {
        return Err(ApiError::bad_request(format!(
            "Máximo {} imágenes permitidas por post",
            config.max_images_per_pack
        )));
    }

    let images = media::save_image(payload, config.get_ref(), &post_id).await?;
    let mut saved_images = Vec::new();

    for (idx, img) in images.iter().enumerate() {
        let position = current_count as i32 + idx as i32;
        if position >= config.max_images_per_pack as i32 {
            break;
        }
        let img_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO post_images (id, post_id, image_url, position) VALUES (?, ?, ?, ?)"
        )
        .bind(&img_id)
        .bind(&post_id)
        .bind(&img.url)
        .bind(position)
        .execute(pool.get_ref())
        .await?;

        let saved = sqlx::query_as::<_, PostImage>(
            "SELECT * FROM post_images WHERE id = ?"
        )
        .bind(&img_id)
        .fetch_one(pool.get_ref())
        .await?;

        saved_images.push(saved);
    }

    Ok(HttpResponse::Created().json(saved_images))
}

/// POST /api/v1/posts/video
pub async fn create_video_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = Uuid::new_v4().to_string();
    let video_id = Uuid::new_v4().to_string();

    // Guardar video temporalmente + leer campos de texto opcionales (title/description/content)
    let saved = media::save_video_temp(payload, config.get_ref()).await?;

    // Crear el post primero — el contenido del post ahora es el `content` enviado por el cliente
    // (si no se envió, queda vacío para preservar comportamiento anterior)
    sqlx::query(
        "INSERT INTO posts (id, user_id, post_type, content) VALUES (?, ?, 'video', ?)"
    )
    .bind(&post_id)
    .bind(&user_id)
    .bind(&saved.content)
    .execute(pool.get_ref())
    .await?;

    // Procesar video en background (en producción sería con una cola de trabajos)
    let config_clone = config.clone();
    let pool_clone = pool.clone();
    let post_id_clone = post_id.clone();
    let video_id_clone = video_id.clone();
    let original_filename_clone = saved.original_filename.clone();
    let title_clone = saved.title.clone();
    let description_clone = saved.description.clone();
    let temp_path = saved.temp_path.clone();

    // Procesamiento asíncrono del video
    tokio::spawn(async move {
        match video::process_video(temp_path.to_str().unwrap_or(""), &video_id_clone, &config_clone).await {
            Ok(result) => {
                let video_record_id = Uuid::new_v4().to_string();
                let query_result = sqlx::query(
                    "INSERT INTO post_videos (id, post_id, original_filename, hls_master_playlist_url, duration, width, height, thumbnail_url, title, description) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&video_record_id)
                .bind(&post_id_clone)
                .bind(&original_filename_clone)
                .bind(&result.hls_master_playlist_url)
                .bind(result.duration)
                .bind(result.width)
                .bind(result.height)
                .bind(&result.thumbnail_url)
                .bind(&title_clone)
                .bind(&description_clone)
                .execute(pool_clone.get_ref())
                .await;

                if let Err(e) = query_result {
                    tracing::error!("Error guardando registro de video: {}", e);
                } else {
                    tracing::info!("Video procesado exitosamente: {}", video_id_clone);
                }

                // Limpiar archivo temporal
                let _ = tokio::fs::remove_file(&temp_path).await;
            }
            Err(e) => {
                tracing::error!("Error procesando video {}: {}", video_id_clone, e);
                let _ = tokio::fs::remove_file(&temp_path).await;
            }
        }
    });

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_one(pool.get_ref())
        .await?;

    // Extract & persist hashtags from the post content (saved.content)
    let _ = crate::handlers::discovery::persist_hashtags_for_post(
        pool.get_ref(), &post_id, &saved.content
    ).await;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "post": post,
        "message": "Video recibido. Se está procesando en segundo plano.",
        "video_id": video_id,
    })))
}

/// GET /api/v1/posts/{id}
pub async fn get_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref()).ok();
    let post_id = post_id.into_inner();

    let post = sqlx::query_as::<_, PostWithAuthor>(
        "SELECT p.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(&post_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Post no encontrado"))?;

    let images = sqlx::query_as::<_, PostImage>(
        "SELECT * FROM post_images WHERE post_id = ? ORDER BY position"
    )
    .bind(&post_id)
    .fetch_all(pool.get_ref())
    .await?;

    let video = sqlx::query_as::<_, PostVideo>(
        "SELECT * FROM post_videos WHERE post_id = ?"
    )
    .bind(&post_id)
    .fetch_optional(pool.get_ref())
    .await?;

    let likes_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM likes WHERE post_id = ?"
    )
    .bind(&post_id)
    .fetch_one(pool.get_ref())
    .await?;

    let comments_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM comments WHERE post_id = ?"
    )
    .bind(&post_id)
    .fetch_one(pool.get_ref())
    .await?;

    let is_liked = if let Some(ref uid) = user_id {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM likes WHERE post_id = ? AND user_id = ?"
        )
        .bind(&post_id)
        .bind(uid)
        .fetch_one(pool.get_ref())
        .await?
        > 0
    } else {
        false
    };

    // Obtener poll si existe
    let poll_response = if post.post_type == "poll" {
        get_poll_for_post(pool.get_ref(), &post_id, user_id.as_deref()).await.ok()
    } else {
        None
    };

    let response = PostResponse {
        post: Post {
            id: post.id,
            user_id: post.user_id,
            community_id: post.community_id,
            channel_id: post.channel_id,
            group_id: post.group_id,
            post_type: post.post_type,
            content: post.content,
            background_color: post.background_color,
            background_image_url: post.background_image_url,
            layout_mode: post.layout_mode,
            created_at: post.created_at,
            updated_at: post.updated_at,
        },
        images,
        video,
        poll: poll_response,
        likes_count,
        comments_count,
        is_liked,
        author_username: post.author_username,
        author_display_name: post.author_display_name,
        author_profile_photo: post.author_profile_photo,
        hashtags: {
            sqlx::query_scalar::<_, String>(
                "SELECT h.tag FROM post_hashtags ph JOIN hashtags h ON h.id = ph.hashtag_id
                 WHERE ph.post_id = ? ORDER BY h.tag"
            )
            .bind(&post.id)
            .fetch_all(pool.get_ref())
            .await.unwrap_or_default()
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/posts/feed
pub async fn get_feed(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    query: web::Query<FeedQuery>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20).min(50);
    let offset = (page - 1) * limit;

    // Build WHERE clause dynamically based on filters
    let mut where_clauses = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref community_id) = query.community_id {
        where_clauses.push("p.community_id = ?".to_string());
        bind_values.push(community_id.clone());
    }
    if let Some(ref group_id) = query.group_id {
        where_clauses.push("p.group_id = ?".to_string());
        bind_values.push(group_id.clone());
    }
    if let Some(ref target_user_id) = query.user_id {
        where_clauses.push("p.user_id = ?".to_string());
        bind_values.push(target_user_id.clone());
    }
    if let Some(ref post_type) = query.post_type {
        where_clauses.push("p.post_type = ?".to_string());
        bind_values.push(post_type.clone());
    }

    let posts = if where_clauses.is_empty() {
        // Feed principal: posts de usuarios seguidos + propios
        sqlx::query_as::<_, PostWithAuthor>(
            "SELECT p.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo FROM posts p JOIN users u ON p.user_id = u.id WHERE p.user_id = ? OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = ?) ORDER BY p.created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(&user_id)
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.get_ref())
        .await?
    } else {
        // Filtered feed with dynamic WHERE
        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "SELECT p.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo FROM posts p JOIN users u ON p.user_id = u.id WHERE {} ORDER BY p.created_at DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let mut query = sqlx::query_as::<_, PostWithAuthor>(&sql);
        for val in &bind_values {
            query = query.bind(val);
        }
        query = query.bind(limit).bind(offset);
        query.fetch_all(pool.get_ref()).await?
    };

    // Enriquecer cada post con likes, comments, imágenes, etc.
    let mut feed = Vec::new();
    for p in posts {
        let images = sqlx::query_as::<_, PostImage>(
            "SELECT * FROM post_images WHERE post_id = ? ORDER BY position"
        )
        .bind(&p.id)
        .fetch_all(pool.get_ref())
        .await?;

        let video = sqlx::query_as::<_, PostVideo>(
            "SELECT * FROM post_videos WHERE post_id = ?"
        )
        .bind(&p.id)
        .fetch_optional(pool.get_ref())
        .await?;

        let likes_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM likes WHERE post_id = ?"
        )
        .bind(&p.id)
        .fetch_one(pool.get_ref())
        .await?;

        let comments_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM comments WHERE post_id = ?"
        )
        .bind(&p.id)
        .fetch_one(pool.get_ref())
        .await?;

        let is_liked = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM likes WHERE post_id = ? AND user_id = ?"
        )
        .bind(&p.id)
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?
        > 0;

        let poll_response = if p.post_type == "poll" {
            get_poll_for_post(pool.get_ref(), &p.id, Some(&user_id)).await.ok()
        } else {
            None
        };

        feed.push(PostResponse {
            post: Post {
                id: p.id,
                user_id: p.user_id,
                community_id: p.community_id,
                channel_id: p.channel_id,
                group_id: p.group_id,
                post_type: p.post_type,
                content: p.content,
                background_color: p.background_color,
                background_image_url: p.background_image_url,
                layout_mode: p.layout_mode,
                created_at: p.created_at,
                updated_at: p.updated_at,
            },
            images,
            video,
            poll: poll_response,
            likes_count,
            comments_count,
            is_liked,
            author_username: p.author_username,
            author_display_name: p.author_display_name,
            author_profile_photo: p.author_profile_photo,
            hashtags: {
                sqlx::query_scalar::<_, String>(
                    "SELECT h.tag FROM post_hashtags ph JOIN hashtags h ON h.id = ph.hashtag_id
                     WHERE ph.post_id = ? ORDER BY h.tag"
                )
                .bind(&p.id)
                .fetch_all(pool.get_ref())
                .await.unwrap_or_default()
            },
        });
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": feed,
        "page": page,
        "limit": limit,
    })))
}

/// DELETE /api/v1/posts/{id}
pub async fn delete_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = post_id.into_inner();

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("Post no encontrado"))?;

    if post.user_id != user_id {
        return Err(ApiError::forbidden("No tienes permiso para eliminar este post"));
    }

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(&post_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Post eliminado correctamente"
    })))
}

/// POST /api/v1/posts/{id}/like
pub async fn toggle_like(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = post_id.into_inner();

    // Verificar que el post existe
    let _ = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("Post no encontrado"))?;

    // Verificar si ya tiene like
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM likes WHERE post_id = ? AND user_id = ?"
    )
    .bind(&post_id)
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let liked: bool;
    if existing > 0 {
        sqlx::query("DELETE FROM likes WHERE post_id = ? AND user_id = ?")
            .bind(&post_id)
            .bind(&user_id)
            .execute(pool.get_ref())
            .await?;
        liked = false;
    } else {
        let like_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO likes (id, post_id, user_id) VALUES (?, ?, ?)"
        )
        .bind(&like_id)
        .bind(&post_id)
        .bind(&user_id)
        .execute(pool.get_ref())
        .await?;
        liked = true;
    }

    let likes_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM likes WHERE post_id = ?"
    )
    .bind(&post_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "liked": liked,
        "likes_count": likes_count,
    })))
}

/// Helper: obtener poll de un post
pub async fn get_poll_for_post(
    pool: &SqlitePool,
    post_id: &str,
    user_id: Option<&str>,
) -> Result<PollResponse, ApiError> {
    let poll = sqlx::query_as::<_, crate::models::poll::Poll>(
        "SELECT * FROM polls WHERE post_id = ?"
    )
    .bind(post_id)
    .fetch_one(pool)
    .await?;

    let options = sqlx::query_as::<_, crate::models::poll::PollOption>(
        "SELECT * FROM poll_options WHERE poll_id = ? ORDER BY position"
    )
    .bind(&poll.id)
    .fetch_all(pool)
    .await?;

    let mut option_responses = Vec::new();
    let mut total_votes: i64 = 0;

    for opt in &options {
        let votes_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ?"
        )
        .bind(&opt.id)
        .fetch_one(pool)
        .await?;

        total_votes += votes_count;

        let has_voted = if let Some(uid) = user_id {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ? AND user_id = ?"
            )
            .bind(&opt.id)
            .bind(uid)
            .fetch_one(pool)
            .await?
            > 0
        } else {
            false
        };

        option_responses.push(PollOptionResponse {
            option: opt.clone(),
            votes_count,
            has_voted,
        });
    }

    let user_has_voted = if let Some(uid) = user_id {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes pv JOIN poll_options po ON pv.poll_option_id = po.id WHERE po.poll_id = ? AND pv.user_id = ?"
        )
        .bind(&poll.id)
        .bind(uid)
        .fetch_one(pool)
        .await?
        > 0
    } else {
        false
    };

    Ok(PollResponse {
        poll,
        options: option_responses,
        total_votes,
        user_has_voted,
    })
}
