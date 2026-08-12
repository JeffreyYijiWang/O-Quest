\set ON_ERROR_STOP on
\pset pager off

\echo 'BEFORE: one of the 12 per-reward transaction lookups'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT id, user_id, reward_name, count, timestamp, status
FROM "transaction"
WHERE user_id = 'hist-00001' AND reward_name = 'Sticker Pack';

\echo 'BEFORE: materialized coin rows returned to the application'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT r.cost * t.count AS total_cost
FROM "transaction" t
JOIN reward r ON t.reward_name = r.name
WHERE t.user_id = 'hist-00001';

\echo 'BEFORE: first leaderboard page'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH user_stats AS (
    SELECT u.user_id, u.name, u.dorm, u.is_admin,
           COALESCE(earned.total_earned, 0) AS coins_earned,
           COALESCE(spent.total_spent, 0) AS coins_spent,
           COALESCE(completed.challenge_count, 0) AS challenges_completed
    FROM "user" u
    LEFT JOIN (
        SELECT c.user_id, SUM(ch.scotty_coins) AS total_earned
        FROM completion c JOIN challenges ch ON c.challenge_name = ch.name
        GROUP BY c.user_id
    ) earned ON u.user_id = earned.user_id
    LEFT JOIN (
        SELECT t.user_id, SUM(t.count * r.cost) AS total_spent
        FROM "transaction" t JOIN reward r ON t.reward_name = r.name
        GROUP BY t.user_id
    ) spent ON u.user_id = spent.user_id
    LEFT JOIN (
        SELECT user_id, COUNT(*) AS challenge_count
        FROM completion GROUP BY user_id
    ) completed ON u.user_id = completed.user_id
), ranked_users AS (
    SELECT *, ROW_NUMBER() OVER (ORDER BY COALESCE(coins_earned, 0) DESC, name ASC) AS rank
    FROM user_stats
)
SELECT * FROM ranked_users WHERE is_admin = FALSE ORDER BY rank LIMIT 20;
