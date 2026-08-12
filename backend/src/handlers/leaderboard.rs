use crate::AppState;
use crate::services::leaderboard::{LeaderboardCursor, LeaderboardEntry};
use crate::services::traits::LeaderboardServiceTrait;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub cursor: Option<String>,
}

fn default_limit() -> u64 {
    20
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardResponse {
    pub entries: Vec<LeaderboardEntry>,
    pub has_next: bool,
    pub next_cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/leaderboard",
    params(
        ("limit" = Option<u64>, Query, description = "Number of entries to return (max 100, default 20)"),
        ("cursor" = Option<String>, Query, description = "Opaque snapshot-stable keyset cursor")
    ),
    responses(
        (status = 200, description = "Leaderboard retrieved successfully", body = LeaderboardResponse),
        (status = 400, description = "Malformed cursor"),
        (status = 500, description = "Internal server error")
    ),
    tag = "leaderboard"
)]
#[axum::debug_handler]
pub async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, StatusCode> {
    let limit = std::cmp::min(params.limit, 100);
    let cursor = match params.cursor.as_deref() {
        Some(value) => LeaderboardCursor::decode(value).map_err(|_| StatusCode::BAD_REQUEST)?,
        None => LeaderboardCursor::start(),
    };

    let mut entries = state
        .leaderboard_service
        .get_leaderboard_page(limit + 1, Some(&cursor))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let has_next = entries.len() > limit as usize;
    if has_next {
        entries.pop();
    }

    let next_cursor = if has_next {
        entries
            .last()
            .map(|entry| cursor.after(entry).encode())
            .transpose()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        None
    };

    Ok(Json(LeaderboardResponse {
        entries,
        has_next,
        next_cursor,
    }))
}
