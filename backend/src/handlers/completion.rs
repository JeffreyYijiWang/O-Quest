use std::io::{self, Write};

use crate::services::traits::{ChallengeServiceTrait, CompletionServiceTrait};
use crate::{AppState, AuthClaims};
use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::{NaiveDateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Geodesic distance (Haversine formula)
fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371000.0; // meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

fn circles_overlap(lat1: f64, lon1: f64, radius1: f64, lat2: f64, lon2: f64, radius2: f64) -> bool {
    let distance = calculate_distance(lat1, lon1, lat2, lon2);
    let required_distance = radius1 + radius2;

    distance <= required_distance
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCompletionRequest {
    pub challenge_name: String,
    pub verification_code: String,
    pub image_data: String, // base64 encoded image
    pub note: Option<String>,
    // User location data
    pub user_latitude: f64,
    pub user_longitude: f64,
    pub user_location_accuracy: f64,
}

#[derive(Serialize, ToSchema)]
pub struct CreateCompletionResponse {
    pub success: bool,
    pub message: String,
    pub completion: Option<CompletionResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct CompletionResponse {
    pub challenge_name: String,
    pub completed_at: NaiveDateTime,
    pub coins_earned: i32,
}

#[utoipa::path(
    post,
    path = "/api/complete",
    request_body = CreateCompletionRequest,
    responses(
        (status = 200, description = "Completion processed", body = CreateCompletionResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "completions"
)]
#[axum::debug_handler]
pub async fn create_completion(
    State(state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<CreateCompletionRequest>,
) -> Result<Json<CreateCompletionResponse>, StatusCode> {
    // Get the challenge to verify it exists and get the secret
    let challenge = state
        .challenge_service
        .get_challenge_by_name(&payload.challenge_name)
        .await
        .map_err(|e| {
            eprintln!("Failed to get challenge: {e}");
            io::stderr().flush().unwrap_or_default();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let challenge = match challenge {
        Some(challenge) => challenge,
        None => {
            return Ok(Json(CreateCompletionResponse {
                success: false,
                message: "Challenge not found".to_string(),
                completion: None,
            }));
        }
    };

    // Check if challenge is unlocked in prod
    if challenge.unlock_timestamp > Utc::now().naive_utc() {
        return Ok(Json(CreateCompletionResponse {
            success: false,
            message: "Challenge is not yet unlocked".to_string(),
            completion: None,
        }));
    }

    // Verify the secret
    if !payload.verification_code.contains(&challenge.secret) {
        return Ok(Json(CreateCompletionResponse {
            success: false,
            message: "Invalid verification code".to_string(),
            completion: None,
        }));
    }

    // Check if user already completed this challenge
    let already_completed = state
        .completion_service
        .completion_exists(&claims.sub, &payload.challenge_name)
        .await
        .map_err(|e| {
            eprintln!("Failed to check completion existence: {e}");
            io::stderr().flush().unwrap_or_default();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if already_completed {
        return Ok(Json(CreateCompletionResponse {
            success: false,
            message: "Challenge already completed".to_string(),
            completion: None,
        }));
    }

    // Verify location if challenge has geolocation data set
    if let (Some(challenge_lat), Some(challenge_lon), Some(challenge_accuracy)) = (
        challenge.latitude,
        challenge.longitude,
        challenge.location_accuracy,
    ) {
        let challenge_accuracy_meters = challenge_accuracy.to_f64().unwrap_or(0.0);

        // Check if user's location circle overlaps with challenge location circle
        let overlaps = circles_overlap(
            challenge_lat,
            challenge_lon,
            challenge_accuracy_meters.max(500.0),
            payload.user_latitude,
            payload.user_longitude,
            payload.user_location_accuracy.max(500.0),
        );

        if !overlaps {
            return Ok(Json(CreateCompletionResponse {
                success: false,
                message: "You are not close enough to the challenge location".to_string(),
                completion: None,
            }));
        }
    }
    // If challenge has no location data (latitude, longitude, or accuracy is None),
    // skip location verification and proceed with completion

    // Upload the image to MinIO
    let mut s3_link: String = "".to_string();
    if !payload.image_data.is_empty() {
        s3_link = state
            .storage_service
            .upload_completion_image(&claims.sub, &payload.challenge_name, &payload.image_data)
            .await
            .map_err(|e| {
                eprintln!("Failed to upload image: {e}");
                io::stderr().flush().unwrap_or_default();
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Create the completion with the S3 link
    let completion = state
        .completion_service
        .create_completion(
            &claims.sub,
            &payload.challenge_name,
            if !s3_link.is_empty() {
                Some(s3_link)
            } else {
                None
            },
            payload.note,
        )
        .await
        .map_err(|e| {
            eprintln!("Failed to create completion: {e}");
            io::stderr().flush().unwrap_or_default();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Invalidate caches affected by this completion
    state.cache_manager.invalidate_user_data(&claims.sub).await;
    // Leaderboard pages are 15-second snapshots. Their bounded TTL prevents a
    // write storm from invalidating a shared hot key on every completion.

    Ok(Json(CreateCompletionResponse {
        success: true,
        message: "Challenge completed successfully!".to_string(),
        completion: Some(CompletionResponse {
            challenge_name: completion.challenge_name,
            completed_at: completion.timestamp,
            coins_earned: challenge.scotty_coins,
        }),
    }))
}
