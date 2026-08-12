import http from "k6/http";
import { check, sleep } from "k6";
import { Gauge, Trend } from "k6/metrics";

const cacheHitRate = new Gauge("cache_hit_rate_percent");
const cacheHits = new Gauge("cache_hits");
const cacheMisses = new Gauge("cache_misses");
const cacheErrors = new Gauge("cache_errors");
const profileDuration = new Trend("profile_duration", true);
const dormDuration = new Trend("dorm_duration", true);
const completionDuration = new Trend("completion_duration", true);
const challengesDuration = new Trend("challenges_duration", true);
const leaderboardDuration = new Trend("leaderboard_duration", true);
const rewardsDuration = new Trend("rewards_duration", true);

const baseUrl = __ENV.BASE_URL || "http://host.docker.internal:3000";
const resultName = __ENV.RESULT_NAME || "loadtest";
const rampDuration = __ENV.RAMP_DURATION || "5m";
const holdDuration = __ENV.HOLD_DURATION || "5m";
const rampDownDuration = __ENV.RAMP_DOWN_DURATION || "1m";
const peakVus = Number(__ENV.PEAK_VUS || "600");

export const options = {
  scenarios: {
    mobile_journey: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: rampDuration, target: peakVus },
        { duration: holdDuration, target: peakVus },
        { duration: rampDownDuration, target: 0 },
      ],
      gracefulRampDown: "30s",
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
    http_req_duration: ["p(95)<300"],
    http_reqs: ["rate>=100"],
    checks: ["rate>0.99"],
  },
  summaryTrendStats: ["avg", "med", "p(90)", "p(95)", "p(99)", "max"],
};

let initialized = false;

function think() {
  sleep(1 + Math.floor(Math.random() * 3));
}

function requestParams(userId) {
  return {
    headers: {
      "Content-Type": "application/json",
      "x-quest-user-id": userId,
      "x-quest-user-name": `Load User ${String(__VU).padStart(4, "0")}`,
    },
    tags: { journey: "mobile" },
  };
}

function expectOk(response, endpoint) {
  check(response, {
    [`${endpoint} returned 2xx`]: (r) => r.status >= 200 && r.status < 300,
  });
}

export function setup() {
  const health = http.get(`${baseUrl}/health`, { tags: { endpoint: "health" } });
  if (health.status !== 200) {
    throw new Error(`API health check failed: ${health.status} ${health.body}`);
  }
}

export default function () {
  const userId = `load-${String(__VU).padStart(4, "0")}`;
  const params = requestParams(userId);

  const profile = http.get(`${baseUrl}/api/profile`, {
    ...params,
    tags: { endpoint: "profile" },
  });
  profileDuration.add(profile.timings.duration);
  expectOk(profile, "profile");
  think();

  if (!initialized && profile.status === 200) {
    const dorm = http.put(
      `${baseUrl}/api/profile/dorm`,
      JSON.stringify({ dorm: "Mudge" }),
      { ...params, tags: { endpoint: "dorm" } },
    );
    dormDuration.add(dorm.timings.duration);
    expectOk(dorm, "dorm");
    think();

    const challengeNumber = ((__VU - 1) % 120) + 1;
    const completion = http.post(
      `${baseUrl}/api/complete`,
      JSON.stringify({
        challenge_name: `Challenge ${String(challengeNumber).padStart(3, "0")}`,
        verification_code: `secret-${challengeNumber}`,
        image_data: "",
        note: "600-user benchmark completion",
        user_latitude: 40.4433,
        user_longitude: -79.9436,
        user_location_accuracy: 10,
      }),
      { ...params, tags: { endpoint: "completion" } },
    );
    completionDuration.add(completion.timings.duration);
    expectOk(completion, "completion");
    think();
    initialized = true;
  }

  const challenges = http.get(`${baseUrl}/api/challenges`, {
    ...params,
    tags: { endpoint: "challenges" },
  });
  challengesDuration.add(challenges.timings.duration);
  expectOk(challenges, "challenges");
  think();

  const leaderboard = http.get(`${baseUrl}/api/leaderboard?limit=20`, {
    ...params,
    tags: { endpoint: "leaderboard" },
  });
  leaderboardDuration.add(leaderboard.timings.duration);
  expectOk(leaderboard, "leaderboard");
  think();

  const rewards = http.get(`${baseUrl}/api/rewards`, {
    ...params,
    tags: { endpoint: "rewards" },
  });
  rewardsDuration.add(rewards.timings.duration);
  expectOk(rewards, "rewards");
  think();
}

export function teardown() {
  const cache = http.get(`${baseUrl}/metrics/cache`, {
    tags: { endpoint: "cache_metrics" },
    responseCallback: http.expectedStatuses(200, 404),
  });
  if (cache.status === 200) {
    try {
      const metrics = cache.json();
      cacheHitRate.add(metrics.hit_rate_percent || 0);
      cacheHits.add(metrics.hits || 0);
      cacheMisses.add(metrics.misses || 0);
      cacheErrors.add(metrics.errors || 0);
    } catch (_) {
      // A malformed metrics response should not hide the request measurements.
    }
  }
}

export function handleSummary(data) {
  const gauge = (name) => data.metrics[name]?.values?.value ?? null;
  const cacheMetrics = gauge("cache_hit_rate_percent") === null
    ? null
    : {
        hit_rate_percent: gauge("cache_hit_rate_percent"),
        hits: gauge("cache_hits"),
        misses: gauge("cache_misses"),
        errors: gauge("cache_errors"),
      };

  const artifact = {
    generated_at: new Date().toISOString(),
    result_name: resultName,
    configuration: {
      base_url: baseUrl,
      peak_vus: peakVus,
      ramp_duration: rampDuration,
      hold_duration: holdDuration,
      ramp_down_duration: rampDownDuration,
    },
    cache: cacheMetrics,
    k6: data,
  };

  return {
    stdout: `${JSON.stringify(artifact, null, 2)}\n`,
    [`/results/${resultName}-summary.json`]: JSON.stringify(artifact, null, 2),
  };
}
