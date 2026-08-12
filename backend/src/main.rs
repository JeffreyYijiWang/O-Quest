use axum::http::Method;
use axum::routing::{get, post};
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

use backend::auth;
use backend::cache::{
    CacheManager, CachedChallengeService, CachedCompletionService, CachedLeaderboardService,
    CachedRewardService,
};
use backend::doc::ApiDoc;
use backend::middleware::admin;
use backend::services::{
    challenge::ChallengeService, completion::CompletionService, leaderboard::LeaderboardService,
    reward::RewardService, storage::StorageService, transaction::TransactionService,
    user::UserService,
};
use backend::{create_connection, handlers};

// Public endpoint handlers
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "API information", body = String)
    ),
    tag = "public"
)]
async fn root() -> &'static str {
    "O-Quest API v2; OpenAPI document: /openapi.json"
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check", body = String)
    ),
    tag = "public"
)]
async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db = create_connection().await?;

    // Create cache manager
    let cache_manager = Arc::new(CacheManager::new());

    let storage_service = StorageService::new(
        &dotenvy::var("MINIO_ENDPOINT").expect("MINIO_ENDPOINT must be set"),
        &dotenvy::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY must be set"),
        &dotenvy::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY must be set"),
        dotenvy::var("MINIO_BUCKET").expect("MINIO_BUCKET must be set"),
    )?;

    // Create base services
    let user_service = UserService::new(db.clone()); // not cached
    let challenge_service = ChallengeService::new(db.clone());
    let completion_service = CompletionService::new(db.clone());
    let reward_service = RewardService::new(db.clone());
    let transaction_service = TransactionService::new(db.clone()); // not cached
    let leaderboard_service = LeaderboardService::new(db.clone());

    // Wrap with caching
    let state = backend::AppState {
        user_service,
        challenge_service: CachedChallengeService::new(challenge_service, cache_manager.clone()),
        completion_service: CachedCompletionService::new(completion_service, cache_manager.clone()),
        reward_service: CachedRewardService::new(reward_service, cache_manager.clone()),
        transaction_service,
        leaderboard_service: CachedLeaderboardService::new(
            leaderboard_service,
            cache_manager.clone(),
        ),
        storage_service,
        cache_manager,
    };

    let admin_routes = OpenApiRouter::new()
        .routes(routes!(
            handlers::admin::verify_transaction,
            handlers::admin::get_all_challenges,
            handlers::admin::put_challenge_geolocation,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            admin::require_admin,
        ));

    let mut protected_routes = OpenApiRouter::new()
        .routes(routes!(
            handlers::user::get_profile,
            handlers::user::update_dorm,
        ))
        .routes(routes!(handlers::challenges::get_challenges,))
        .routes(routes!(handlers::rewards::get_rewards,))
        .routes(routes!(handlers::leaderboard::get_leaderboard,))
        .routes(routes!(
            handlers::transaction::create_transaction,
            handlers::transaction::cancel_transaction,
        ))
        .routes(routes!(handlers::completion::create_completion,))
        .routes(routes!(handlers::journal::get_journal,))
        .routes(routes!(
            handlers::journal::get_journal_entry,
            handlers::journal::update_journal_entry,
            handlers::journal::delete_journal_photo,
        ))
        .merge(admin_routes)
        .layer(build_cors_layer())
        .with_state(state.clone());

    protected_routes = protected_routes.layer(axum::middleware::from_fn(auth::auth_middleware));

    let public_routes = OpenApiRouter::new()
        .routes(routes!(root))
        .merge(OpenApiRouter::new().routes(routes!(health)));

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(protected_routes)
        .merge(public_routes)
        .split_for_parts();

    let metrics_cache = state.cache_manager.clone();
    let openapi = Arc::new(api);
    let app = router
        .route(
            "/openapi.json",
            get({
                let openapi = openapi.clone();
                move || {
                    let openapi = openapi.clone();
                    async move { axum::Json((*openapi).clone()) }
                }
            }),
        )
        .route(
            "/metrics/cache",
            get(move || {
                let cache = metrics_cache.clone();
                async move { axum::Json(cache.metrics_snapshot()) }
            }),
        )
        .route("/api/auth/session", post(auth::create_session))
        .route("/oauth2/authorization/quest", get(auth::login_redirect))
        .route("/logout", post(auth::logout))
        .layer(axum::middleware::from_fn(
            backend::middleware::compression::gzip_responses,
        ));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_cors_layer() -> CorsLayer {
    let configured = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:1420,http://tauri.localhost,tauri://localhost".to_string()
    });
    let origins = configured
        .split(',')
        .filter_map(|origin| origin.trim().parse().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::ORIGIN,
        ])
        .allow_credentials(true)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
