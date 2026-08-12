use crate::services::traits::LeaderboardServiceTrait;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, FromQueryResult, Serialize, Deserialize, ToSchema, Clone)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub name: String,
    pub dorm: Option<String>,
    pub coins_earned: i64,
    pub coins_spent: i64,
    pub challenges_completed: i64,
}

#[derive(Debug, FromQueryResult)]
struct LeaderboardPosition {
    user_id: String,
    rank: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardCursor {
    pub as_of: NaiveDateTime,
    pub coins_earned: Option<i64>,
    pub name: Option<String>,
    pub user_id: Option<String>,
}

impl LeaderboardCursor {
    pub fn start() -> Self {
        const SNAPSHOT_SECONDS: i64 = 15;
        let timestamp = Utc::now().timestamp();
        let bucket = timestamp - timestamp.rem_euclid(SNAPSHOT_SECONDS);
        Self {
            as_of: chrono::DateTime::<Utc>::from_timestamp(bucket, 0)
                .expect("valid leaderboard snapshot timestamp")
                .naive_utc(),
            coins_earned: None,
            name: None,
            user_id: None,
        }
    }

    pub fn after(&self, entry: &LeaderboardEntry) -> Self {
        Self {
            as_of: self.as_of,
            coins_earned: Some(entry.coins_earned),
            name: Some(entry.name.clone()),
            user_id: Some(entry.user_id.clone()),
        }
    }

    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_vec(self).map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn decode(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = URL_SAFE_NO_PAD.decode(value)?;
        let cursor: Self = serde_json::from_slice(&bytes)?;
        if cursor.coins_earned.is_some() != (cursor.name.is_some() && cursor.user_id.is_some()) {
            return Err("incomplete leaderboard cursor".into());
        }
        Ok(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip_preserves_snapshot_and_tie_breakers() {
        let cursor = LeaderboardCursor {
            as_of: NaiveDateTime::parse_from_str("2026-08-10 12:34:56", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            coins_earned: Some(125),
            name: Some("Alex".to_string()),
            user_id: Some("alex-2".to_string()),
        };

        let decoded = LeaderboardCursor::decode(&cursor.encode().unwrap()).unwrap();
        assert_eq!(decoded.as_of, cursor.as_of);
        assert_eq!(decoded.coins_earned, cursor.coins_earned);
        assert_eq!(decoded.name, cursor.name);
        assert_eq!(decoded.user_id, cursor.user_id);
    }

    #[test]
    fn cursor_rejects_incomplete_tie_breaker() {
        let malformed = LeaderboardCursor {
            as_of: Utc::now().naive_utc(),
            coins_earned: Some(100),
            name: None,
            user_id: None,
        };
        assert!(LeaderboardCursor::decode(&malformed.encode().unwrap()).is_err());
    }
}

#[derive(Clone)]
pub struct LeaderboardService {
    db: DatabaseConnection,
}

impl LeaderboardService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LeaderboardServiceTrait for LeaderboardService {
    async fn get_leaderboard_page(
        &self,
        limit: u64,
        cursor: Option<&LeaderboardCursor>,
    ) -> Result<Vec<LeaderboardEntry>, sea_orm::DbErr> {
        let cursor = cursor.cloned().unwrap_or_else(LeaderboardCursor::start);
        let query = r#"
            WITH user_stats AS (
                SELECT
                    u.user_id,
                    u.name,
                    u.dorm,
                    u.is_admin,
                    COALESCE(earned.total_earned, 0)::BIGINT AS coins_earned,
                    COALESCE(spent.total_spent, 0)::BIGINT AS coins_spent,
                    COALESCE(completed.challenge_count, 0)::BIGINT AS challenges_completed
                FROM "user" u
                LEFT JOIN (
                    SELECT c.user_id, SUM(ch.scotty_coins)::BIGINT AS total_earned
                    FROM completion c
                    JOIN challenges ch ON c.challenge_name = ch.name
                    WHERE c.timestamp <= $1
                    GROUP BY c.user_id
                ) earned ON u.user_id = earned.user_id
                LEFT JOIN (
                    SELECT t.user_id, SUM(t.count * r.cost)::BIGINT AS total_spent
                    FROM "transaction" t
                    JOIN reward r ON t.reward_name = r.name
                    WHERE t.timestamp <= $1
                    GROUP BY t.user_id
                ) spent ON u.user_id = spent.user_id
                LEFT JOIN (
                    SELECT user_id, COUNT(*)::BIGINT AS challenge_count
                    FROM completion
                    WHERE timestamp <= $1
                    GROUP BY user_id
                ) completed ON u.user_id = completed.user_id
            ),
            ranked_users AS (
                SELECT
                    *,
                    ROW_NUMBER() OVER (
                        ORDER BY coins_earned DESC, name ASC, user_id ASC
                    )::BIGINT AS rank
                FROM user_stats
                WHERE is_admin = false
            )
            SELECT rank, user_id, name, dorm, coins_earned, coins_spent, challenges_completed
            FROM ranked_users
            WHERE
                $2::BIGINT IS NULL
                OR coins_earned < $2
                OR (
                    coins_earned = $2
                    AND (
                        name > $3
                        OR (name = $3 AND user_id > $4)
                    )
                )
            ORDER BY coins_earned DESC, name ASC, user_id ASC
            LIMIT $5
        "#;

        LeaderboardEntry::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            query,
            vec![
                cursor.as_of.into(),
                cursor.coins_earned.into(),
                cursor.name.into(),
                cursor.user_id.into(),
                (limit as i64).into(),
            ],
        ))
        .all(&self.db)
        .await
    }

    async fn get_user_leaderboard_position(&self, user_id: &str) -> Result<i64, sea_orm::DbErr> {
        let query = r#"
            WITH user_stats AS (
                SELECT
                    u.user_id,
                    u.name,
                    COALESCE(earned.total_earned, 0)::BIGINT AS coins_earned
                FROM "user" u
                LEFT JOIN (
                    SELECT c.user_id, SUM(ch.scotty_coins)::BIGINT AS total_earned
                    FROM completion c
                    JOIN challenges ch ON c.challenge_name = ch.name
                    GROUP BY c.user_id
                ) earned ON u.user_id = earned.user_id
                WHERE u.is_admin = false
            ),
            ranked_users AS (
                SELECT
                    user_id,
                    ROW_NUMBER() OVER (
                        ORDER BY coins_earned DESC, name ASC, user_id ASC
                    )::BIGINT AS rank
                FROM user_stats
            )
            SELECT rank FROM ranked_users WHERE user_id = $1
        "#;

        let rank = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                query,
                vec![user_id.into()],
            ))
            .await?
            .ok_or_else(|| sea_orm::DbErr::Custom("User not found".to_string()))?
            .try_get_by_index(0)?;

        Ok(rank)
    }

    async fn get_all_user_leaderboard_positions(
        &self,
    ) -> Result<HashMap<String, i64>, sea_orm::DbErr> {
        let query = r#"
            WITH user_stats AS (
                SELECT
                    u.user_id,
                    u.name,
                    COALESCE(earned.total_earned, 0)::BIGINT AS coins_earned
                FROM "user" u
                LEFT JOIN (
                    SELECT c.user_id, SUM(ch.scotty_coins)::BIGINT AS total_earned
                    FROM completion c
                    JOIN challenges ch ON c.challenge_name = ch.name
                    GROUP BY c.user_id
                ) earned ON u.user_id = earned.user_id
                WHERE u.is_admin = false
            )
            SELECT
                user_id,
                ROW_NUMBER() OVER (
                    ORDER BY coins_earned DESC, name ASC, user_id ASC
                )::BIGINT AS rank
            FROM user_stats
        "#;

        let positions = LeaderboardPosition::find_by_statement(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        ))
        .all(&self.db)
        .await?;

        Ok(positions
            .into_iter()
            .map(|position| (position.user_id, position.rank))
            .collect())
    }
}
