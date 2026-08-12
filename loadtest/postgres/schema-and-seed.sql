\set ON_ERROR_STOP on

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "user" (
    user_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    dorm TEXT,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE reward (
    name TEXT PRIMARY KEY,
    cost INTEGER NOT NULL,
    stock INTEGER NOT NULL,
    trade_limit INTEGER NOT NULL
);

CREATE TABLE challenges (
    name TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    location TEXT NOT NULL,
    scotty_coins INTEGER NOT NULL,
    maps_link TEXT,
    tagline TEXT NOT NULL,
    description TEXT NOT NULL,
    more_info_link TEXT,
    unlock_timestamp TIMESTAMP NOT NULL,
    secret TEXT NOT NULL,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    location_accuracy NUMERIC(8, 2)
);

CREATE TABLE completion (
    user_id TEXT NOT NULL REFERENCES "user"(user_id) ON DELETE CASCADE,
    challenge_name TEXT NOT NULL REFERENCES challenges(name) ON DELETE CASCADE,
    timestamp TIMESTAMP NOT NULL,
    s3_link TEXT,
    note TEXT,
    PRIMARY KEY (user_id, challenge_name)
);

CREATE TABLE "transaction" (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES "user"(user_id) ON DELETE CASCADE,
    reward_name TEXT NOT NULL REFERENCES reward(name) ON DELETE CASCADE,
    count INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending'
);

INSERT INTO reward (name, cost, stock, trade_limit) VALUES
    ('Carnegie Cup Contribution', 1, -1, 10000),
    ('Sticker Pack', 10, 100000, 5),
    ('Quest Pin', 20, 100000, 3),
    ('Water Bottle', 50, 100000, 2),
    ('T-Shirt', 75, 100000, 1),
    ('Crewneck', 120, 100000, 1),
    ('Backpack', 150, 100000, 1),
    ('Poster', 15, 100000, 3),
    ('Hat', 65, 100000, 1),
    ('Mug', 35, 100000, 2),
    ('Keychain', 12, 100000, 4),
    ('Mystery Box', 90, 100000, 1);

INSERT INTO challenges (
    name, category, location, scotty_coins, maps_link, tagline,
    description, more_info_link, unlock_timestamp, secret,
    latitude, longitude, location_accuracy
)
SELECT
    'Challenge ' || lpad(n::text, 3, '0'),
    (ARRAY['Explore', 'Create', 'Connect', 'Learn', 'Wellness'])[((n - 1) % 5) + 1],
    'Campus Zone ' || (((n - 1) % 12) + 1),
    5 + ((n - 1) % 8) * 5,
    NULL,
    'Complete challenge ' || n,
    'Deterministic benchmark challenge ' || n,
    NULL,
    TIMESTAMP '2026-01-01 00:00:00',
    'secret-' || n,
    NULL,
    NULL,
    NULL
FROM generate_series(1, 120) AS n;

INSERT INTO "user" (user_id, name, dorm, is_admin)
SELECT
    'hist-' || lpad(n::text, 5, '0'),
    'Historical User ' || lpad(n::text, 5, '0'),
    (ARRAY['Mudge', 'Stever', 'Donner', 'The Hill', 'Morewood Gardens'])[((n - 1) % 5) + 1],
    FALSE
FROM generate_series(1, 5000) AS n;

INSERT INTO completion (user_id, challenge_name, timestamp, s3_link, note)
SELECT
    'hist-' || lpad(u::text, 5, '0'),
    'Challenge ' || lpad((((u + slot * 7) % 120) + 1)::text, 3, '0'),
    TIMESTAMP '2026-08-01 12:00:00' - ((u + slot) % 45) * INTERVAL '1 day',
    NULL,
    NULL
FROM generate_series(1, 5000) AS u
CROSS JOIN generate_series(1, 30) AS slot;

INSERT INTO "transaction" (id, user_id, reward_name, count, timestamp, status)
SELECT
    gen_random_uuid(),
    'hist-' || lpad(u::text, 5, '0'),
    (ARRAY['Sticker Pack', 'Quest Pin', 'Water Bottle', 'T-Shirt', 'Poster', 'Hat', 'Mug', 'Keychain'])[((slot - 1) % 8) + 1],
    1 + ((u + slot) % 2),
    TIMESTAMP '2026-08-01 12:00:00' - ((u + slot) % 30) * INTERVAL '1 day',
    CASE WHEN (u + slot) % 3 = 0 THEN 'complete' ELSE 'pending' END
FROM generate_series(1, 5000) AS u
CROSS JOIN generate_series(1, 8) AS slot;

ANALYZE;
