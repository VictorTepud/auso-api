use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ── Categories ─────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub icon: String,
    pub created_at: String,
}

// ── Hashtags ───────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Hashtag {
    pub id: String,
    pub tag: String,
    pub usage_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostHashtag {
    pub id: String,
    pub post_id: String,
    pub hashtag_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashtagWithCount {
    pub tag: String,
    pub usage_count: i64,
}

// ── User interests ─────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserInterest {
    pub id: String,
    pub user_id: String,
    pub category_id: String,
    pub weight: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SetInterestsRequest {
    pub category_ids: Vec<String>,
}

// ── Post impressions ───────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostImpression {
    pub id: String,
    pub user_id: String,
    pub post_id: String,
    pub impression_type: String,
    pub weight: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateImpressionRequest {
    pub impression_type: String, // 'view' | 'like' | 'comment' | 'share' | 'skip'
}

impl PostImpression {
    /// Weight lookup table — drives the recommender's affinity score.
    /// Likes and shares are the strongest positive signals; a skip is a
    /// mild negative signal. Views are the weakest positive signal.
    pub fn weight_for(impression_type: &str) -> f64 {
        match impression_type {
            "view" => 1.0,
            "like" => 5.0,
            "comment" => 4.0,
            "share" => 6.0,
            "skip" => -1.0,
            _ => 0.0,
        }
    }
}
