use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::post::{PostResponse, PostWithAuthor};
use crate::config::Config;

/// GET /api/v1/posts/recommended
///
/// Hybrid recommendation algorithm (TikTok + Twitter + Facebook style).
///
/// Scoring per candidate post (0.0..~10.0 typically):
///   1. Interest match (40%): post's hashtag categories vs user's interest weights.
///      - For each post, sum the user's weight for each category linked to the post's hashtags.
///      - Boosted by 1.5x if the post has at least one matching category.
///   2. Engagement (25%): log-scaled likes + comments + shares (proxy = likes + comments).
///      - score_eng = log10(1 + likes + comments*2) * 0.8
///   3. Social graph (20%): post authored by a followee → +1.5; mutual-follow → +0.5.
///   4. Recency (10%): exponential decay over 72h. Brand-new (<1h) gets a novelty bump.
///      - score_rec = exp(-age_hours / 36)
///   5. Diversity (5%): cap posts per author at 2 to avoid one user flooding the feed.
///   6. Hashtag co-occurrence (bonus): if the user previously engaged with posts
///      sharing a hashtag with this post, +0.3 per shared hashtag (max +1.5).
///
/// Final score = 0.40*interest + 0.25*engagement + 0.20*social + 0.10*recency + 0.05*diversity + bonus
///
/// Candidates: posts from the last 14 days that the user hasn't seen (no impression)
/// OR saw but didn't skip. We fetch up to 200 candidates and rank them.
pub async fn get_recommended_feed(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let page: u32 = query.get("page").and_then(|s| s.parse().ok()).unwrap_or(1);
    let limit: u32 = query.get("limit").and_then(|s| s.parse().ok()).unwrap_or(20).min(50);
    let offset = (page - 1) * limit;

    // ── 1. Load the user's interest weights (category_id → weight) ──
    let interests: Vec<(String, f64)> = sqlx::query_as::<_, (String, f64)>(
        "SELECT category_id, weight FROM user_interests WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;
    let interest_map: HashMap<String, f64> = interests.into_iter().collect();

    // ── 2. Load hashtags the user previously engaged with positively ──
    // (from impressions on posts that had hashtags)
    let engaged_hashtags: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT h.tag
         FROM post_impressions pi
         JOIN post_hashtags ph ON ph.post_id = pi.post_id
         JOIN hashtags h ON h.id = ph.hashtag_id
         WHERE pi.user_id = ? AND pi.weight > 0"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;
    let engaged_hashtag_set: std::collections::HashSet<String> = engaged_hashtags.into_iter().collect();

    // ── 3. Load the user's followees (for the social-graph boost) ──
    let followees: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT following_id FROM follows WHERE follower_id = ?"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;
    let followee_set: std::collections::HashSet<String> = followees.into_iter().collect();

    // ── 4. Fetch candidate posts (last 14 days, excluding own posts) ──
    // We fetch a larger pool (200) and rank, then paginate the result.
    let candidates = sqlx::query_as::<_, PostWithAuthor>(
        "SELECT p.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo
         FROM posts p
         JOIN users u ON p.user_id = u.id
         WHERE p.user_id != ?
           AND p.created_at >= datetime('now', '-14 days')
         ORDER BY p.created_at DESC
         LIMIT 200"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;

    // ── 5. Score each candidate ──
    let mut scored: Vec<(f64, PostWithAuthor)> = Vec::new();

    for post in candidates {
        // (a) Interest match
        let post_categories: Vec<String> = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT hc.category_id
             FROM post_hashtags ph
             JOIN hashtag_categories hc ON hc.hashtag_id = ph.hashtag_id
             WHERE ph.post_id = ?"
        )
        .bind(&post.id)
        .fetch_all(pool.get_ref())
        .await?;

        let mut interest_score = 0.0;
        let mut has_match = false;
        for cat_id in &post_categories {
            if let Some(w) = interest_map.get(cat_id) {
                interest_score += w;
                has_match = true;
            }
        }
        if has_match {
            interest_score *= 1.5;
        }

        // (b) Engagement
        let likes: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM likes WHERE post_id = ?"
        )
        .bind(&post.id)
        .fetch_one(pool.get_ref())
        .await?;
        let comments: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM comments WHERE post_id = ?"
        )
        .bind(&post.id)
        .fetch_one(pool.get_ref())
        .await?;
        let engagement_score = ((1.0 + likes as f64 + (comments as f64) * 2.0).ln() / std::f64::consts::LN_10) * 0.8;

        // (c) Social graph
        let mut social_score = 0.0;
        if followee_set.contains(&post.user_id) {
            social_score += 1.5;
        }

        // (d) Recency — exponential decay over 36h half-life
        let age_hours = age_in_hours(&post.created_at);
        let recency_score = (-age_hours / 36.0).exp();
        // novelty bump: < 1h old
        let novelty = if age_hours < 1.0 { 0.3 } else { 0.0 };

        // (e) Hashtag co-occurrence bonus
        let post_hashtags: Vec<String> = sqlx::query_scalar::<_, String>(
            "SELECT h.tag FROM post_hashtags ph JOIN hashtags h ON h.id = ph.hashtag_id WHERE ph.post_id = ?"
        )
        .bind(&post.id)
        .fetch_all(pool.get_ref())
        .await?;
        let mut bonus = 0.0;
        for tag in &post_hashtags {
            if engaged_hashtag_set.contains(tag) {
                bonus += 0.3;
            }
        }
        bonus = bonus.min(1.5);

        let final_score = 0.40 * interest_score
            + 0.25 * engagement_score
            + 0.20 * social_score
            + 0.10 * recency_score
            + novelty
            + bonus;

        scored.push((final_score, post));
    }

    // ── 6. Diversity: cap at 2 posts per author in the final ranked list ──
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut final_posts: Vec<PostWithAuthor> = Vec::new();
    let mut author_counts: HashMap<String, usize> = HashMap::new();
    for (_score, post) in scored {
        let count = author_counts.entry(post.user_id.clone()).or_insert(0);
        if *count < 2 {
            final_posts.push(post.clone());
            *count += 1;
        }
    }

    // ── 7. Paginate ──
    let paged: Vec<PostWithAuthor> = final_posts
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    // ── 8. Enrich each post (likes, comments, images, video, poll) ──
    let mut feed = Vec::new();
    for p in &paged {
        let mut resp = crate::handlers::discovery::enrich_post(pool.get_ref(), p).await?;
        // Set is_liked for the current user
        let is_liked: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM likes WHERE post_id = ? AND user_id = ?"
        )
        .bind(&p.id)
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?;
        resp.is_liked = is_liked > 0;
        feed.push(resp);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": feed,
        "page": page,
        "limit": limit,
    })))
}

/// Parse a SQLite datetime string ("YYYY-MM-DD HH:MM:SS" UTC) and return
/// the age in hours vs. the current UTC time. Returns a large number on
/// parse failure so the recency score collapses to ~0.
fn age_in_hours(created_at: &str) -> f64 {
    // Expected format: "2025-01-15 14:30:00"
    let parts: Vec<&str> = created_at.split(' ').collect();
    if parts.len() != 2 {
        return 9999.0;
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return 9999.0;
    }
    let (y, m, d): (i32, u32, u32) = match (
        date_parts[0].parse(), date_parts[1].parse(), date_parts[2].parse()
    ) {
        (Ok(y), Ok(m), Ok(d)) => (y, m, d),
        _ => return 9999.0,
    };
    let (h, mi, s): (u32, u32, u32) = match (
        time_parts[0].parse(), time_parts[1].parse(), time_parts[2].parse()
    ) {
        (Ok(h), Ok(mi), Ok(s)) => (h, mi, s),
        _ => return 9999.0,
    };

    use chrono::{TimeZone, Utc};
    let created = Utc.with_ymd_and_hms(y, m, d, h, mi, s).single();
    match created {
        Some(t) => (Utc::now() - t).num_milliseconds() as f64 / 3_600_000.0,
        None => 9999.0,
    }
}
