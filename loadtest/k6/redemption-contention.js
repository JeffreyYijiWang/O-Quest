import http from "k6/http";
import { check, sleep } from "k6";
import { Counter, Trend } from "k6/metrics";

const requests = Number.parseInt(__ENV.REQUESTS || "100", 10);
const baseUrl = __ENV.BASE_URL || "http://host.docker.internal:3000";
const rewardName = __ENV.REWARD_NAME || "Atomic Stress Reward";
const sameUser = (__ENV.SAME_USER || "false") === "true";
const userPrefix = __ENV.USER_PREFIX || "stress";
const summaryName = __ENV.SUMMARY_NAME || "redemption-stress";

const accepted = new Counter("redemption_accepted");
const rejected = new Counter("redemption_rejected");
const transportErrors = new Counter("redemption_transport_errors");
const redemptionDuration = new Trend("redemption_duration", true);

export const options = {
  scenarios: {
    synchronized_burst: {
      executor: "per-vu-iterations",
      vus: requests,
      iterations: 1,
      maxDuration: "2m",
    },
  },
  summaryTrendStats: ["avg", "med", "p(90)", "p(95)", "p(99)", "max"],
};

export function setup() {
  return { releaseAt: Date.now() + 3000 };
}

export default function (data) {
  const waitMilliseconds = data.releaseAt - Date.now();
  if (waitMilliseconds > 0) {
    sleep(waitMilliseconds / 1000);
  }

  const userId = sameUser
    ? `${userPrefix}-same`
    : `${userPrefix}-${String(__VU).padStart(5, "0")}`;
  const response = http.post(
    `${baseUrl}/api/transaction`,
    JSON.stringify({ reward_name: rewardName, count: 1 }),
    {
      headers: {
        "Content-Type": "application/json",
        "x-quest-user-id": userId,
        "x-quest-user-name": `Stress User ${userId}`,
      },
      tags: { endpoint: "redemption-contention" },
      timeout: "60s",
    },
  );

  redemptionDuration.add(response.timings.duration);
  let payload = null;
  try {
    payload = response.json();
  } catch (_) {
    // A failed atomic stock update currently maps to an empty HTTP 500 response.
  }

  if (response.status === 200 && payload?.success === true) {
    accepted.add(1);
  } else if (response.status === 200 && payload?.success === false) {
    rejected.add(1);
  } else {
    transportErrors.add(1);
  }

  check(response, {
    "response is a handled decision or stock-contention rejection": (res) =>
      res.status === 200 || res.status === 500,
  });
}

export function handleSummary(data) {
  return {
    [`/results/${summaryName}.json`]: JSON.stringify(
      {
        generated_at: new Date().toISOString(),
        configuration: { requests, reward_name: rewardName, same_user: sameUser },
        k6: data,
      },
      null,
      2,
    ),
  };
}
