# Backend performance and independence changelog

## 2026-08-10 - local backend rebuild

### Independence and security

- Removed runtime dependencies on the retired OIDC and API hosts.
- Added HMAC-SHA-256 signed local sessions with HttpOnly cookies, expiry, logout, origin allowlists, and explicit production secret requirements.
- Restricted synthetic per-user headers to `LOAD_TEST_MODE=true` and debug authentication to debug builds.
- Replaced hard-coded production CORS origins with runtime configuration.
- Retained private MinIO/S3 objects and presigned reads for journal media.

### Query and consistency changes

- Replaced the rewards endpoint's per-reward transaction queries with one per-user query.
- Moved coin and transaction totals into SQL aggregates.
- Added stable leaderboard keyset cursors with deterministic tie-breakers and a bounded snapshot timestamp.
- Added synchronized rank snapshots so individual profile reads do not repeatedly execute the global ranking window.
- Made reward inventory decrement and transaction insertion one atomic database transaction.
- Added eight measured indexes for completion order/lookups, transaction aggregates/status/history, and challenge filtering/unlocks.

### Cache and transport changes

- Added a bounded five-second Moka L1 and a pooled shared Redis L2.
- Added versioned cache families for constant-time invalidation across replicas.
- Added bounded TTLs by volatility: challenges 1 hour, users 5 minutes, rewards 60 seconds, leaderboard pages 15 seconds, and user positions 30 seconds.
- Added cache hits, misses, writes, invalidations, errors, and hit-rate instrumentation at `/metrics/cache`.
- Added gzip for eligible JSON responses of at least 1 KiB.
- Configured bounded PostgreSQL and Redis connection pools with timeouts.

### Validation

- Added deterministic local PostgreSQL/Redis/MinIO infrastructure and separate before/after databases.
- Added query-plan capture and an identical k6 journey scaling to 600 virtual users with realistic think time.
- Added raw JSON summaries, CSV output, SVG charts, and a generated Markdown report under `loadtest/results`.
