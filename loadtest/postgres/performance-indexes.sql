\set ON_ERROR_STOP on

CREATE INDEX idx_completion_timestamp
    ON completion (timestamp);
CREATE INDEX idx_completion_challenge
    ON completion (challenge_name);
CREATE INDEX idx_transaction_user_reward
    ON "transaction" (user_id, reward_name);
CREATE INDEX idx_transaction_user_status
    ON "transaction" (user_id, status);
CREATE INDEX idx_transaction_reward_user
    ON "transaction" (reward_name, user_id);
CREATE INDEX idx_transaction_timestamp
    ON "transaction" (timestamp);
CREATE INDEX idx_challenges_category
    ON challenges (category);
CREATE INDEX idx_challenges_unlock_timestamp
    ON challenges (unlock_timestamp);

ANALYZE;
