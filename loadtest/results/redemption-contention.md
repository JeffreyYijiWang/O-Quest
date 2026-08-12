# Redemption contention stress test

Date: 2026-08-10

## Purpose

Verify three independent concurrency invariants in the rebuilt redemption path:

1. finite reward stock never becomes negative or produces more committed units than available;
2. simultaneous requests from one user cannot exceed a reward's per-user trade limit;
3. simultaneous requests from one user cannot spend more coins than that user earned.

The test used the optimized release backend, PostgreSQL 17, Redis 8, an isolated `quest_redemption_stress_20260810` database, synthetic load-test identities, and synchronized k6 `per-vu-iterations` bursts. Every result was reconciled against committed PostgreSQL rows after the burst.

## Results

| Attack | Synchronized requests | Constraint | Accepted | Database result | Verdict |
| --- | ---: | --- | ---: | --- | --- |
| Limited inventory | 300 unique users | 50 units, one per user | 50 | stock `0`; 50 rows; 50 units; 50 distinct users | **Pass: zero overselling or duplicates** |
| Duplicate redemption | 100 requests, one user | trade limit `1` | 52 | 52 rows for one user; 51 above limit | **Fail** |
| Balance overspend | 100 requests, one user | 100 earned coins; each item costs 100 | 100 | 10,000 spent; final balance `-9,900` | **Fail** |
| Minimum balance race | 2 requests, one user | 100 earned coins; each item costs 100 | 2 | 200 spent; final balance `-100` | **Fail** |

The 300-request inventory burst completed with p95 request latency of 813.71 ms. The 250 attempts that lost the stock race reached the API but returned HTTP 500 because the handler currently maps the conditional stock-update rejection to an internal error.

A separate 1,000-request probe exceeded the Windows listener backlog: 336 requests reached the application and 664 connections were refused. Among the 336 application requests, exactly 50 units committed and stock remained zero. It is excluded from the verified concurrency number because not every request reached the redemption logic.

## Conclusion

The conditional PostgreSQL stock update protects **reward inventory** across 300 fully delivered simultaneous requests against 50 available units. The broader statement that atomic redemption also protects coin balances and duplicate redemptions is not supported: both invariants fail because their checks occur before the database transaction. The balance invariant fails with only two synchronized requests, so no concurrent value greater than one is defensible for the combined claim.

## Required remediation before retesting

- Acquire a per-user transaction lock (`SELECT ... FOR UPDATE` on the user row or an equivalent PostgreSQL advisory lock).
- Recalculate earned coins, spent coins, and the reward trade-limit total inside the same transaction and after acquiring the lock.
- Keep the conditional stock decrement and transaction insert in that transaction.
- Add an idempotency key or a database uniqueness rule when the product requires duplicate request replay protection.
- Map expected insufficient-stock contention to a handled conflict response instead of HTTP 500.

Raw k6 outputs are `redemption-inventory-300.json`, `redemption-duplicate-100.json`, `redemption-balance-100.json`, and `redemption-balance-2.json` in this directory. The reusable synchronized workload is `loadtest/k6/redemption-contention.js`.
