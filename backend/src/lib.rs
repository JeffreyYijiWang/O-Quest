use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod auth;
pub mod cache;
pub mod doc;
pub mod entities;
pub mod handlers;
pub mod middleware;
pub mod services;

use cache::{
    CacheManager, CachedChallengeService, CachedCompletionService, CachedLeaderboardService,
    CachedRewardService,
};
use services::{
    challenge::ChallengeService, completion::CompletionService, leaderboard::LeaderboardService,
    reward::RewardService, storage::StorageService, transaction::TransactionService,
    user::UserService,
};

#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
    pub challenge_service: CachedChallengeService<ChallengeService>,
    pub completion_service: CachedCompletionService<CompletionService>,
    pub reward_service: CachedRewardService<RewardService>,
    pub transaction_service: TransactionService,
    pub leaderboard_service: CachedLeaderboardService<LeaderboardService>,
    pub storage_service: StorageService,
    pub cache_manager: Arc<CacheManager>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub auth_time: i64,
    pub acr: String,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    pub given_name: String,
    pub preferred_username: String,
    pub nickname: String,
    pub groups: Vec<String>,
}

pub async fn create_connection() -> Result<DatabaseConnection, DbErr> {
    dotenvy::dotenv().ok();

    let database_url =
        dotenvy::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    let max_connections = dotenvy::var("DATABASE_MAX_CONNECTIONS")
        .or_else(|_| dotenvy::var("DB_MAX_CONNECTIONS"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(64);
    let min_connections = dotenvy::var("DATABASE_MIN_CONNECTIONS")
        .or_else(|_| dotenvy::var("DB_MIN_CONNECTIONS"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8);
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(max_connections)
        .min_connections(min_connections.min(max_connections))
        .connect_timeout(std::time::Duration::from_secs(5))
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(5 * 60))
        .max_lifetime(std::time::Duration::from_secs(30 * 60))
        .sqlx_logging(false);

    Database::connect(opt).await
}
