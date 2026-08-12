use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use crate::cache::CacheManager;
use crate::entities::{challenges, completion, reward};
use crate::services::leaderboard::{LeaderboardCursor, LeaderboardEntry};
use crate::services::traits::*;

#[derive(Clone)]
pub struct CachedChallengeService<T: ChallengeServiceTrait> {
    inner: T,
    cache: Arc<CacheManager>,
}

#[derive(Clone)]
pub struct CachedCompletionService<T: CompletionServiceTrait> {
    inner: T,
    cache: Arc<CacheManager>,
}

#[derive(Clone)]
pub struct CachedRewardService<T: RewardServiceTrait> {
    inner: T,
    cache: Arc<CacheManager>,
}

#[derive(Clone)]
pub struct CachedLeaderboardService<T: LeaderboardServiceTrait> {
    inner: T,
    cache: Arc<CacheManager>,
    position_snapshot: Arc<RwLock<Option<PositionSnapshot>>>,
    position_refresh: Arc<Mutex<()>>,
}

struct PositionSnapshot {
    expires_at: Instant,
    positions: HashMap<String, i64>,
}

// Challenge service implementation
impl<T: ChallengeServiceTrait> CachedChallengeService<T> {
    pub fn new(inner: T, cache: Arc<CacheManager>) -> Self {
        Self { inner, cache }
    }
}

#[async_trait]
impl<T: ChallengeServiceTrait> ChallengeServiceTrait for CachedChallengeService<T> {
    async fn get_all_challenges(&self) -> Result<Vec<challenges::Model>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_challenges().await {
            return Ok(cached);
        }

        let result = self.inner.get_all_challenges().await?;
        self.cache.set_challenges(result.clone()).await;
        Ok(result)
    }

    async fn get_challenge_by_name(
        &self,
        name: &str,
    ) -> Result<Option<challenges::Model>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_challenge_by_name(name).await {
            return Ok(cached);
        }

        let result = self.inner.get_challenge_by_name(name).await?;
        self.cache.set_challenge_by_name(name, result.clone()).await;
        Ok(result)
    }

    async fn update_challenge_geolocation(
        &self,
        name: &str,
        latitude: f64,
        longitude: f64,
        location_accuracy: f64,
    ) -> Result<Option<challenges::Model>, sea_orm::DbErr> {
        self.inner
            .update_challenge_geolocation(name, latitude, longitude, location_accuracy)
            .await
    }

    async fn get_total_challenges_by_category(
        &self,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_challenge_counts().await {
            return Ok(cached);
        }

        let result = self.inner.get_total_challenges_by_category().await?;
        self.cache.set_challenge_counts(result.clone()).await;
        Ok(result)
    }

    async fn get_total_challenge_count(&self) -> Result<i32, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_total_challenge_count().await {
            return Ok(cached);
        }

        let result = self.inner.get_total_challenge_count().await?;
        self.cache.set_total_challenge_count(result).await;
        Ok(result)
    }

    async fn upsert_challenges_batch(
        &self,
        challenges: Vec<challenges::ActiveModel>,
    ) -> Result<usize, sea_orm::DbErr> {
        self.inner.upsert_challenges_batch(challenges).await
    }
}

// Completion service implementation
impl<T: CompletionServiceTrait> CachedCompletionService<T> {
    pub fn new(inner: T, cache: Arc<CacheManager>) -> Self {
        Self { inner, cache }
    }
}

#[async_trait]
impl<T: CompletionServiceTrait> CompletionServiceTrait for CachedCompletionService<T> {
    async fn get_user_completion_map(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, NaiveDateTime>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_user_completions(user_id).await {
            return Ok(cached);
        }

        let result = self.inner.get_user_completion_map(user_id).await?;
        self.cache
            .set_user_completions(user_id, result.clone())
            .await;
        Ok(result)
    }

    async fn create_completion(
        &self,
        user_id: &str,
        challenge_name: &str,
        s3_link: Option<String>,
        note: Option<String>,
    ) -> Result<completion::Model, sea_orm::DbErr> {
        self.inner
            .create_completion(user_id, challenge_name, s3_link, note)
            .await
    }

    async fn completion_exists(
        &self,
        user_id: &str,
        challenge_name: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        // For exists checks, we can use the cached completion map if available
        if let Some(completions) = self.cache.get_user_completions(user_id).await {
            return Ok(completions.contains_key(challenge_name));
        }

        // Otherwise, fall back to the database
        self.inner.completion_exists(user_id, challenge_name).await
    }

    async fn get_user_completions_by_category(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_user_completions_by_category(user_id).await {
            return Ok(cached);
        }

        let result = self.inner.get_user_completions_by_category(user_id).await?;
        self.cache
            .set_user_completions_by_category(user_id, result.clone())
            .await;
        Ok(result)
    }

    async fn get_user_completion_count(&self, user_id: &str) -> Result<i32, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_user_completion_count(user_id).await {
            return Ok(cached);
        }

        let result = self.inner.get_user_completion_count(user_id).await?;
        self.cache.set_user_completion_count(user_id, result).await;
        Ok(result)
    }

    async fn get_user_recent_activity_days(
        &self,
        user_id: &str,
        num_days_back: i64,
    ) -> Result<Vec<NaiveDateTime>, sea_orm::DbErr> {
        if let Some(cached) = self
            .cache
            .get_user_recent_activity(user_id, num_days_back)
            .await
        {
            return Ok(cached);
        }

        let result = self
            .inner
            .get_user_recent_activity_days(user_id, num_days_back)
            .await?;
        self.cache
            .set_user_recent_activity(user_id, num_days_back, result.clone())
            .await;
        Ok(result)
    }

    async fn get_user_total_coins_earned(&self, user_id: &str) -> Result<i32, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_user_coins_earned(user_id).await {
            return Ok(cached);
        }

        let result = self.inner.get_user_total_coins_earned(user_id).await?;
        self.cache.set_user_coins_earned(user_id, result).await;
        Ok(result)
    }

    async fn get_user_completions_with_challenges(
        &self,
        user_id: &str,
    ) -> Result<Vec<(completion::Model, challenges::Model)>, sea_orm::DbErr> {
        if let Some(cached) = self
            .cache
            .get_user_completions_with_challenges(user_id)
            .await
        {
            return Ok(cached);
        }

        let result = self
            .inner
            .get_user_completions_with_challenges(user_id)
            .await?;
        self.cache
            .set_user_completions_with_challenges(user_id, result.clone())
            .await;
        Ok(result)
    }

    async fn get_user_completion_with_challenge(
        &self,
        user_id: &str,
        challenge_name: &str,
    ) -> Result<Option<(completion::Model, challenges::Model)>, sea_orm::DbErr> {
        // For individual completions, we can check if it exists in the user's completion map first
        if let Some(completion_map) = self.cache.get_user_completions(user_id).await
            && !completion_map.contains_key(challenge_name)
        {
            return Ok(None);
        }

        self.inner
            .get_user_completion_with_challenge(user_id, challenge_name)
            .await
    }

    async fn update_completion_note(
        &self,
        user_id: &str,
        challenge_name: &str,
        note: Option<String>,
    ) -> Result<Option<completion::Model>, sea_orm::DbErr> {
        self.inner
            .update_completion_note(user_id, challenge_name, note)
            .await
    }

    async fn update_completion_photo(
        &self,
        user_id: &str,
        challenge_name: &str,
        s3_link: Option<String>,
    ) -> Result<Option<completion::Model>, sea_orm::DbErr> {
        self.inner
            .update_completion_photo(user_id, challenge_name, s3_link)
            .await
    }
}

// Reward service implementation
impl<T: RewardServiceTrait> CachedRewardService<T> {
    pub fn new(inner: T, cache: Arc<CacheManager>) -> Self {
        Self { inner, cache }
    }
}

#[async_trait]
impl<T: RewardServiceTrait> RewardServiceTrait for CachedRewardService<T> {
    async fn get_all_rewards(&self) -> Result<Vec<reward::Model>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_rewards().await {
            return Ok(cached);
        }

        let result = self.inner.get_all_rewards().await?;
        self.cache.set_rewards(result.clone()).await;
        Ok(result)
    }

    async fn get_reward_by_name(
        &self,
        name: &str,
    ) -> Result<Option<reward::Model>, sea_orm::DbErr> {
        if let Some(cached) = self.cache.get_reward_by_name(name).await {
            return Ok(cached);
        }

        let result = self.inner.get_reward_by_name(name).await?;
        self.cache.set_reward_by_name(name, result.clone()).await;
        Ok(result)
    }

    async fn upsert_rewards_batch(
        &self,
        rewards: Vec<reward::ActiveModel>,
    ) -> Result<usize, sea_orm::DbErr> {
        self.inner.upsert_rewards_batch(rewards).await
    }

    async fn decrement_stock(
        &self,
        reward_name: &str,
        amount: i32,
    ) -> Result<Option<reward::Model>, sea_orm::DbErr> {
        let result = self.inner.decrement_stock(reward_name, amount).await?;

        // Invalidate reward caches when stock changes
        if result.is_some() {
            self.cache.invalidate_rewards().await;
        }

        Ok(result)
    }

    async fn increment_stock(
        &self,
        reward_name: &str,
        amount: i32,
    ) -> Result<Option<reward::Model>, sea_orm::DbErr> {
        let result = self.inner.increment_stock(reward_name, amount).await?;

        // Invalidate reward caches when stock changes
        if result.is_some() {
            self.cache.invalidate_rewards().await;
        }

        Ok(result)
    }
}

// Leaderboard service implementation
impl<T: LeaderboardServiceTrait> CachedLeaderboardService<T> {
    pub fn new(inner: T, cache: Arc<CacheManager>) -> Self {
        Self {
            inner,
            cache,
            position_snapshot: Arc::new(RwLock::new(None)),
            position_refresh: Arc::new(Mutex::new(())),
        }
    }

    async fn position_from_snapshot(&self, user_id: &str) -> Option<i64> {
        let snapshot = self.position_snapshot.read().await;
        let snapshot = snapshot
            .as_ref()
            .filter(|value| value.expires_at > Instant::now())?;
        Some(
            snapshot
                .positions
                .get(user_id)
                .copied()
                // A just-created user may not be present until the next refresh.
                // With no completions yet, the bounded-staleness position is last.
                .unwrap_or(snapshot.positions.len() as i64 + 1),
        )
    }
}

#[async_trait]
impl<T: LeaderboardServiceTrait> LeaderboardServiceTrait for CachedLeaderboardService<T> {
    async fn get_leaderboard_page(
        &self,
        limit: u64,
        cursor: Option<&LeaderboardCursor>,
    ) -> Result<Vec<LeaderboardEntry>, sea_orm::DbErr> {
        let encoded_cursor = cursor.and_then(|value| value.encode().ok());
        if let Some(cached) = self
            .cache
            .get_leaderboard_page(limit, encoded_cursor.as_deref())
            .await
        {
            return Ok(cached);
        }

        let result = self.inner.get_leaderboard_page(limit, cursor).await?;
        self.cache
            .set_leaderboard_page(limit, encoded_cursor.as_deref(), result.clone())
            .await;
        Ok(result)
    }

    async fn get_user_leaderboard_position(&self, user_id: &str) -> Result<i64, sea_orm::DbErr> {
        if let Some(position) = self.position_from_snapshot(user_id).await {
            return Ok(position);
        }

        let _refresh = self.position_refresh.lock().await;
        if let Some(position) = self.position_from_snapshot(user_id).await {
            return Ok(position);
        }

        let positions = self.inner.get_all_user_leaderboard_positions().await?;
        let position = positions
            .get(user_id)
            .copied()
            .unwrap_or(positions.len() as i64 + 1);
        *self.position_snapshot.write().await = Some(PositionSnapshot {
            expires_at: Instant::now() + Duration::from_secs(15),
            positions,
        });
        Ok(position)
    }

    async fn get_all_user_leaderboard_positions(
        &self,
    ) -> Result<HashMap<String, i64>, sea_orm::DbErr> {
        self.inner.get_all_user_leaderboard_positions().await
    }
}
