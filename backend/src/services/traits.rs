use crate::entities::{challenges, completion, reward};
use crate::services::leaderboard::LeaderboardCursor;
use crate::services::leaderboard::LeaderboardEntry;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::collections::HashMap;

#[async_trait]
pub trait ChallengeServiceTrait: Clone + Send + Sync {
    async fn get_all_challenges(&self) -> Result<Vec<challenges::Model>, sea_orm::DbErr>;
    async fn get_challenge_by_name(
        &self,
        name: &str,
    ) -> Result<Option<challenges::Model>, sea_orm::DbErr>;
    async fn update_challenge_geolocation(
        &self,
        name: &str,
        latitude: f64,
        longitude: f64,
        location_accuracy: f64,
    ) -> Result<Option<challenges::Model>, sea_orm::DbErr>;
    async fn get_total_challenges_by_category(
        &self,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr>;
    async fn get_total_challenge_count(&self) -> Result<i32, sea_orm::DbErr>;
    async fn upsert_challenges_batch(
        &self,
        challenges: Vec<challenges::ActiveModel>,
    ) -> Result<usize, sea_orm::DbErr>;
}

#[async_trait]
pub trait CompletionServiceTrait: Clone + Send + Sync {
    async fn get_user_completion_map(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, NaiveDateTime>, sea_orm::DbErr>;
    async fn create_completion(
        &self,
        user_id: &str,
        challenge_name: &str,
        s3_link: Option<String>,
        note: Option<String>,
    ) -> Result<completion::Model, sea_orm::DbErr>;
    async fn completion_exists(
        &self,
        user_id: &str,
        challenge_name: &str,
    ) -> Result<bool, sea_orm::DbErr>;
    async fn get_user_completions_by_category(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr>;
    async fn get_user_completion_count(&self, user_id: &str) -> Result<i32, sea_orm::DbErr>;
    async fn get_user_recent_activity_days(
        &self,
        user_id: &str,
        num_days_back: i64,
    ) -> Result<Vec<NaiveDateTime>, sea_orm::DbErr>;
    async fn get_user_total_coins_earned(&self, user_id: &str) -> Result<i32, sea_orm::DbErr>;
    async fn get_user_completions_with_challenges(
        &self,
        user_id: &str,
    ) -> Result<Vec<(completion::Model, challenges::Model)>, sea_orm::DbErr>;
    async fn get_user_completion_with_challenge(
        &self,
        user_id: &str,
        challenge_name: &str,
    ) -> Result<Option<(completion::Model, challenges::Model)>, sea_orm::DbErr>;
    async fn update_completion_note(
        &self,
        user_id: &str,
        challenge_name: &str,
        note: Option<String>,
    ) -> Result<Option<completion::Model>, sea_orm::DbErr>;
    async fn update_completion_photo(
        &self,
        user_id: &str,
        challenge_name: &str,
        s3_link: Option<String>,
    ) -> Result<Option<completion::Model>, sea_orm::DbErr>;
}

#[async_trait]
pub trait RewardServiceTrait: Clone + Send + Sync {
    async fn get_all_rewards(&self) -> Result<Vec<reward::Model>, sea_orm::DbErr>;
    async fn get_reward_by_name(&self, name: &str)
    -> Result<Option<reward::Model>, sea_orm::DbErr>;
    async fn upsert_rewards_batch(
        &self,
        rewards: Vec<reward::ActiveModel>,
    ) -> Result<usize, sea_orm::DbErr>;
    async fn decrement_stock(
        &self,
        reward_name: &str,
        amount: i32,
    ) -> Result<Option<reward::Model>, sea_orm::DbErr>;
    async fn increment_stock(
        &self,
        reward_name: &str,
        amount: i32,
    ) -> Result<Option<reward::Model>, sea_orm::DbErr>;
}

#[async_trait]
pub trait LeaderboardServiceTrait: Clone + Send + Sync {
    async fn get_leaderboard_page(
        &self,
        limit: u64,
        cursor: Option<&LeaderboardCursor>,
    ) -> Result<Vec<LeaderboardEntry>, sea_orm::DbErr>;
    async fn get_user_leaderboard_position(&self, user_id: &str) -> Result<i64, sea_orm::DbErr>;
    async fn get_all_user_leaderboard_positions(
        &self,
    ) -> Result<HashMap<String, i64>, sea_orm::DbErr>;
}
