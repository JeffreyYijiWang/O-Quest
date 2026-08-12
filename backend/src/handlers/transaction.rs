use crate::services::traits::{CompletionServiceTrait, RewardServiceTrait};
use crate::{AppState, AuthClaims};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateTransactionRequest {
    pub reward_name: String,
    pub count: i32,
}

#[derive(Serialize, ToSchema)]
pub struct CreateTransactionResponse {
    pub success: bool,
    pub message: String,
    pub transaction: Option<TransactionResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct TransactionResponse {
    pub transaction_id: String,
    pub reward_name: String,
    pub count: i32,
    pub total_cost: i32,
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/transaction",
    request_body = CreateTransactionRequest,
    responses(
        (status = 200, description = "Transaction processed", body = CreateTransactionResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "transactions"
)]
#[axum::debug_handler]
pub async fn create_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<Json<CreateTransactionResponse>, StatusCode> {
    // Validate count is positive
    if payload.count <= 0 {
        return Ok(Json(CreateTransactionResponse {
            success: false,
            message: "Count must be positive".to_string(),
            transaction: None,
        }));
    }

    // Get the reward to check if it exists and get cost
    let reward = state
        .reward_service
        .get_reward_by_name(&payload.reward_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let reward = match reward {
        Some(reward) => reward,
        None => {
            return Ok(Json(CreateTransactionResponse {
                success: false,
                message: "Reward not found".to_string(),
                transaction: None,
            }));
        }
    };
    // Check if there's enough stock
    if reward.stock >= 0 && payload.count > reward.stock {
        return Ok(Json(CreateTransactionResponse {
            success: false,
            message: "Insufficient stock".to_string(),
            transaction: None,
        }));
    }

    // Get user's current total count for this reward and check trade limit
    let current_totals = state
        .transaction_service
        .get_user_total_counts(&claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let current_count = current_totals
        .get(&payload.reward_name)
        .copied()
        .unwrap_or(0);

    if reward.trade_limit >= 0 && current_count + payload.count > reward.trade_limit {
        return Ok(Json(CreateTransactionResponse {
            success: false,
            message: "Would exceed trade limit".to_string(),
            transaction: None,
        }));
    }

    // Check if user has enough coins
    let (coins_earned, coins_spent) = tokio::try_join!(
        state
            .completion_service
            .get_user_total_coins_earned(&claims.sub),
        state
            .transaction_service
            .get_user_total_coins_spent(&claims.sub)
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let available_coins = coins_earned - coins_spent;
    let total_cost = reward.cost * payload.count;

    if total_cost > available_coins {
        return Ok(Json(CreateTransactionResponse {
            success: false,
            message: "Insufficient coins".to_string(),
            transaction: None,
        }));
    }

    // Create the transaction
    let transaction = state
        .transaction_service
        .create_transaction(&claims.sub, &payload.reward_name, payload.count)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Invalidate caches affected by this transaction
    state.cache_manager.invalidate_user_data(&claims.sub).await;
    state.cache_manager.invalidate_rewards().await;

    Ok(Json(CreateTransactionResponse {
        success: true,
        message: "Transaction successful".to_string(),
        transaction: Some(TransactionResponse {
            transaction_id: transaction.id.to_string(),
            reward_name: transaction.reward_name,
            count: transaction.count,
            total_cost,
            status: transaction.status,
        }),
    }))
}

#[derive(Serialize, ToSchema)]
pub struct CancelTransactionResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    delete,
    path = "/api/transaction/{transaction_id}",
    params(
        ("transaction_id" = String, Path, description = "ID of the transaction to cancel")
    ),
    responses(
        (status = 200, description = "Transaction cancellation processed", body = CancelTransactionResponse),
        (status = 403, description = "Cannot cancel this transaction"),
        (status = 404, description = "Transaction not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "transactions"
)]
#[axum::debug_handler]
pub async fn cancel_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(transaction_id): Path<String>,
) -> Result<Json<CancelTransactionResponse>, StatusCode> {
    // Get the transaction to verify ownership and status
    let transaction = state
        .transaction_service
        .get_transaction_by_id(&transaction_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let transaction = match transaction {
        Some(transaction) => transaction,
        None => {
            return Ok(Json(CancelTransactionResponse {
                success: false,
                message: "Transaction not found".to_string(),
            }));
        }
    };

    // Check if user owns this transaction
    if transaction.user_id != claims.sub {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if transaction can be cancelled (only incomplete transactions)
    if transaction.status != "incomplete" {
        return Ok(Json(CancelTransactionResponse {
            success: false,
            message: "Cannot cancel completed transactions".to_string(),
        }));
    }

    // Delete the transaction
    let deleted = state
        .transaction_service
        .delete_transaction(&transaction_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        // Restore stock when transaction is cancelled
        let reward = state
            .reward_service
            .get_reward_by_name(&transaction.reward_name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(reward) = reward {
            // Only restore stock if stock tracking is enabled
            if reward.stock != -1 {
                state
                    .reward_service
                    .increment_stock(&transaction.reward_name, transaction.count)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        }

        // Invalidate caches affected by transaction cancellation
        state.cache_manager.invalidate_user_data(&claims.sub).await;
        state.cache_manager.invalidate_rewards().await;
    }

    let (success, message) = if deleted {
        (true, "Transaction cancelled successfully".to_string())
    } else {
        (false, "Failed to cancel transaction".to_string())
    };

    Ok(Json(CancelTransactionResponse { success, message }))
}
