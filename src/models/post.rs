use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: String,
    pub user_id: String,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
    pub post_type: String,
    pub content: String,
    pub background_color: Option<String>,
    pub background_image_url: Option<String>,
    pub layout_mode: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostImage {
    pub id: String,
    pub post_id: String,
    pub image_url: String,
    pub position: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostVideo {
    pub id: String,
    pub post_id: String,
    pub original_filename: String,
    pub hls_master_playlist_url: String,
    pub duration: f64,
    pub width: i32,
    pub height: i32,
    pub thumbnail_url: Option<String>,
    pub title: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTextPostRequest {
    #[validate(length(max = 5000, message = "El contenido no puede exceder 5000 caracteres"))]
    pub content: String,
    pub background_color: Option<String>,
    pub background_image_url: Option<String>,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateImagePostRequest {
    pub content: Option<String>,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateImagePackPostRequest {
    #[validate(length(max = 1000))]
    pub content: Option<String>,
    pub layout_mode: Option<String>,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateVideoPostRequest {
    pub content: Option<String>,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub post: Post,
    pub images: Vec<PostImage>,
    pub video: Option<PostVideo>,
    pub poll: Option<super::poll::PollResponse>,
    pub likes_count: i64,
    pub comments_count: i64,
    pub is_liked: bool,
    pub author_username: String,
    pub author_display_name: String,
    pub author_profile_photo: Option<String>,
    /// Hashtags linked to this post (without the # prefix). Empty if none.
    #[serde(default)]
    pub hashtags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PostWithAuthor {
    pub id: String,
    pub user_id: String,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
    pub post_type: String,
    pub content: String,
    pub background_color: Option<String>,
    pub background_image_url: Option<String>,
    pub layout_mode: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: String,
    pub author_profile_photo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub community_id: Option<String>,
    pub group_id: Option<String>,
    pub user_id: Option<String>,
    pub post_type: Option<String>,
}
