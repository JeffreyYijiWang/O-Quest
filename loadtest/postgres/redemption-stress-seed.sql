\set ON_ERROR_STOP on

INSERT INTO challenges (
    name, category, location, scotty_coins, maps_link, tagline,
    description, more_info_link, unlock_timestamp, secret,
    latitude, longitude, location_accuracy
) VALUES (
    'Stress Balance Grant', 'Test', 'Local', 100, NULL,
    'Concurrency-test coin grant', 'Local contention fixture', NULL,
    TIMESTAMP '2026-01-01 00:00:00', 'stress-secret', NULL, NULL, NULL
);

INSERT INTO "user" (user_id, name, dorm, is_admin)
VALUES ('stress-same', 'Shared Contention User', 'Test Dorm', FALSE);

INSERT INTO "user" (user_id, name, dorm, is_admin)
SELECT
    'stress-' || lpad(n::text, 5, '0'),
    'Stress User ' || lpad(n::text, 5, '0'),
    'Test Dorm',
    FALSE
FROM generate_series(1, 5000) AS n;

INSERT INTO completion (user_id, challenge_name, timestamp, s3_link, note)
SELECT user_id, 'Stress Balance Grant', TIMESTAMP '2026-08-10 00:00:00', NULL, NULL
FROM "user"
WHERE user_id = 'stress-same' OR user_id LIKE 'stress-%';
