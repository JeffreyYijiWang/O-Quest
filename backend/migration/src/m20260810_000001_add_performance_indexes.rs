use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let indexes = [
            Index::create()
                .name("idx_completion_timestamp")
                .table(Completion::Table)
                .col(Completion::Timestamp)
                .to_owned(),
            Index::create()
                .name("idx_completion_challenge")
                .table(Completion::Table)
                .col(Completion::ChallengeName)
                .to_owned(),
            Index::create()
                .name("idx_transaction_user_reward")
                .table(Transaction::Table)
                .col(Transaction::UserId)
                .col(Transaction::RewardName)
                .to_owned(),
            Index::create()
                .name("idx_transaction_user_status")
                .table(Transaction::Table)
                .col(Transaction::UserId)
                .col(Transaction::Status)
                .to_owned(),
            Index::create()
                .name("idx_transaction_reward_user")
                .table(Transaction::Table)
                .col(Transaction::RewardName)
                .col(Transaction::UserId)
                .to_owned(),
            Index::create()
                .name("idx_transaction_timestamp")
                .table(Transaction::Table)
                .col(Transaction::Timestamp)
                .to_owned(),
            Index::create()
                .name("idx_challenges_category")
                .table(Challenges::Table)
                .col(Challenges::Category)
                .to_owned(),
            Index::create()
                .name("idx_challenges_unlock_timestamp")
                .table(Challenges::Table)
                .col(Challenges::UnlockTimestamp)
                .to_owned(),
        ];

        for index in indexes {
            manager.create_index(index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "idx_challenges_unlock_timestamp",
            "idx_challenges_category",
            "idx_transaction_timestamp",
            "idx_transaction_reward_user",
            "idx_transaction_user_status",
            "idx_transaction_user_reward",
            "idx_completion_challenge",
            "idx_completion_timestamp",
        ] {
            manager
                .drop_index(Index::drop().name(name).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Completion {
    Table,
    ChallengeName,
    Timestamp,
}

#[derive(DeriveIden)]
enum Transaction {
    Table,
    UserId,
    RewardName,
    Timestamp,
    Status,
}

#[derive(DeriveIden)]
enum Challenges {
    Table,
    Category,
    UnlockTimestamp,
}
