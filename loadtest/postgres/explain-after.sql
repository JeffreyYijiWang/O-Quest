\set ON_ERROR_STOP on
\pset pager off

\echo 'AFTER: single indexed transaction fetch used for the complete rewards page'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT id, user_id, reward_name, count, timestamp, status
FROM "transaction"
WHERE user_id = 'hist-00001';

\echo 'AFTER: database-side coin aggregate'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT COALESCE(SUM(t.count * r.cost), 0)::BIGINT AS total
FROM "transaction" t
JOIN reward r ON r.name = t.reward_name
WHERE t.user_id = 'hist-00001';

\echo 'AFTER: dorm-wide Carnegie Cup aggregate'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT COALESCE(SUM(t.count), 0)::BIGINT AS total
FROM "transaction" t
JOIN "user" u ON u.user_id = t.user_id
WHERE t.reward_name = 'Carnegie Cup Contribution'
  AND u.dorm = 'Mudge';

\echo 'AFTER: one uncached user rank lookup'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH user_stats AS (
    SELECT u.user_id, u.name,
           COALESCE(earned.total_earned, 0)::BIGINT AS coins_earned
    FROM "user" u
    LEFT JOIN (
        SELECT c.user_id, SUM(ch.scotty_coins)::BIGINT AS total_earned
        FROM completion c
        JOIN challenges ch ON c.challenge_name = ch.name
        GROUP BY c.user_id
    ) earned ON u.user_id = earned.user_id
    WHERE u.is_admin = FALSE
), ranked_users AS (
    SELECT user_id, ROW_NUMBER() OVER (
        ORDER BY coins_earned DESC, name ASC, user_id ASC
    )::BIGINT AS rank
    FROM user_stats
)
SELECT rank FROM ranked_users WHERE user_id = 'load-0001';

\echo 'AFTER: snapshot-stable first leaderboard page'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH user_stats AS (
    SELECT u.user_id, u.name, u.dorm, u.is_admin,
           COALESCE(earned.total_earned, 0)::BIGINT AS coins_earned,
           COALESCE(spent.total_spent, 0)::BIGINT AS coins_spent,
           COALESCE(completed.challenge_count, 0)::BIGINT AS challenges_completed
    FROM "user" u
    LEFT JOIN (
        SELECT c.user_id, SUM(ch.scotty_coins)::BIGINT AS total_earned
        FROM completion c JOIN challenges ch ON c.challenge_name = ch.name
        WHERE c.timestamp <= TIMESTAMP '2026-08-10 00:00:00'
        GROUP BY c.user_id
    ) earned ON u.user_id = earned.user_id
    LEFT JOIN (
        SELECT t.user_id, SUM(t.count * r.cost)::BIGINT AS total_spent
        FROM "transaction" t JOIN reward r ON t.reward_name = r.name
        WHERE t.timestamp <= TIMESTAMP '2026-08-10 00:00:00'
        GROUP BY t.user_id
    ) spent ON u.user_id = spent.user_id
    LEFT JOIN (
        SELECT user_id, COUNT(*)::BIGINT AS challenge_count
        FROM completion
        WHERE timestamp <= TIMESTAMP '2026-08-10 00:00:00'
        GROUP BY user_id
    ) completed ON u.user_id = completed.user_id
), ranked_users AS (
    SELECT *, ROW_NUMBER() OVER (
        ORDER BY coins_earned DESC, name ASC, user_id ASC
    )::BIGINT AS rank
    FROM user_stats WHERE is_admin = FALSE
)
SELECT rank, user_id, name, dorm, coins_earned, coins_spent, challenges_completed
FROM ranked_users
ORDER BY coins_earned DESC, name ASC, user_id ASC
LIMIT 20;
