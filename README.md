# O-Quest

O-Quest is a mobile-first campus exploration game for first-year orientation. Students discover campus locations, scan challenge posters, verify their position, earn Scotty Coins, keep a photo journal, compare progress, and redeem rewards. Staff place challenge posters and verify reward pickups.

This repository now runs as a self-contained application. It does **not** connect to the retired O-Quest API or identity servers. The replacement backend owns authentication, querying, caching, durable user data, and media storage through locally configurable services.

## What is in this repository

| Component | Purpose | Main technologies |
| --- | --- | --- |
| `frontend` | Shared student/staff UI for web, PWA, iOS, and Android | React 19, TypeScript, Vite, TanStack Router/Query, Tauri 2 |
| `backend` | Authentication, business rules, query API, cache coordination, and storage integration | Rust, Axum, SeaORM, PostgreSQL, Moka, Redis |
| `qr-code-gen` | Staff tool for branded, printable challenge posters | React, Vite, QR rendering |
| `loadtest` | Isolated local infrastructure and reproducible before/after performance test | Docker Compose, PostgreSQL, Redis, MinIO, k6 |

## Product functionality

| Area | Purpose | Behavior |
| --- | --- | --- |
| Session and onboarding | Associate activity with a participant | Creates a signed local session, creates a user record on first use, and records a housing community. |
| Challenges | Guide campus exploration | Shows locked, available, and completed activities with categories, search, descriptions, maps, and coin values. |
| Completion | Verify an on-site visit | Combines a challenge-specific QR value with device location and accepts an optional note and photo. |
| Profile | Summarize individual progress | Shows earned and available coins, rank, completions, categories, recent activity, dorm, and journal photos. |
| Journal | Preserve orientation memories | Stores completion notes and private photos and supports note updates and photo deletion. |
| Leaderboard | Make progress visible | Ranks non-admin participants and exposes stable, cursor-paginated pages for infinite scrolling. |
| Terrier Trade | Exchange coins for rewards | Enforces balance, inventory, and per-user limits and creates a QR-backed pending redemption. |
| Carnegie Cup | Compare housing communities | Aggregates participant contributions by dorm. |
| Staff challenge setup | Register physical posters | Captures poster latitude, longitude, and location accuracy. |
| Staff reward verification | Finish the redemption lifecycle | Scans a transaction QR and marks the pending pickup complete. |
| Poster generator | Produce physical materials | Generates category-branded, letter-size QR posters for individual or batch printing. |

## Architecture

```mermaid
flowchart LR
    subgraph Clients["Client surfaces"]
        Web["React web / PWA"]
        Mobile["Tauri iOS / Android"]
        Poster["QR poster generator"]
    end

    subgraph API["Rust / Axum backend"]
        Router["Routes, signed-session auth, CORS, gzip"]
        Handlers["Handlers and business workflows"]
        Services["Service traits and SeaORM implementations"]
        L1["Moka L1 cache<br/>5-second TTL"]
        Storage["Storage service"]
    end

    L2[("Redis L2 cache")]
    DB[("PostgreSQL<br/>source of truth")]
    Objects[("MinIO / S3<br/>private media")]

    Web -->|"JSON over HTTP + HttpOnly cookie"| Router
    Mobile -->|"JSON over HTTP + HttpOnly cookie"| Router
    Router --> Handlers --> Services
    Services <--> L1
    L1 <--> L2
    Services --> DB
    Storage --> Objects
    Handlers --> Storage
    Poster -. "printed challenge QR" .-> Mobile
```

The backend is stateless except for its bounded L1 cache and connection pools. PostgreSQL remains authoritative, Redis provides a shared cache and invalidation versions across API replicas, and MinIO/S3 stores binary media outside database rows. A failed or empty cache is a performance degradation, not a data-loss event.

## Repository layout

```text
.
|-- backend/
|   |-- migration/          # Schema and workload-specific indexes
|   `-- src/
|       |-- auth.rs         # Signed local sessions and auth middleware
|       |-- cache/          # Moka L1, Redis protocol pool, cached decorators
|       |-- entities/       # SeaORM models and relations
|       |-- handlers/       # HTTP orchestration and invalidation boundaries
|       |-- middleware/     # Admin authorization and gzip compression
|       `-- services/       # PostgreSQL queries, ranking, transactions, storage
|-- frontend/
|   |-- src/components/     # Feature and reusable UI components
|   |-- src/lib/            # Runtime API client, auth, hooks, generated schema
|   |-- src/routes/         # TanStack Router routes
|   `-- src-tauri/          # Native shell and mobile projects
|-- loadtest/               # Local services, deterministic seed, k6, reports
`-- qr-code-gen/            # Printable poster application
```

## Mobile application design

### One client across platforms

React and TypeScript implement the application once. Vite builds the web/PWA bundle, and Tauri packages that bundle for iOS and Android. Browser camera and geolocation APIs keep the core challenge flow portable. TanStack Router owns navigation and protected-route context; TanStack Query owns remote server state.

The API URL is runtime configuration, not a retired hard-coded domain:

- `VITE_API_BASE_URL` selects an explicit backend URL when needed.
- Browser deployments default to the current origin.
- Local web/Tauri development falls back to `http://localhost:3000`.

All authenticated requests include credentials so the `quest_session` cookie works consistently across navigation and query refreshes.

### Client state and server state

- React component state contains temporary UI state: open panels, filters, completion steps, a captured image, or a draft note.
- TanStack Query contains server state: profile, challenges, rewards, journal entries, and leaderboard pages.
- TanStack Router contains route state, parameters, redirects, scroll restoration, and view transitions.
- PostgreSQL contains durable game and user state.
- MinIO/S3 contains private photo bytes; PostgreSQL stores only their object keys.

The global client cache avoids duplicate requests during mobile navigation. Mutations invalidate the affected profile, challenge, reward, journal, or leaderboard query so the backend remains the reconciliation source.

### Challenge completion flow

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Location: Start
    Location --> QR: Best position acquired
    Location --> Idle: Permission or position failure
    QR --> Commemorate: Challenge QR decoded
    QR --> Idle: Cancel or timeout
    Commemorate --> Submitting: Add photo/note or skip
    Submitting --> Idle: Completion accepted
    Submitting --> Commemorate: Validation or API error
```

The device retains the most accurate location reading during the sampling window. The client scans frames for a QR payload and sends the payload, coordinates, reported accuracy, optional note, and optional photo. The backend loads the challenge, checks its unlock time and secret, rejects a duplicate, validates the location circles with the Haversine distance, stores optional media, inserts the completion, and bumps only that user's cache version.

### Typed API boundary

Rust route definitions generate `/openapi.json`. `frontend/src/lib/schema.gen.ts` supplies TypeScript request and response types to `openapi-fetch` and `openapi-react-query`. Contract changes therefore surface during the frontend build instead of becoming production parsing errors.

## Backend design

The API follows a handler/service/decorator structure:

1. Axum middleware validates a signed session, restricts admin routes, applies CORS, and compresses eligible JSON responses.
2. Handlers parse input, coordinate independent work, map service errors to HTTP responses, and invalidate the smallest affected cache family after a write.
3. Service traits define operations independently of HTTP.
4. SeaORM service implementations execute set-based PostgreSQL queries and transactions.
5. Cached decorators perform L1/L2 read-through caching without moving data ownership out of PostgreSQL.
6. `StorageService` isolates upload, presigned-read URL, and deletion behavior.

`AppState` holds cloneable service handles, one shared cache manager, and database/object-storage connection pools. This keeps request handlers cheap to clone and permits horizontal API scaling.

### Authentication and security boundaries

The retired external identity provider is no longer called. The replacement auth layer issues an HMAC-SHA-256 signed session after `POST /api/auth/session` or the local compatibility login route. Production self-registration always assigns an unguessable server-generated user ID; requested IDs are honored only in debug or isolated load-test mode. Clients receive an HttpOnly, SameSite cookie; `Secure` is enabled with `SESSION_COOKIE_SECURE=true`. Session lifetime defaults to 12 hours.

Production requirements:

- Set a random `SESSION_SECRET` of at least 32 bytes and keep it outside source control.
- Set `SESSION_COOKIE_SECURE=true` behind HTTPS.
- Use `SESSION_COOKIE_SAME_SITE=None` together with `SESSION_COOKIE_SECURE=true` only when an owned native/web origin must call a different HTTPS API site; otherwise keep `Lax`.
- Set `ALLOW_SELF_REGISTRATION` according to the deployment's enrollment policy. Leave it off when enrollment is controlled by an owned account-provisioning layer.
- Restrict `CORS_ALLOWED_ORIGINS` and `ALLOWED_REDIRECT_ORIGINS` to owned clients.
- Never enable `LOAD_TEST_MODE`; its synthetic identity headers exist only for the isolated benchmark.
- `ALLOW_DEV_AUTH` is honored only in debug builds.

Admin handlers still require the database `is_admin` flag or an admin group in the signed claims. Challenge secrets are absent from student responses. Photo buckets remain private; clients receive time-limited signed URLs instead of storage credentials.

## Data ownership and storage

```mermaid
erDiagram
    USER ||--o{ COMPLETION : completes
    USER ||--o{ TRANSACTION : creates
    CHALLENGE ||--o{ COMPLETION : records
    REWARD ||--o{ TRANSACTION : purchases

    USER {
        text user_id PK
        text name
        text dorm
        boolean is_admin
    }
    CHALLENGE {
        text name PK
        text category
        integer scotty_coins
        timestamp unlock_timestamp
        text secret
        double latitude
        double longitude
        decimal location_accuracy
    }
    COMPLETION {
        text user_id PK, FK
        text challenge_name PK, FK
        timestamp timestamp
        text s3_link
        text note
    }
    REWARD {
        text name PK
        integer cost
        integer stock
        integer trade_limit
    }
    TRANSACTION {
        uuid id PK
        text user_id FK
        text reward_name FK
        integer count
        timestamp timestamp
        text status
    }
```

| Data | Durable location | Why |
| --- | --- | --- |
| User ID, display name, dorm, admin flag | PostgreSQL `user` | Strong relational consistency and a stable application-owned identity. |
| Challenge definitions, secrets, unlocks, poster coordinates | PostgreSQL `challenges` | Consistent validation, filtering, and staff updates. |
| Completion time, note, media key | PostgreSQL `completion` | Composite key enforces one completion per user/challenge. |
| Reward catalog and stock | PostgreSQL `reward` | Supports atomic inventory checks and updates. |
| Redemption lifecycle | PostgreSQL `transaction` | UUID is the pickup QR payload; status records pending/completed/cancelled state. |
| Journal photo bytes | Private MinIO/S3 bucket | Scales large binary objects independently from relational queries. |
| Hot serialized read models | Moka and Redis | Disposable acceleration; never the source of truth. |

Photo object keys use `completions/{user_id}/{challenge_name}/{uuid}.jpg`. The UUID prevents collisions and the prefixes keep inspection and lifecycle rules practical. Photo deletion clears the database reference and attempts object cleanup; operators can reconcile orphaned objects from the durable references.

### User-data lifecycle

```mermaid
sequenceDiagram
    participant App as Mobile app
    participant API as Axum API
    participant Cache as Moka / Redis
    participant DB as PostgreSQL
    participant S3 as MinIO / S3

    App->>API: Authenticated read
    API->>Cache: Versioned key lookup
    alt Cache hit
        Cache-->>API: Serialized read model
    else Cache miss
        API->>DB: Set-based query
        DB-->>API: Authoritative rows/aggregate
        API->>Cache: Fill with bounded TTL
    end
    API-->>App: Typed JSON (gzip when useful)

    App->>API: Completion/reward mutation
    API->>DB: Atomic transaction
    opt Photo is present
        API->>S3: Store private object
    end
    API->>Cache: Bump affected version(s)
    API-->>App: Mutation result
```

## Query optimization

The rebuilt paths keep database work bounded as history and concurrency grow:

- **Rewards N+1 removed:** the rewards response loads all of one user's transactions once and groups them in memory, rather than running one query per reward.
- **Aggregates stay in SQL:** coin balances, transaction totals, completion counts, categories, and dorm totals use `SUM`, `COUNT`, and `GROUP BY` instead of transferring historical rows to Rust.
- **Atomic inventory:** reward creation conditionally decrements stock and inserts the transaction inside one database transaction, preventing overselling and orphaned redemptions.
- **Concurrent independent reads:** profile and challenge assembly use `tokio::try_join!` where queries do not depend on one another.
- **Stable keyset pagination:** leaderboard cursors contain the snapshot time and `(coins, name, user_id)` tie-breakers. Pages do not pay the offset cost or drift while users scroll.
- **Short synchronized leaderboard snapshots:** the expensive rank view is shared for a 15-second snapshot bucket; user positions use the same ordering and refresh boundary.
- **Connection-pool bounds:** the API defaults to 64 maximum and 8 minimum PostgreSQL connections, with acquisition, idle, and lifetime limits.
- **Response compression:** JSON bodies at least 1 KiB are gzip-compressed when the client advertises support.

The performance migration adds indexes for the measured access patterns:

```text
completion(timestamp)
completion(challenge_name)
transaction(user_id, reward_name)
transaction(user_id, status)
transaction(reward_name, user_id)
transaction(timestamp)
challenges(category)
challenges(unlock_timestamp)
```

`loadtest/results/query-plans-before.txt` and `query-plans-after.txt` preserve `EXPLAIN (ANALYZE, BUFFERS)` evidence against the deterministic large dataset. Indexes are intentionally workload-specific because every index also increases write cost and storage.

## Caching and invalidation

```mermaid
flowchart LR
    Request["Read request"] --> L1{"Moka L1 hit?<br/>5 s"}
    L1 -->|yes| Response["Return read model"]
    L1 -->|no| L2{"Redis L2 hit?"}
    L2 -->|yes| Fill["Fill L1"] --> Response
    L2 -->|no| Query["Query PostgreSQL"] --> Save["Fill Redis + L1"] --> Response
    Mutation["Committed mutation"] --> Version["INCR cache-family version"]
    Version --> NewKey["Future reads use new key namespace"]
```

Moka absorbs repeated reads inside one API process. Redis shares hot results and invalidation versions between replicas. Versioned keys make invalidation an atomic counter increment instead of a slow wildcard deletion. A one-second local version cache avoids a Redis round trip on every L1 lookup.

| Cache family | Redis TTL | Invalidation/staleness boundary |
| --- | ---: | --- |
| Challenge catalog, item, and counts | 1 hour | Admin challenge change bumps `challenges`. |
| Per-user completions, coins, categories, journal, activity | 5 minutes | That user's completion, journal, dorm, or transaction write bumps `user:{id}`. |
| Reward catalog and items | 60 seconds | Inventory or reward write bumps `rewards`. |
| Leaderboard pages | 15 seconds | Snapshot bucket bounds visibility without invalidating every page on every completion. |
| User leaderboard position | 30 seconds | Uses the synchronized rank snapshot ordering. |

The L1 cache is bounded at 50,000 entries with a five-second TTL. Redis operations have two-second connect/I/O limits, pooled connections, error metrics, and graceful cache-miss fallback. `GET /metrics/cache` reports hits, misses, writes, invalidations, errors, and hit rate.

## API surface

| Method and path | Purpose |
| --- | --- |
| `GET /health` | Liveness check |
| `GET /openapi.json` | Machine-readable contract |
| `GET /metrics/cache` | Cache effectiveness and error counters |
| `POST /api/auth/session` | Create a signed local session |
| `POST /logout` | Expire the session cookie |
| `GET /api/profile` | Identity, coins, rank, progress, and activity |
| `PUT /api/profile/dorm` | Update housing community |
| `GET /api/challenges` | Personalized challenge catalog |
| `POST /api/complete` | Validate and store a completion |
| `GET /api/journal` | List journal entries and temporary photo URLs |
| `GET/PUT /api/journal/{challenge_name}` | Read or update one journal entry |
| `DELETE /api/journal/{challenge_name}/photo` | Delete a journal photo |
| `GET /api/rewards` | Reward catalog and user redemption history |
| `POST /api/transaction` | Atomically create a redemption |
| `DELETE /api/transaction/{transaction_id}` | Cancel an eligible redemption |
| `GET /api/leaderboard` | Stable cursor-paginated leaderboard |
| `GET /api/admin/challenges` | Staff challenge details, including verification data |
| `PUT /api/admin/challenges/geolocation` | Set poster location |
| `POST /api/admin/verify_transaction` | Verify reward pickup |

## Local development

### Prerequisites

- Rust 1.88 or newer
- Bun
- PostgreSQL 17
- Redis 8
- MinIO or compatible S3 storage
- Docker Desktop for the reproducible local stack and load test

### Start the isolated local services

The load-test stack is also a convenient development stack and never uses the retired servers:

```powershell
docker compose -f loadtest/docker-compose.yml up -d
Copy-Item backend/.env.example backend/.env
```

Apply migrations and start the API:

```powershell
cargo run --manifest-path backend/migration/Cargo.toml -- up
cargo run --manifest-path backend/Cargo.toml
```

Important configuration:

```dotenv
DATABASE_URL=postgres://quest:quest@127.0.0.1:55432/quest_after
DATABASE_MAX_CONNECTIONS=64
DATABASE_MIN_CONNECTIONS=8
REDIS_URL=redis://127.0.0.1:56379
REDIS_POOL_SIZE=24
MINIO_ENDPOINT=http://127.0.0.1:59000
MINIO_ACCESS_KEY=quest-minio
MINIO_SECRET_KEY=quest-minio-secret
MINIO_BUCKET=quest
SESSION_SECRET=replace-with-at-least-32-random-bytes
SESSION_COOKIE_SECURE=false
SESSION_COOKIE_SAME_SITE=Lax
ALLOW_SELF_REGISTRATION=true
CORS_ALLOWED_ORIGINS=http://localhost:1420,http://tauri.localhost,tauri://localhost
```

Then run the client:

```powershell
Set-Location frontend
bun install
bun run dev
```

Use `VITE_API_BASE_URL` only when the client should call a different explicitly owned API origin. For native development, use `bun tauri android dev` or `bun tauri ios dev` with the appropriate platform toolchain.

### Seed and poster data

Place the challenge CSV at `backend/data/challenges.csv`, then run:

```powershell
cargo run --manifest-path backend/Cargo.toml --bin seed
cargo run --manifest-path backend/Cargo.toml --bin qr-export
```

The QR export is input to the separately access-controlled poster generator. Treat challenge verification values as operational secrets; frontend build variables are not a secret store.

## Performance validation

The reproducible comparison is documented in [`loadtest/README.md`](loadtest/README.md). It creates separate `quest_before` and `quest_after` databases, seeds 5,000 users, 120 challenges, 150,000 completions, and 40,000 transactions, then runs the same k6 journey against a frozen original binary and the rebuilt backend.

The definitive profile ramps from 0 to 600 virtual users over 5 minutes, holds for 5 minutes, and ramps down for 1 minute. Users pause realistically between profile, dorm, completion, challenge, leaderboard, and reward actions. Targets are p95 latency at most 300 ms, error rate below 1%, throughput at least 100 requests/second, and cache hit rate at least 80% for the rebuilt backend.

See [`loadtest/results/comparison.md`](loadtest/results/comparison.md) for measured results, endpoint percentiles, charts, query plans, limitations, and next steps.

## Build and deployment

The backend and frontend support independent container builds. Deploy PostgreSQL, Redis, and private S3-compatible storage as managed or replicated services; run one or more stateless backend replicas behind TLS; serve the web/PWA bundle from a static origin; and configure only owned origins and secrets at runtime.

Before release:

1. Run backend tests and strict Clippy checks.
2. Build the frontend and regenerate its schema when the OpenAPI contract changes.
3. Apply database migrations before admitting traffic to new API replicas.
4. Confirm `/health` and `/metrics/cache`, session-cookie security, CORS, object bucket privacy, backups, and restore procedures.
5. Run the load profile against a staging environment with production-like infrastructure.

## License

O-Quest is dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE-2.0`.
