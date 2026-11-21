import assert from "node:assert/strict";
import fs from "node:fs";

const fixturePath = process.argv[2];
if (!fixturePath) throw new Error("usage: node reference_oracle.mjs <reliability-cases.json>");
const corpus = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
assert.equal(corpus.schema, "luminous.orbital-interface-reliability.corpus.v0.1");
assert.equal(corpus.evidence_class, "synthetic");
assert.equal(corpus.claim_ceiling, "synthetic-independent-mission-binomial-semantics-only");

function logAddExp(a, b) {
  if (a === -Infinity) return b;
  if (b === -Infinity) return a;
  const m = Math.max(a, b);
  return m + Math.log(Math.exp(a - m) + Math.exp(b - m));
}

// This oracle computes each binomial log-PMF from log combinations,
// rather than using the production recurrence for adjacent PMFs.
function logCdf(n, failures, q) {
  if (failures >= n) return 0;
  if (q <= 0) return 0;
  if (q >= 1) return -Infinity;
  const lq = Math.log(q);
  const lr = Math.log1p(-q);
  let aggregate = -Infinity;
  for (let k = 0; k <= failures; k += 1) {
    let logChoose = 0;
    for (let j = 1; j <= k; j += 1) {
      logChoose += Math.log(n - j + 1) - Math.log(j);
    }
    const logMass = logChoose + k * lq + (n - k) * lr;
    aggregate = logAddExp(aggregate, logMass);
  }
  return Math.min(aggregate, 0);
}
function reliabilityLower(n, failures, confidence) {
  if (!Number.isInteger(n) || n < 1 || n > 100000 || failures > n
      || !(confidence > 0 && confidence < 1)) return null;
  if (failures === n) return 0;
  const logAlpha = Math.log(1 - confidence);
  let low = 0;
  let high = 1;
  for (let i = 0; i < 100; i += 1) {
    const mid = low + (high - low) / 2;
    if (logCdf(n, failures, mid) > logAlpha) low = mid;
    else high = mid;
  }
  return Math.max(0, Math.min(1, 1 - high));
}
function minimumZeroFailure(target, confidence) {
  if (!(target > 0 && target < 1 && confidence > 0 && confidence < 1)) return null;
  let n = Math.ceil(Math.log(1 - confidence) / Math.log(target));
  if (n < 1 || n > 100000) return null;
  while (reliabilityLower(n, 0, confidence) < target) n += 1;
  while (n > 1 && reliabilityLower(n - 1, 0, confidence) >= target) n -= 1;
  return n;
}
function empty(disposition, duplicates = 0) {
  return {
    disposition, unique_trials: 0, successes: 0, failures: 0, restricted_outcomes: 0,
    invalid_trials: 0, aborted_trials: 0, withdrawn_trials: 0, censored_trials: 0,
    duplicate_trials: duplicates, lower_confidence_bound: null, meets_reliability_target: null,
    naive_success_fraction: null, naive_baseline_false_assurance: false,
    minimum_zero_failure_trials: null, naive_cycle_lower_bound: null,
    naive_cycle_false_assurance: false
  };
}
function validDigest(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}
function assess(input) {
  if (!(Number.isFinite(input.reliability_target)
      && input.reliability_target > 0 && input.reliability_target < 1
      && Number.isFinite(input.confidence)
      && input.confidence > 0 && input.confidence < 1)) return empty("rejected");
  if (!input.preregistration_ref || !input.sampling_frame_ref || !input.dependence_assessment_ref) {
    return empty("indeterminate");
  }
  if (input.sampling_design !== "simple_random_single_lot_independent_articles") {
    return empty("model_not_supported");
  }

  const byTrial = new Map();
  const byArticle = new Map();
  const byCluster = new Map();
  const unique = [];
  let duplicates = 0;
  for (const trial of input.trials) {
    if (!trial.trial_id || !trial.article_serial || !trial.sampling_cluster_id) {
      return empty("indeterminate", duplicates);
    }
    if (byTrial.has(trial.trial_id)) {
      if (JSON.stringify(byTrial.get(trial.trial_id)) === JSON.stringify(trial)) {
        duplicates += 1;
        continue;
      }
      return empty("disputed", duplicates);
    }
    if (byArticle.has(trial.article_serial)) {
      const prior = byArticle.get(trial.article_serial);
      const disposition = prior.outcome !== trial.outcome
        || prior.raw_artifact_sha256 !== trial.raw_artifact_sha256
        ? "disputed" : "indeterminate";
      return empty(disposition, duplicates);
    }
    if (byCluster.has(trial.sampling_cluster_id)) return empty("indeterminate", duplicates);
    byTrial.set(trial.trial_id, trial);
    byArticle.set(trial.article_serial, trial);
    byCluster.set(trial.sampling_cluster_id, trial);
    unique.push(trial);
  }

  if (unique.some(t => t.population_id !== input.population_id
      || t.lot_id !== input.lot_id
      || t.configuration_id !== input.configuration_id
      || t.mission_profile_id !== input.mission_profile_id
      || t.failure_definition_id !== input.failure_definition_id)) {
    return empty("rejected", duplicates);
  }

  let successes = 0;
  let failures = 0;
  let restricted = 0;
  let invalid = 0;
  let aborted = 0;
  let withdrawn = 0;
  let censored = 0;
  for (const t of unique) {
    if (!validDigest(t.raw_artifact_sha256) || !t.evidence_complete
      || !t.mission_profile_verified || t.intervention_during_mission) {
      invalid += 1;
      continue;
    }
    switch (t.outcome) {
      case "success": successes += 1; break;
      case "failure": failures += 1; break;
      case "restricted": restricted += 1; failures += 1; break;
      case "aborted": aborted += 1; break;
      case "withdrawn": withdrawn += 1; break;
      case "invalid": invalid += 1; break;
      case "right_censored": censored += 1; break;
      default: invalid += 1; break;
    }
  }

  const base = {
    ...empty("indeterminate", duplicates),
    unique_trials: unique.length, successes, failures,
    restricted_outcomes: restricted, invalid_trials: invalid, aborted_trials: aborted,
    withdrawn_trials: withdrawn, censored_trials: censored
  };
  if (invalid || aborted || withdrawn || censored) {
    base.naive_success_fraction = successes + failures > 0 ? successes / (successes + failures) : null;
    base.minimum_zero_failure_trials = minimumZeroFailure(input.reliability_target, input.confidence);
    return base;
  }

  const n = successes + failures;
  if (n === 0) return base;
  const lower = reliabilityLower(n, failures, input.confidence);
  const meets = lower >= input.reliability_target;
  const fraction = successes / n;
  return {
    ...base,
    disposition: meets ? "reliability_demonstrated" : "reliability_not_demonstrated",
    lower_confidence_bound: lower,
    meets_reliability_target: meets,
    naive_success_fraction: fraction,
    naive_baseline_false_assurance: fraction >= input.reliability_target && !meets,
    minimum_zero_failure_trials: minimumZeroFailure(input.reliability_target, input.confidence)
  };
}

assert.equal(corpus.cases.length, 36);
const seen = new Set();
for (const item of corpus.cases) {
  assert.ok(!seen.has(item.case_id), "duplicate reliability case " + item.case_id);
  seen.add(item.case_id);
  const actual = assess(item);
  const cycleN = item.reported_cycle_successes + item.reported_cycle_failures;
  if (cycleN > 0 && cycleN <= 100000) {
    actual.naive_cycle_lower_bound = reliabilityLower(
      cycleN, item.reported_cycle_failures, item.confidence
    );
    actual.naive_cycle_false_assurance = actual.naive_cycle_lower_bound >= item.reliability_target
      && actual.disposition !== "reliability_demonstrated";
  }
  const expected = item.expected;
  for (const key of Object.keys(expected)) {
    if (typeof expected[key] === "number") {
      assert.ok(Math.abs(actual[key] - expected[key]) <= 1e-10, item.case_id + " " + key);
    } else {
      assert.deepEqual(actual[key], expected[key], item.case_id + " " + key);
    }
  }
  const permutedTrials = { ...item, trials: [...item.trials].reverse() };
  const actualPermuted = assess(permutedTrials);
  if (cycleN > 0 && cycleN <= 100000) {
    actualPermuted.naive_cycle_lower_bound = reliabilityLower(
      cycleN, item.reported_cycle_failures, item.confidence
    );
    actualPermuted.naive_cycle_false_assurance = actualPermuted.naive_cycle_lower_bound >= item.reliability_target
      && actualPermuted.disposition !== "reliability_demonstrated";
  }
  for (const key of Object.keys(actual)) {
    if (typeof actual[key] === "number") {
      assert.ok(Math.abs(actualPermuted[key] - actual[key]) <= 1e-10, item.case_id + " permutation " + key);
    } else {
      assert.deepEqual(actualPermuted[key], actual[key], item.case_id + " permutation " + key);
    }
  }
}

for (const [target, expected] of [[0.90,22],[0.95,45],[0.99,230]]) {
  assert.equal(minimumZeroFailure(target,0.90),expected);
}
const cycle = corpus.cases.find(c => c.case_id === "case-34-cycle-baseline-false-assurance-on-one-article");
assert.equal(cycle.expected.naive_cycle_false_assurance,true);
assert.equal(cycle.expected.disposition,"reliability_not_demonstrated");
process.stdout.write("independent reliability oracle: 35/35 cases, exact bounds and permutation checks passed\n");
