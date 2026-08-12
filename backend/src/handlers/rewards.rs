use crate::services::traits::RewardServiceTrait;
use crate::{AppState, AuthClaims};
use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

const CCUP_LIMIT_BY_DORM: fn(&str) -> i32 = |dorm| match dorm {
    "Res on Fifth" => 4350,
    "McGill and Boss" => 4350,
    "The Hill" => 4350,
    "Hammerschlag" => 4350,
    "Mudge" => 8670,
    "Stever" => 8670,
    "Morewood E-Tower" => 6090,
    "Morewood Gardens" => 4170,
    "Donner" => 6960,
    "Margaret Morrison" => 8010,
    "Whesco" => 8010,
    _ => -1,
};

#[derive(Serialize, ToSchema)]
pub struct RewardsListResponse {
    pub rewards: Vec<RewardResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct RewardResponse {
    pub name: String,
    pub cost: i32,
    pub stock: i32,
    pub trade_limit: i32,
    pub transaction_info: RewardTransactionInfo,
}

#[derive(Serialize, ToSchema)]
pub struct RewardTransactionInfo {
    pub total_purchased: i32,
    pub incomplete_count: i32,
    pub complete_count: i32,
    pub transactions: Vec<TransactionDetail>,
}

#[derive(Serialize, ToSchema)]
pub struct TransactionDetail {
    pub transaction_id: String,
    pub count: i32,
    pub timestamp: NaiveDateTime,
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/api/rewards",
    responses(
        (status = 200, description = "Rewards retrieved successfully", body = RewardsListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "rewards"
)]
#[axum::debug_handler]
pub async fn get_rewards(
    State(state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
) -> Result<Json<RewardsListResponse>, StatusCode> {
    let rewards = state
        .reward_service
        .get_all_rewards()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let transactions = state
        .transaction_service
        .get_user_transactions(&claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut transactions_by_reward = HashMap::new();
    for transaction in transactions {
        transactions_by_reward
            .entry(transaction.reward_name.clone())
            .or_insert_with(Vec::new)
            .push(transaction);
    }

    let mut reward_responses = Vec::new();

    for reward in rewards {
        let transactions = transactions_by_reward
            .remove(&reward.name)
            .unwrap_or_default();

        let mut total_purchased = 0;
        let mut incomplete_count = 0;
        let mut complete_count = 0;
        let mut transaction_details = Vec::new();
        let mut stock = reward.stock;
        let mut trade_limit = reward.trade_limit;

        for transaction in transactions {
            total_purchased += transaction.count;

            if transaction.status == "complete" {
                complete_count += transaction.count;
            } else {
                incomplete_count += transaction.count;
            }

            transaction_details.push(TransactionDetail {
                transaction_id: transaction.id.to_string(),
                count: transaction.count,
                timestamp: transaction.timestamp,
                status: transaction.status,
            });
        }

        // Sort transactions by timestamp, most recent first
        transaction_details.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if reward.name.eq("Carnegie Cup Contribution") {
            let user_dorm = state
                .user_service
                .get_or_create_user(&claims.sub, &claims.name)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .dorm;

            let user_dorm = user_dorm.as_deref().unwrap_or("Unknown");

            stock = CCUP_LIMIT_BY_DORM(user_dorm);
            trade_limit = stock;
            total_purchased = state
                .transaction_service
                .get_ccup_total_purchased(user_dorm)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .try_into()
                .unwrap();
        }

        reward_responses.push(RewardResponse {
            name: reward.name.clone(),
            cost: reward.cost,
            stock,
            trade_limit,
            transaction_info: RewardTransactionInfo {
                total_purchased,
                incomplete_count,
                complete_count,
                transactions: transaction_details,
            },
        });
    }

    Ok(Json(RewardsListResponse {
        rewards: reward_responses,
    }))
}
