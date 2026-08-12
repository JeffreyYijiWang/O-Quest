use crate::cache::redis::RedisPool;
use crate::entities::{challenges, completion, reward};
use crate::services::leaderboard::LeaderboardEntry;
use chrono::NaiveDateTime;
use moka::future::Cache;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const CHALLENGE_TTL_SECONDS: u64 = 60 * 60;
const USER_TTL_SECONDS: u64 = 5 * 60;
const REWARD_TTL_SECONDS: u64 = 60;
const LEADERBOARD_TTL_SECONDS: u64 = 15;
const USER_POSITION_TTL_SECONDS: u64 = 30;

#[derive(Debug, Clone, Serialize)]
pub struct CacheMetricsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub writes: u64,
    pub invalidations: u64,
    pub errors: u64,
    pub hit_rate_percent: f64,
}

#[derive(Default)]
struct L1Metrics {
    hits: AtomicU64,
}

#[derive(Clone)]
pub struct CacheManager {
    redis: Arc<RedisPool>,
    l1: Cache<String, String>,
    versions: Cache<String, i64>,
    l1_metrics: Arc<L1Metrics>,
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheManager {
    pub fn new() -> Self {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let pool_size = std::env::var("REDIS_POOL_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(24);

        Self {
            redis: Arc::new(RedisPool::new(&redis_url, pool_size, "quest:v2")),
            l1: Cache::builder()
                .max_capacity(50_000)
                .time_to_live(Duration::from_secs(5))
                .build(),
            versions: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(1))
                .build(),
            l1_metrics: Arc::new(L1Metrics::default()),
        }
    }

    pub fn metrics_snapshot(&self) -> CacheMetricsSnapshot {
        let mut snapshot = self.redis.metrics_snapshot();
        snapshot.hits += self.l1_metrics.hits.load(Ordering::Relaxed);
        let requests = snapshot.hits + snapshot.misses;
        snapshot.hit_rate_percent = if requests == 0 {
            0.0
        } else {
            snapshot.hits as f64 * 100.0 / requests as f64
        };
        snapshot
    }

    async fn version(&self, family: &str) -> i64 {
        if let Some(version) = self.versions.get(family).await {
            return version;
        }
        let version = self.redis.version(family).await.unwrap_or(0);
        self.versions.insert(family.to_string(), version).await;
        version
    }

    async fn bump_version(&self, family: &str) {
        if let Ok(version) = self.redis.bump_version(family).await {
            self.versions.insert(family.to_string(), version).await;
        } else {
            self.versions.invalidate(family).await;
        }
    }

    async fn versioned_key(&self, family: &str, suffix: &str) -> String {
        let version = self.version(family).await;
        format!("{family}:v{version}:{suffix}")
    }

    async fn user_key(&self, user_id: &str, suffix: &str) -> String {
        let family = format!("user:{user_id}");
        self.versioned_key(&family, suffix).await
    }

    async fn get_cached<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned + Serialize,
    {
        if let Some(json) = self.l1.get(key).await {
            if let Ok(value) = serde_json::from_str(&json) {
                self.l1_metrics.hits.fetch_add(1, Ordering::Relaxed);
                return Some(value);
            }
            self.l1.invalidate(key).await;
        }

        let value: Option<T> = self.redis.get_json(key).await.ok().flatten();
        if let Some(value) = value.as_ref()
            && let Ok(json) = serde_json::to_string(value)
        {
            self.l1.insert(key.to_string(), json).await;
        }
        value
    }

    async fn set_cached<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: u64) {
        if let Ok(json) = serde_json::to_string(value) {
            self.l1.insert(key.to_string(), json).await;
        }
        let _ = self.redis.set_json(key, value, ttl_seconds).await;
    }

    pub async fn get_challenges(&self) -> Option<Vec<challenges::Model>> {
        let key = self.versioned_key("challenges", "all").await;
        self.get_cached(&key).await
    }

    pub async fn set_challenges(&self, value: Vec<challenges::Model>) {
        let key = self.versioned_key("challenges", "all").await;
        self.set_cached(&key, &value, CHALLENGE_TTL_SECONDS).await;
    }

    pub async fn get_challenge_by_name(&self, name: &str) -> Option<Option<challenges::Model>> {
        let key = self
            .versioned_key("challenges", &format!("name:{name}"))
            .await;
        self.get_cached(&key).await
    }

    pub async fn set_challenge_by_name(&self, name: &str, value: Option<challenges::Model>) {
        let key = self
            .versioned_key("challenges", &format!("name:{name}"))
            .await;
        self.set_cached(&key, &value, CHALLENGE_TTL_SECONDS).await;
    }

    pub async fn get_challenge_counts(&self) -> Option<HashMap<String, i32>> {
        let key = self.versioned_key("challenges", "counts").await;
        self.get_cached(&key).await
    }

    pub async fn set_challenge_counts(&self, value: HashMap<String, i32>) {
        let key = self.versioned_key("challenges", "counts").await;
        self.set_cached(&key, &value, CHALLENGE_TTL_SECONDS).await;
    }

    pub async fn get_total_challenge_count(&self) -> Option<i32> {
        let key = self.versioned_key("challenges", "total").await;
        self.get_cached(&key).await
    }

    pub async fn set_total_challenge_count(&self, value: i32) {
        let key = self.versioned_key("challenges", "total").await;
        self.set_cached(&key, &value, CHALLENGE_TTL_SECONDS).await;
    }

    pub async fn get_user_completions(
        &self,
        user_id: &str,
    ) -> Option<HashMap<String, NaiveDateTime>> {
        let key = self.user_key(user_id, "completions").await;
        self.get_cached(&key).await
    }

    pub async fn set_user_completions(&self, user_id: &str, value: HashMap<String, NaiveDateTime>) {
        let key = self.user_key(user_id, "completions").await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_user_completion_count(&self, user_id: &str) -> Option<i32> {
        let key = self.user_key(user_id, "completion-count").await;
        self.get_cached(&key).await
    }

    pub async fn set_user_completion_count(&self, user_id: &str, value: i32) {
        let key = self.user_key(user_id, "completion-count").await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_user_coins_earned(&self, user_id: &str) -> Option<i32> {
        let key = self.user_key(user_id, "coins-earned").await;
        self.get_cached(&key).await
    }

    pub async fn set_user_coins_earned(&self, user_id: &str, value: i32) {
        let key = self.user_key(user_id, "coins-earned").await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_user_completions_by_category(
        &self,
        user_id: &str,
    ) -> Option<HashMap<String, i32>> {
        let key = self.user_key(user_id, "completion-categories").await;
        self.get_cached(&key).await
    }

    pub async fn set_user_completions_by_category(
        &self,
        user_id: &str,
        value: HashMap<String, i32>,
    ) {
        let key = self.user_key(user_id, "completion-categories").await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_user_completions_with_challenges(
        &self,
        user_id: &str,
    ) -> Option<Vec<(completion::Model, challenges::Model)>> {
        let key = self.user_key(user_id, "journal").await;
        self.get_cached(&key).await
    }

    pub async fn set_user_completions_with_challenges(
        &self,
        user_id: &str,
        value: Vec<(completion::Model, challenges::Model)>,
    ) {
        let key = self.user_key(user_id, "journal").await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_user_recent_activity(
        &self,
        user_id: &str,
        num_days_back: i64,
    ) -> Option<Vec<NaiveDateTime>> {
        let key = self
            .user_key(user_id, &format!("recent:{num_days_back}"))
            .await;
        self.get_cached(&key).await
    }

    pub async fn set_user_recent_activity(
        &self,
        user_id: &str,
        num_days_back: i64,
        value: Vec<NaiveDateTime>,
    ) {
        let key = self
            .user_key(user_id, &format!("recent:{num_days_back}"))
            .await;
        self.set_cached(&key, &value, USER_TTL_SECONDS).await;
    }

    pub async fn get_rewards(&self) -> Option<Vec<reward::Model>> {
        let key = self.versioned_key("rewards", "all").await;
        self.get_cached(&key).await
    }

    pub async fn set_rewards(&self, value: Vec<reward::Model>) {
        let key = self.versioned_key("rewards", "all").await;
        self.set_cached(&key, &value, REWARD_TTL_SECONDS).await;
    }

    pub async fn get_reward_by_name(&self, name: &str) -> Option<Option<reward::Model>> {
        let key = self.versioned_key("rewards", &format!("name:{name}")).await;
        self.get_cached(&key).await
    }

    pub async fn set_reward_by_name(&self, name: &str, value: Option<reward::Model>) {
        let key = self.versioned_key("rewards", &format!("name:{name}")).await;
        self.set_cached(&key, &value, REWARD_TTL_SECONDS).await;
    }

    pub async fn get_leaderboard_page(
        &self,
        limit: u64,
        cursor: Option<&str>,
    ) -> Option<Vec<LeaderboardEntry>> {
        let suffix = format!("page:{limit}:{}", cursor.unwrap_or("start"));
        let key = self.versioned_key("leaderboard", &suffix).await;
        self.get_cached(&key).await
    }

    pub async fn set_leaderboard_page(
        &self,
        limit: u64,
        cursor: Option<&str>,
        value: Vec<LeaderboardEntry>,
    ) {
        let suffix = format!("page:{limit}:{}", cursor.unwrap_or("start"));
        let key = self.versioned_key("leaderboard", &suffix).await;
        self.set_cached(&key, &value, LEADERBOARD_TTL_SECONDS).await;
    }

    pub async fn get_user_position(&self, user_id: &str) -> Option<i64> {
        let key = self
            .versioned_key("user-position", &format!("position:{user_id}"))
            .await;
        self.get_cached(&key).await
    }

    pub async fn set_user_position(&self, user_id: &str, value: i64) {
        let key = self
            .versioned_key("user-position", &format!("position:{user_id}"))
            .await;
        self.set_cached(&key, &value, USER_POSITION_TTL_SECONDS)
            .await;
    }

    pub async fn invalidate_challenges(&self) {
        self.bump_version("challenges").await;
    }

    pub async fn invalidate_user_data(&self, user_id: &str) {
        self.bump_version(&format!("user:{user_id}")).await;
    }

    pub async fn invalidate_leaderboard(&self) {
        self.bump_version("leaderboard").await;
    }

    pub async fn invalidate_rewards(&self) {
        self.bump_version("rewards").await;
    }

    pub async fn invalidate_all(&self) {
        self.invalidate_challenges().await;
        self.invalidate_leaderboard().await;
        self.invalidate_rewards().await;
        self.bump_version("user-position").await;
        self.bump_version("global").await;
    }
}
