#!/usr/bin/env node
import fs from "node:fs";

const MODEL_PROFILE = "power_law_process_repairable_minimal_repair";

function digest(value, length) {
  return typeof value === "string" &&
    new RegExp("^[0-9a-f]{" + length + "}$").test(value);
}

function poissonCdf(k, mean) {
  if (mean <= 0) return 1;
  let mass = Math.exp(-mean);
  let sum = mass;
  for (let i = 1; i <= k; i += 1) {
    mass *= mean / i;
    sum += mass;
  }
  return Math.min(1, Math.max(0, sum));
}

// For integer n, Gamma(n,1) tails equal Poisson(n-1) tails.
// This probability-domain recurrence is independent of Rust's incomplete-gamma
// series / continued-fraction implementation.
function gammaLowerInteger(n, x) {
  return 1 - poissonCdf(n - 1, x);
}

function deviance(ratio) {
  return ratio - 1 - Math.log(ratio);
}

function lowerRoot(target) {
  let low = 0;
  let high = 1;
  for (let i = 0; i < 120; i += 1) {
    const mid = (low + high) / 2;
    if (mid === 0 || deviance(mid) > target) low = mid;
    else high = mid;
  }
  return (low + high) / 2;
}

function upperRoot(target) {
  let low = 1;
  let high = 2;
  while (deviance(high) < target) high *= 2;
  for (let i = 0; i < 120; i += 1) {
    const mid = (low + high) / 2;
    if (deviance(mid) < target) low = mid;
    else high = mid;
  }
  return (low + high) / 2;
}

function exactConditionalPValue(n, ratio) {
  const target = deviance(ratio);
  if (target <= 1e-13) return 1;
  const lo = lowerRoot(target);
  const hi = upperRoot(target);
  return Math.min(1, Math.max(0,
    gammaLowerInteger(n, n * lo) + poissonCdf(n - 1, n * hi),
  ));
}

function evaluate(scenario) {
  const n = scenario.event_times_seconds.length;
  const blockers = [];
  const add = code => { if (!blockers.includes(code)) blockers.push(code); };

  if (scenario.model_profile !== MODEL_PROFILE) {
    add("selected_model_profile_is_not_power_law_minimal_repair");
    return { disposition: "model_not_supported", event_count: n,
      beta_hat: null, likelihood_ratio_statistic: null,
      exact_conditional_p_value: null, blockers };
  }
  if (!digest(scenario.admissibility_receipt_sha256, 64) ||
      !digest(scenario.admissibility_source_commit, 40)) {
    add("missing_or_malformed_admissibility_receipt_pin");
    return { disposition: "indeterminate", event_count: n,
      beta_hat: null, likelihood_ratio_statistic: null,
      exact_conditional_p_value: null, blockers };
  }
  if (!Number.isFinite(scenario.significance_level) ||
      !(0 < scenario.significance_level && scenario.significance_level < 1)) {
    add("invalid_significance_level");
    return { disposition: "rejected", event_count: n,
      beta_hat: null, likelihood_ratio_statistic: null,
      exact_conditional_p_value: null, blockers };
  }

  const times = scenario.event_times_seconds;
  if (scenario.exposure_end_seconds <= 0 ||
      !Number.isSafeInteger(scenario.exposure_end_seconds) ||
      times.some(t => !Number.isSafeInteger(t) ||
        t <= 0 || t >= scenario.exposure_end_seconds) ||
      times.some((t, i) => i > 0 && times[i - 1] >= t)) {
    add("event_history_or_exposure_window_invalid");
    return { disposition: "rejected", event_count: n,
      beta_hat: null, likelihood_ratio_statistic: null,
      exact_conditional_p_value: null, blockers };
  }
  if (n < 2) {
    add("fewer_than_two_events_for_trend_test");
    return { disposition: "indeterminate", event_count: n,
      beta_hat: null, likelihood_ratio_statistic: null,
      exact_conditional_p_value: null, blockers };
  }

  const sum = times.reduce(
    (acc, time) => acc + Math.log(scenario.exposure_end_seconds / time),
    0,
  );
  const ratio = sum / n;
  const beta = n / sum;
  const lr = 2 * n * deviance(ratio);
  const p = exactConditionalPValue(n, ratio);
  return {
    disposition: p <= scenario.significance_level
      ? "trend_signal_detected"
      : "no_trend_signal_detected",
    event_count: n,
    beta_hat: beta,
    likelihood_ratio_statistic: lr,
    exact_conditional_p_value: p,
    blockers,
  };
}

function close(actual, expected, tolerance = 1e-10) {
  if (actual === null || expected === null) return actual === expected;
  return Math.abs(actual - expected) <= tolerance * Math.max(1, Math.abs(actual));
}

const path = process.argv[2];
if (!path) {
  process.stderr.write("usage: reference_oracle.mjs <trend-cases.json>\n");
  process.exit(2);
}
const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
if (corpus.schema !== "luminous.orbital-reliability-plp-trend.corpus.v0.1" ||
    corpus.evidence_class !== "synthetic" ||
    corpus.claim_ceiling !== "synthetic-constant-vs-plp-trend-diagnostic-only") {
  throw new Error("unsupported PLP trend corpus contract");
}
if (corpus.cases.length !== 24) throw new Error("expected 24 frozen PLP trend cases");

let mismatches = 0;
let trendSignals = 0;
let noTrendSignals = 0;
for (const scenario of corpus.cases) {
  const actual = evaluate(scenario);
  const expected = scenario.expected;
  const mismatch =
    actual.disposition !== expected.disposition ||
    actual.event_count !== expected.event_count ||
    !close(actual.beta_hat, expected.beta_hat) ||
    !close(actual.likelihood_ratio_statistic, expected.likelihood_ratio_statistic) ||
    !close(actual.exact_conditional_p_value, expected.exact_conditional_p_value) ||
    JSON.stringify(actual.blockers) !== JSON.stringify(expected.blockers);
  if (mismatch) {
    mismatches += 1;
    process.stderr.write(
      scenario.case_id + ": expected " + expected.disposition +
      ", got " + actual.disposition +
      "; expected p=" + expected.exact_conditional_p_value +
      ", got p=" + actual.exact_conditional_p_value + "\n",
    );
  }
  if (actual.disposition === "trend_signal_detected") trendSignals += 1;
  if (actual.disposition === "no_trend_signal_detected") noTrendSignals += 1;
  if (actual.disposition === "trend_signal_detected" &&
      (expected.beta_hat === null || expected.exact_conditional_p_value === null)) {
    throw new Error("trend signal without a valid estimate / p-value");
  }
}
if (trendSignals < 1 || noTrendSignals < 1) {
  throw new Error("both trend and no-trend positive controls are required");
}
if (mismatches > 0) process.exit(1);
process.stdout.write(
  "PASS: 24 cases; independent probability-domain gamma-tail oracle; " +
  trendSignals + " trend signals; " + noTrendSignals +
  " no-trend signals; zero mismatches; no fit acceptance or authority\n",
);
