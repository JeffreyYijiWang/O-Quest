use std::collections::HashMap;

use crate::entities::{prelude::*, transaction, user};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QuerySelect, Statement, TransactionTrait,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct TransactionService {
    db: DatabaseConnection,
}

impl TransactionService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_ccup_total_purchased(&self, user_dorm: &str) -> Result<i64, sea_orm::DbErr> {
        let total = Transaction::find()
            .inner_join(User)
            .filter(transaction::Column::RewardName.eq("Carnegie Cup Contribution"))
            .filter(user::Column::Dorm.eq(user_dorm))
            .select_only()
            .column_as(transaction::Column::Count.sum(), "total")
            .into_tuple::<Option<i64>>()
            .one(&self.db)
            .await?;

        Ok(total.flatten().unwrap_or(0))
    }

    pub async fn create_transaction(
        &self,
        user_id: &str,
        reward_name: &str,
        count: i32,
    ) -> Result<transaction::Model, sea_orm::DbErr> {
        let database_transaction = self.db.begin().await?;

        let stock_updated = database_transaction
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"
                UPDATE reward
                SET stock = CASE WHEN stock = -1 THEN -1 ELSE stock - $1 END
                WHERE name = $2 AND (stock = -1 OR stock >= $1)
                RETURNING name
                "#,
                vec![count.into(), reward_name.into()],
            ))
            .await?;

        if stock_updated.is_none() {
            database_transaction.rollback().await?;
            return Err(sea_orm::DbErr::Custom(
                "Reward not found or insufficient stock".to_string(),
            ));
        }

        let transaction_id = Uuid::new_v4();

        let new_transaction = transaction::ActiveModel {
            id: Set(transaction_id),
            user_id: Set(user_id.to_string()),
            reward_name: Set(reward_name.to_string()),
            timestamp: Set(Utc::now().naive_utc()),
            status: Set("pending".to_string()),
            count: Set(count),
        };
        let created = new_transaction.insert(&database_transaction).await?;
        database_transaction.commit().await?;
        Ok(created)
    }

    // Get total counts for a user across all transactions (for trade limits)
    pub async fn get_user_total_counts(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        let totals = Transaction::find()
            .filter(transaction::Column::UserId.eq(user_id))
            .select_only()
            .column(transaction::Column::RewardName)
            .column_as(transaction::Column::Count.sum(), "total")
            .group_by(transaction::Column::RewardName)
            .into_tuple::<(String, i64)>()
            .all(&self.db)
            .await?;

        Ok(totals
            .into_iter()
            .map(|(name, count)| (name, count as i32))
            .collect::<HashMap<_, _>>())
    }

    // Get total coins spent by user
    pub async fn get_user_total_coins_spent(&self, user_id: &str) -> Result<i32, sea_orm::DbErr> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"
                SELECT COALESCE(SUM(t.count * r.cost), 0)::BIGINT AS total
                FROM "transaction" t
                JOIN reward r ON r.name = t.reward_name
                WHERE t.user_id = $1
                "#,
                vec![user_id.into()],
            ))
            .await?
            .ok_or_else(|| sea_orm::DbErr::Custom("Missing aggregate row".to_string()))?;
        let total: i64 = row.try_get("", "total")?;
        i32::try_from(total).map_err(|_| sea_orm::DbErr::Custom("Coin total overflow".to_string()))
    }

    pub async fn get_user_transactions(
        &self,
        user_id: &str,
    ) -> Result<Vec<transaction::Model>, sea_orm::DbErr> {
        Transaction::find()
            .filter(transaction::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
    }

    // Get user transactions for a specific reward (for rewards page)
    pub async fn get_user_reward_transactions(
        &self,
        user_id: &str,
        reward_name: &str,
    ) -> Result<Vec<transaction::Model>, sea_orm::DbErr> {
        Transaction::find()
            .filter(transaction::Column::UserId.eq(user_id))
            .filter(transaction::Column::RewardName.eq(reward_name))
            .all(&self.db)
            .await
    }

    // Delete a transaction (for cancellation)
    pub async fn delete_transaction(&self, transaction_id: &str) -> Result<bool, sea_orm::DbErr> {
        let uuid = Uuid::parse_str(transaction_id)
            .map_err(|_| sea_orm::DbErr::Custom("Invalid UUID".to_string()))?;

        let result = Transaction::delete_many()
            .filter(crate::entities::transaction::Column::Id.eq(uuid))
            .exec(&self.db)
            .await?;

        Ok(result.rows_affected > 0)
    }

    // Get transaction by ID (for admin verification)
    pub async fn get_transaction_by_id(
        &self,
        transaction_id: &str,
    ) -> Result<Option<transaction::Model>, sea_orm::DbErr> {
        let uuid = Uuid::parse_str(transaction_id)
            .map_err(|_| sea_orm::DbErr::Custom("Invalid UUID".to_string()))?;

        Transaction::find()
            .filter(transaction::Column::Id.eq(uuid))
            .one(&self.db)
            .await
    }

    // Update transaction status (for admin verification)
    pub async fn update_transaction_status(
        &self,
        transaction_id: &str,
        status: &str,
    ) -> Result<Option<transaction::Model>, sea_orm::DbErr> {
        let Some(transaction) = self.get_transaction_by_id(transaction_id).await? else {
            return Ok(None);
        };

        let mut active_transaction: transaction::ActiveModel = transaction.into();
        active_transaction.status = Set(status.to_string());

        active_transaction.update(&self.db).await.map(Some)
    }
}
