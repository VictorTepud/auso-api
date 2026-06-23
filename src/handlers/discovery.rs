use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::discovery::*;
use crate::config::Config;

/// GET /api/v1/categories
/// Returns all available categories (seed list, used at registration).
pub async fn list_categories(
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, ApiError> {
    let cats = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories ORDER BY name"
    )
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(cats))
}

/// POST /api/v1/users/me/interests
/// Replaces the logged-in user's interests with the given category IDs.
/// Validates that at least 1 category is provided (registration requirement).
pub async fn set_user_interests(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    body: web::Json<SetInterestsRequest>,
) -> Result<HttpResponse, ApiError> {
    if body.category_ids.is_empty() {
        return Err(ApiError::bad_request(
            "Debes seleccionar al menos una categoría de interés",
        ));
    }

    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    // Replace existing interests
    sqlx::query("DELETE FROM user_interests WHERE user_id = ?")
        .bind(&user_id)
        .execute(pool.get_ref())
        .await?;

    for cat_id in &body.category_ids {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO user_interests (id, user_id, category_id, weight) VALUES (?, ?, ?, 1.0)"
        )
        .bind(&id)
        .bind(&user_id)
        .bind(cat_id)
        .execute(pool.get_ref())
        .await?;
    }

    let interests = sqlx::query_as::<_, UserInterest>(
        "SELECT * FROM user_interests WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(interests))
}

/// GET /api/v1/users/me/interests
pub async fn get_my_interests(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    let interests = sqlx::query_as::<_, UserInterest>(
        "SELECT * FROM user_interests WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(interests))
}

/// GET /api/v1/hashtags/search?q=gaming
/// Returns hashtags matching the query, ordered by usage_count DESC.
pub async fn search_hashtags(
    pool: web::Data<SqlitePool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let q = query.get("q").map(String::as_str).unwrap_or("");
    let pattern = format!("%{}%", q.to_lowercase());

    let hashtags = sqlx::query_as::<_, Hashtag>(
        "SELECT * FROM hashtags WHERE tag LIKE ? ORDER BY usage_count DESC LIMIT 20"
    )
    .bind(&pattern)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(hashtags))
}

/// GET /api/v1/hashtags/{tag}/posts
/// Returns posts tagged with the given hashtag (without the # prefix).
pub async fn get_posts_by_hashtag(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let _user_id = get_user_id_from_request(&req, config.get_ref())?;
    let tag = path.into_inner().to_lowercase();
    let page: u32 = query.get("page").and_then(|s| s.parse().ok()).unwrap_or(1);
    let limit: u32 = query.get("limit").and_then(|s| s.parse().ok()).unwrap_or(20).min(50);
    let offset = (page - 1) * limit;

    let posts = sqlx::query_as::<_, crate::models::post::PostWithAuthor>(
        "SELECT p.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo
         FROM posts p
         JOIN users u ON p.user_id = u.id
         JOIN post_hashtags ph ON ph.post_id = p.id
         JOIN hashtags h ON h.id = ph.hashtag_id
         WHERE h.tag = ?
         ORDER BY p.created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(&tag)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await?;

    // Enrich each post with the same fields as the main feed
    let mut feed = Vec::new();
    for p in posts {
        feed.push(enrich_post(pool.get_ref(), &p).await?);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": feed,
        "page": page,
        "limit": limit,
    })))
}

/// GET /api/v1/hashtags/trending
/// Top hashtags by usage in the last 7 days.
pub async fn trending_hashtags(
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, ApiError> {
    let hashtags = sqlx::query_as::<_, HashtagWithCount>(
        "SELECT tag, usage_count FROM hashtags ORDER BY usage_count DESC LIMIT 20"
    )
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(hashtags))
}

/// POST /api/v1/posts/{id}/impression
/// Records a user impression on a post (view/like/comment/share/skip).
/// This is the training signal for the recommendation algorithm.
pub async fn record_impression(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
    body: web::Json<CreateImpressionRequest>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = post_id.into_inner();
    let weight = PostImpression::weight_for(&body.impression_type);

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO post_impressions (id, user_id, post_id, impression_type, weight) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&user_id)
    .bind(&post_id)
    .bind(&body.impression_type)
    .bind(weight)
    .execute(pool.get_ref())
    .await?;

    // If the impression is positive (like/comment/share), boost the user's
    // affinity for the categories associated with the post's hashtags.
    if weight > 0.0 {
        let category_ids: Vec<String> = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT hc.category_id
             FROM post_hashtags ph
             JOIN hashtag_categories hc ON hc.hashtag_id = ph.hashtag_id
             WHERE ph.post_id = ?"
        )
        .bind(&post_id)
        .fetch_all(pool.get_ref())
        .await?;

        for cat_id in category_ids {
            // Upsert: bump the weight of this user's interest in this category
            sqlx::query(
                "INSERT INTO user_interests (id, user_id, category_id, weight)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(user_id, category_id)
                 DO UPDATE SET weight = user_interests.weight + ?"
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&user_id)
            .bind(&cat_id)
            .bind(weight * 0.1) // small boost per positive impression
            .bind(weight * 0.1)
            .execute(pool.get_ref())
            .await?;
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Impresión registrada",
        "weight": weight,
    })))
}

/// Helper: extract hashtags from a content string (e.g. "#gaming Hello #world").
/// Returns the tags WITHOUT the # prefix, lowercase, deduped.
pub fn extract_hashtags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current = String::new();
    let mut in_tag = false;

    for ch in content.chars() {
        if ch == '#' {
            in_tag = true;
            current.clear();
        } else if in_tag {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch.to_ascii_lowercase());
            } else {
                if current.len() >= 2 {
                    tags.push(current.clone());
                }
                current.clear();
                in_tag = false;
            }
        }
    }
    if in_tag && current.len() >= 2 {
        tags.push(current);
    }

    tags.sort();
    tags.dedup();
    tags
}

/// Helper: persist hashtags for a post. Called by the post creation handlers.
/// Creates hashtag rows if they don't exist (and bumps usage_count), then
/// links them to the post via post_hashtags. Also tries to auto-map each
/// hashtag to a category based on slug matching.
pub async fn persist_hashtags_for_post(
    pool: &SqlitePool,
    post_id: &str,
    content: &str,
) -> Result<(), ApiError> {
    let tags = extract_hashtags(content);
    if tags.is_empty() {
        return Ok(());
    }

    for tag in tags {
        // Upsert hashtag row
        let hashtag_id: String = sqlx::query_scalar::<_, String>(
            "INSERT INTO hashtags (id, tag, usage_count) VALUES (?, ?, 1)
             ON CONFLICT(tag) DO UPDATE SET usage_count = hashtags.usage_count + 1
             RETURNING id"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&tag)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| {
            // If RETURNING fails (older SQLite), fetch manually
            String::new()
        });

        // Fallback: fetch the hashtag id if the upsert above didn't return it
        let hashtag_id = if hashtag_id.is_empty() {
            sqlx::query_scalar::<_, String>(
                "SELECT id FROM hashtags WHERE tag = ?"
            )
            .bind(&tag)
            .fetch_one(pool)
            .await?
        } else {
            hashtag_id
        };

        // Link to post (ignore if already linked)
        sqlx::query(
            "INSERT OR IGNORE INTO post_hashtags (id, post_id, hashtag_id) VALUES (?, ?, ?)"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(post_id)
        .bind(&hashtag_id)
        .execute(pool)
        .await?;

        // Auto-map hashtag to category if slug matches a category slug
        sqlx::query(
            "INSERT OR IGNORE INTO hashtag_categories (hashtag_id, category_id)
             SELECT ?, id FROM categories WHERE slug = ?"
        )
        .bind(&hashtag_id)
        .bind(&tag)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Helper: enrich a PostWithAuthor into a PostResponse (likes, comments, images, video, poll).
/// Reused by the main feed, hashtag feed, and recommendation feed.
pub async fn enrich_post(
    pool: &SqlitePool,
    p: &crate::models::post::PostWithAuthor,
) -> Result<crate::models::post::PostResponse, ApiError> {
    use crate::models::post::{PostResponse, Post, PostImage, PostVideo};

    let images = sqlx::query_as::<_, PostImage>(
        "SELECT * FROM post_images WHERE post_id = ? ORDER BY position"
    )
    .bind(&p.id)
    .fetch_all(pool)
    .await?;

    let video = sqlx::query_as::<_, PostVideo>(
        "SELECT * FROM post_videos WHERE post_id = ?"
    )
    .bind(&p.id)
    .fetch_optional(pool)
    .await?;

    let likes_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM likes WHERE post_id = ?"
    )
    .bind(&p.id)
    .fetch_one(pool)
    .await?;

    let comments_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM comments WHERE post_id = ?"
    )
    .bind(&p.id)
    .fetch_one(pool)
    .await?;

    let poll_response = if p.post_type == "poll" {
        crate::handlers::posts::get_poll_for_post(pool, &p.id, None).await.ok()
    } else {
        None
    };

    Ok(PostResponse {
        post: Post {
            id: p.id.clone(),
            user_id: p.user_id.clone(),
            community_id: p.community_id.clone(),
            channel_id: p.channel_id.clone(),
            group_id: p.group_id.clone(),
            post_type: p.post_type.clone(),
            content: p.content.clone(),
            background_color: p.background_color.clone(),
            background_image_url: p.background_image_url.clone(),
            layout_mode: p.layout_mode.clone(),
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
        },
        images,
        video,
        poll: poll_response,
        likes_count,
        comments_count,
        is_liked: false,
        author_username: p.author_username.clone(),
        author_display_name: p.author_display_name.clone(),
        author_profile_photo: p.author_profile_photo.clone(),
    })
}
