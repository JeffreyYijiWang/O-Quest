\set ON_ERROR_STOP on

INSERT INTO "user" (user_id, name, dorm, is_admin)
VALUES ('balance-same', 'Balance Contention User', 'Test Dorm', FALSE);

INSERT INTO completion (user_id, challenge_name, timestamp)
VALUES ('balance-same', 'Stress Balance Grant', TIMESTAMP '2026-08-10 00:00:00');

INSERT INTO reward (name, cost, stock, trade_limit)
VALUES ('Stress Balance 100', 100, 100, 100);
