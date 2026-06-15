use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Poll {
    pub id: String,
    pub post_id: String,
    pub question: String,
    pub allows_multiple_answers: bool,
    pub is_active: bool,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PollOption {
    pub id: String,
    pub poll_id: String,
    pub option_text: String,
    pub position: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct PollVote {
    pub id: String,
    pub poll_option_id: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePollRequest {
    #[validate(length(min = 5, max = 500, message = "La pregunta debe tener entre 5 y 500 caracteres"))]
    pub question: String,
    #[validate(length(min = 2, message = "Se necesitan al menos 2 opciones"))]
    pub options: Vec<String>,
    pub allows_multiple_answers: Option<bool>,
    pub expires_at: Option<String>,
    pub community_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VotePollRequest {
    pub option_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PollOptionResponse {
    pub option: PollOption,
    pub votes_count: i64,
    pub has_voted: bool,
}

#[derive(Debug, Serialize)]
pub struct PollResponse {
    pub poll: Poll,
    pub options: Vec<PollOptionResponse>,
    pub total_votes: i64,
    pub user_has_voted: bool,
}
