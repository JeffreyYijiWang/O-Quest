# Measured before/after comparison

Generated from the committed k6 journey on 2026-08-10T11:53:20.871Z. Both runs used the same host, stages, request sequence, think-time distribution, and equivalently seeded PostgreSQL databases.

## Result

| Metric | Before | After | Target | After met target? |
| --- | ---: | ---: | ---: | :---: |
| p50 HTTP latency | 98.13 ms | 32.15 ms | Reported | - |
| p95 HTTP latency | 5365.61 ms | 113.33 ms | <= 300 ms | Yes |
| p99 HTTP latency | 18189.07 ms | 505.97 ms | Reported | - |
| Average HTTP latency | 1368.82 ms | 53.80 ms | Reported | - |
| Throughput | 129.13 req/s | 211.46 req/s | >= 100 req/s | Yes |
| Total requests | 86326 | 141094 | Reported | - |
| Request error rate | 0.000% | 0.000% | < 1% | Yes |
| Cache hit rate | n/a | 97.11% | >= 80% | Yes |

The rebuilt backend met all four acceptance targets. Its p95 latency was 97.89% lower and throughput was 63.75% higher than the frozen baseline. The baseline has no cache instrumentation, so its hit rate is intentionally n/a rather than estimated.

![p95 latency](latency-p95.svg)

![throughput](throughput.svg)

## Endpoint latency

| Endpoint | Before p50 | Before p95 | Before p99 | After p50 | After p95 | After p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Profile | 478.29 ms | 2705.61 ms | 4305.13 ms | 33.78 ms | 115.16 ms | 596.12 ms |
| Dorm update | 242.10 ms | 3832.64 ms | 4328.43 ms | 36.62 ms | 66.46 ms | 103.53 ms |
| Completion | 306.89 ms | 3830.35 ms | 4448.69 ms | 57.60 ms | 91.66 ms | 127.88 ms |
| Challenges | 23.23 ms | 48.77 ms | 568.54 ms | 35.47 ms | 120.07 ms | 430.17 ms |
| Leaderboard | 16.14 ms | 491.98 ms | 2031.92 ms | 21.06 ms | 91.63 ms | 544.63 ms |
| Rewards | 3221.08 ms | 16906.64 ms | 25257.94 ms | 37.69 ms | 120.23 ms | 520.64 ms |

## Controlled method

- Profile: 0 to 600 virtual users over 5m, hold 5m, down 1m.
- Journey: profile, dorm update on the first iteration, completion attempt, challenges, leaderboard, and rewards, with a random 1-3 second pause between actions.
- Seed: 5,000 users, 120 challenges, 150,000 completions, 40,000 transactions, and 12 rewards.
- Baseline source: commit `ffb70c357e89466fc7d7b0dbfcd3e9d679e2c67a`; executable SHA-256 `542D799A5007AD6FA7681F615359E3172FB1AF306B398B68FABCD1E95DD09D1E`.
- Isolation: local Docker PostgreSQL, Redis, and MinIO only. No deprecated server or external identity provider was contacted.

## What changed

- Removed per-reward N+1 reads and moved totals into SQL aggregates.
- Added measured composite and ordering indexes.
- Made stock decrement and redemption insertion atomic.
- Replaced rank-offset pagination with stable keyset cursors and synchronized 15-second leaderboard snapshots.
- Added bounded Moka L1 plus shared Redis L2 caching with TTLs and versioned invalidation.
- Added PostgreSQL/Redis pools, timeouts, and gzip response compression.

## Approaches that did not solve the bottleneck

Diagnostic runs (excluded from the table) showed that indexes alone did not remove the global rank/window cost, invalidating every leaderboard key after every write caused cache churn, and a Redis-only hot path added a network round trip to every request. The final design therefore combines query changes with snapshot reuse, bounded staleness, per-family versioning, and a five-second in-process L1.

## Limitations and next work

This is a controlled single-host application benchmark, not a claim about public-internet latency or a specific production cluster. The journey exercises representative JSON endpoints but not photo upload bandwidth, object-storage failure, mobile radio loss, multi-region replication, or database failover. Before launch, repeat the same script in staging, add a media-upload profile, run a long soak test, validate Redis/PostgreSQL failover, and tune pool sizes against the deployed CPU and connection budgets.

Raw machine-readable output is in `before-summary.json`, `after-summary.json`, and `comparison.csv`. Query-plan evidence is in `query-plans-before.txt` and `query-plans-after.txt`.
