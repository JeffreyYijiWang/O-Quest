\set ON_ERROR_STOP on

INSERT INTO "user" (user_id, name, dorm, is_admin)
VALUES ('pair-same', 'Two-Request Contention User', 'Test Dorm', FALSE);

INSERT INTO completion (user_id, challenge_name, timestamp)
VALUES ('pair-same', 'Stress Balance Grant', TIMESTAMP '2026-08-10 00:00:00');

INSERT INTO reward (name, cost, stock, trade_limit)
VALUES ('Stress Balance Pair', 100, 2, 2);
