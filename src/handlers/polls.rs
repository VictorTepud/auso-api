use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::poll::*;

/// POST /api/v1/posts/poll
pub async fn create_poll_post(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<CreatePollRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    if body.options.len() < 2 {
        return Err(ApiError::bad_request("Se necesitan al menos 2 opciones"));
    }

    if body.options.len() > 10 {
        return Err(ApiError::bad_request("Máximo 10 opciones permitidas"));
    }

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = Uuid::new_v4().to_string();
    let poll_id = Uuid::new_v4().to_string();

    // Crear el post
    sqlx::query(
        "INSERT INTO posts (id, user_id, community_id, channel_id, group_id, post_type, content) VALUES (?, ?, ?, ?, ?, 'poll', ?)"
    )
    .bind(&post_id)
    .bind(&user_id)
    .bind(&body.community_id)
    .bind(&body.channel_id)
    .bind(&body.group_id)
    .bind(&body.question)
    .execute(pool.get_ref())
    .await?;

    // Crear la encuesta
    sqlx::query(
        "INSERT INTO polls (id, post_id, question, allows_multiple_answers, expires_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&poll_id)
    .bind(&post_id)
    .bind(&body.question)
    .bind(body.allows_multiple_answers.unwrap_or(false) as bool)
    .bind(&body.expires_at)
    .execute(pool.get_ref())
    .await?;

    // Crear las opciones
    for (idx, option_text) in body.options.iter().enumerate() {
        let option_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO poll_options (id, poll_id, option_text, position) VALUES (?, ?, ?, ?)"
        )
        .bind(&option_id)
        .bind(&poll_id)
        .bind(option_text)
        .bind(idx as i32)
        .execute(pool.get_ref())
        .await?;
    }

    let post = sqlx::query_as::<_, crate::models::post::Post>(
        "SELECT * FROM posts WHERE id = ?"
    )
    .bind(&post_id)
    .fetch_one(pool.get_ref())
    .await?;

    let poll = sqlx::query_as::<_, Poll>(
        "SELECT * FROM polls WHERE id = ?"
    )
    .bind(&poll_id)
    .fetch_one(pool.get_ref())
    .await?;

    let options = sqlx::query_as::<_, PollOption>(
        "SELECT * FROM poll_options WHERE poll_id = ? ORDER BY position"
    )
    .bind(&poll_id)
    .fetch_all(pool.get_ref())
    .await?;

    let option_responses: Vec<PollOptionResponse> = options
        .iter()
        .map(|opt| PollOptionResponse {
            option: opt.clone(),
            votes_count: 0,
            has_voted: false,
        })
        .collect();

    Ok(HttpResponse::Created().json(serde_json::json!({
        "post": post,
        "poll": PollResponse {
            poll,
            options: option_responses,
            total_votes: 0,
            user_has_voted: false,
        },
    })))
}

/// POST /api/v1/polls/{id}/vote
pub async fn vote_poll(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    poll_id: web::Path<String>,
    body: web::Json<VotePollRequest>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let poll_id = poll_id.into_inner();

    let poll = sqlx::query_as::<_, Poll>(
        "SELECT * FROM polls WHERE id = ?"
    )
    .bind(&poll_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Encuesta no encontrada"))?;

    if !poll.is_active {
        return Err(ApiError::bad_request("Esta encuesta está cerrada"));
    }

    // Verificar expiración
    if let Some(ref expires) = poll.expires_at {
        let now = chrono::Utc::now().naive_utc().to_string();
        if expires < &now {
            return Err(ApiError::bad_request("Esta encuesta ha expirado"));
        }
    }

    // Verificar si el usuario ya votó
    let existing_vote = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM poll_votes pv JOIN poll_options po ON pv.poll_option_id = po.id WHERE po.poll_id = ? AND pv.user_id = ?"
    )
    .bind(&poll_id)
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await?;

    if existing_vote > 0 && !poll.allows_multiple_answers {
        return Err(ApiError::conflict("Ya has votado en esta encuesta"));
    }

    if body.option_ids.is_empty() {
        return Err(ApiError::bad_request("Debes seleccionar al menos una opción"));
    }

    // Registrar votos
    for option_id in &body.option_ids {
        // Verificar que la opción pertenece a esta encuesta
        let option_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_options WHERE id = ? AND poll_id = ?"
        )
        .bind(option_id)
        .bind(&poll_id)
        .fetch_one(pool.get_ref())
        .await?;

        if option_exists == 0 {
            continue;
        }

        // Verificar que no votó ya por esta opción
        let already_voted = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ? AND user_id = ?"
        )
        .bind(option_id)
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?;

        if already_voted == 0 {
            let vote_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO poll_votes (id, poll_option_id, user_id) VALUES (?, ?, ?)"
            )
            .bind(&vote_id)
            .bind(option_id)
            .bind(&user_id)
            .execute(pool.get_ref())
            .await?;
        }
    }

    // Retornar resultados actualizados
    let options = sqlx::query_as::<_, PollOption>(
        "SELECT * FROM poll_options WHERE poll_id = ? ORDER BY position"
    )
    .bind(&poll_id)
    .fetch_all(pool.get_ref())
    .await?;

    let mut option_responses = Vec::new();
    let mut total_votes: i64 = 0;

    for opt in &options {
        let votes_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ?"
        )
        .bind(&opt.id)
        .fetch_one(pool.get_ref())
        .await?;

        total_votes += votes_count;

        let has_voted = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ? AND user_id = ?"
        )
        .bind(&opt.id)
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?
        > 0;

        option_responses.push(PollOptionResponse {
            option: opt.clone(),
            votes_count,
            has_voted,
        });
    }

    Ok(HttpResponse::Ok().json(PollResponse {
        poll,
        options: option_responses,
        total_votes,
        user_has_voted: true,
    }))
}

/// GET /api/v1/polls/{id}/results
pub async fn get_poll_results(
    pool: web::Data<SqlitePool>,
    poll_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let poll_id = poll_id.into_inner();

    let poll = sqlx::query_as::<_, Poll>(
        "SELECT * FROM polls WHERE id = ?"
    )
    .bind(&poll_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Encuesta no encontrada"))?;

    let options = sqlx::query_as::<_, PollOption>(
        "SELECT * FROM poll_options WHERE poll_id = ? ORDER BY position"
    )
    .bind(&poll_id)
    .fetch_all(pool.get_ref())
    .await?;

    let mut option_responses = Vec::new();
    let mut total_votes: i64 = 0;

    for opt in &options {
        let votes_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM poll_votes WHERE poll_option_id = ?"
        )
        .bind(&opt.id)
        .fetch_one(pool.get_ref())
        .await?;

        total_votes += votes_count;

        option_responses.push(PollOptionResponse {
            option: opt.clone(),
            votes_count,
            has_voted: false,
        });
    }

    Ok(HttpResponse::Ok().json(PollResponse {
        poll,
        options: option_responses,
        total_votes,
        user_has_voted: false,
    }))
}
