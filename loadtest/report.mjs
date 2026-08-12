import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const results = path.join(root, "results");

function load(name) {
  return JSON.parse(fs.readFileSync(path.join(results, `${name}-summary.json`), "utf8"));
}

function values(run, metric) {
  return run.k6.metrics?.[metric]?.values || {};
}

function number(value, digits = 2) {
  return Number.isFinite(value) ? value.toFixed(digits) : "n/a";
}

function yesNo(condition) {
  return condition ? "Yes" : "No";
}

function row(run) {
  const duration = values(run, "http_req_duration");
  const requests = values(run, "http_reqs");
  const failed = values(run, "http_req_failed");
  return {
    p50: duration.med,
    p95: duration["p(95)"],
    p99: duration["p(99)"],
    average: duration.avg,
    throughput: requests.rate,
    requests: requests.count,
    errors: (failed.rate || 0) * 100,
    cache: run.cache?.hit_rate_percent ?? null,
  };
}

function endpointRows(beforeRun, afterRun) {
  const endpoints = [
    ["Profile", "profile_duration"],
    ["Dorm update", "dorm_duration"],
    ["Completion", "completion_duration"],
    ["Challenges", "challenges_duration"],
    ["Leaderboard", "leaderboard_duration"],
    ["Rewards", "rewards_duration"],
  ];
  return endpoints.map(([label, metric]) => {
    const before = values(beforeRun, metric);
    const after = values(afterRun, metric);
    return `| ${label} | ${number(before.med)} ms | ${number(before["p(95)"])} ms | ${number(before["p(99)"])} ms | ${number(after.med)} ms | ${number(after["p(95)"])} ms | ${number(after["p(99)"])} ms |`;
  }).join("\n");
}

function svgBars(title, unit, before, after, filename, lowerIsBetter) {
  const width = 760;
  const height = 330;
  const maximum = Math.max(before, after, 1) * 1.15;
  const scale = 520 / maximum;
  const bars = [
    { label: "Before", value: before, color: "#e76f51", y: 115 },
    { label: "After", value: after, color: "#2a9d8f", y: 210 },
  ];
  const markup = bars.map((bar) => {
    const barWidth = Math.max(bar.value * scale, 2);
    return `<text x="28" y="${bar.y + 28}" font-family="system-ui" font-size="18">${bar.label}</text>
<rect x="120" y="${bar.y}" width="${barWidth}" height="42" rx="6" fill="${bar.color}" />
<text x="${Math.min(130 + barWidth, 675)}" y="${bar.y + 28}" font-family="system-ui" font-size="18" font-weight="700">${number(bar.value)} ${unit}</text>`;
  }).join("\n");
  const direction = lowerIsBetter ? "Lower is better" : "Higher is better";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
<rect width="100%" height="100%" fill="#fbfbf8" />
<text x="28" y="44" font-family="system-ui" font-size="25" font-weight="700" fill="#182026">${title}</text>
<text x="28" y="74" font-family="system-ui" font-size="15" fill="#5c6770">${direction} - identical workload and seeded data</text>
${markup}
</svg>`;
  fs.writeFileSync(path.join(results, filename), svg);
}

const beforeRun = load("before");
const afterRun = load("after");
const before = row(beforeRun);
const after = row(afterRun);

const p95Reduction = ((before.p95 - after.p95) / before.p95) * 100;
const throughputChange = ((after.throughput - before.throughput) / before.throughput) * 100;
const afterPasses = after.p95 <= 300 && after.errors < 1 && after.throughput >= 100 && after.cache >= 80;

const csv = [
  "variant,p50_ms,p95_ms,p99_ms,average_ms,throughput_rps,total_requests,error_percent,cache_hit_percent",
  `before,${before.p50},${before.p95},${before.p99},${before.average},${before.throughput},${before.requests},${before.errors},${before.cache ?? "n/a"}`,
  `after,${after.p50},${after.p95},${after.p99},${after.average},${after.throughput},${after.requests},${after.errors},${after.cache ?? "n/a"}`,
].join("\n");
fs.writeFileSync(path.join(results, "comparison.csv"), `${csv}\n`);

const markdown = `# Measured before/after comparison

Generated from the committed k6 journey on ${afterRun.generated_at}. Both runs used the same host, stages, request sequence, think-time distribution, and equivalently seeded PostgreSQL databases.

## Result

| Metric | Before | After | Target | After met target? |
| --- | ---: | ---: | ---: | :---: |
| p50 HTTP latency | ${number(before.p50)} ms | ${number(after.p50)} ms | Reported | - |
| p95 HTTP latency | ${number(before.p95)} ms | ${number(after.p95)} ms | <= 300 ms | ${yesNo(after.p95 <= 300)} |
| p99 HTTP latency | ${number(before.p99)} ms | ${number(after.p99)} ms | Reported | - |
| Average HTTP latency | ${number(before.average)} ms | ${number(after.average)} ms | Reported | - |
| Throughput | ${number(before.throughput)} req/s | ${number(after.throughput)} req/s | >= 100 req/s | ${yesNo(after.throughput >= 100)} |
| Total requests | ${number(before.requests, 0)} | ${number(after.requests, 0)} | Reported | - |
| Request error rate | ${number(before.errors, 3)}% | ${number(after.errors, 3)}% | < 1% | ${yesNo(after.errors < 1)} |
| Cache hit rate | n/a | ${number(after.cache)}% | >= 80% | ${yesNo(after.cache >= 80)} |

The rebuilt backend ${afterPasses ? "met all four acceptance targets" : "did not meet every acceptance target"}. Its p95 latency was ${number(p95Reduction)}% lower and throughput was ${number(Math.abs(throughputChange))}% ${throughputChange >= 0 ? "higher" : "lower"} than the frozen baseline. The baseline has no cache instrumentation, so its hit rate is intentionally n/a rather than estimated.

![p95 latency](latency-p95.svg)

![throughput](throughput.svg)

## Endpoint latency

| Endpoint | Before p50 | Before p95 | Before p99 | After p50 | After p95 | After p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
${endpointRows(beforeRun, afterRun)}

## Controlled method

- Profile: 0 to ${afterRun.configuration.peak_vus} virtual users over ${afterRun.configuration.ramp_duration}, hold ${afterRun.configuration.hold_duration}, down ${afterRun.configuration.ramp_down_duration}.
- Journey: profile, dorm update on the first iteration, completion attempt, challenges, leaderboard, and rewards, with a random 1-3 second pause between actions.
- Seed: 5,000 users, 120 challenges, 150,000 completions, 40,000 transactions, and 12 rewards.
- Baseline source: commit \`ffb70c357e89466fc7d7b0dbfcd3e9d679e2c67a\`; executable SHA-256 \`542D799A5007AD6FA7681F615359E3172FB1AF306B398B68FABCD1E95DD09D1E\`.
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

Raw machine-readable output is in \`before-summary.json\`, \`after-summary.json\`, and \`comparison.csv\`. Query-plan evidence is in \`query-plans-before.txt\` and \`query-plans-after.txt\`.
`;
fs.writeFileSync(path.join(results, "comparison.md"), markdown);

svgBars("p95 HTTP latency at the 600-VU workload", "ms", before.p95, after.p95, "latency-p95.svg", true);
svgBars("Sustained request throughput", "req/s", before.throughput, after.throughput, "throughput.svg", false);

console.log(path.join(results, "comparison.md"));
