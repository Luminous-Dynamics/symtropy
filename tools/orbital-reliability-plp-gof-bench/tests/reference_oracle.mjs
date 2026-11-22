#!/usr/bin/env node
"use strict";

import fs from "node:fs";
const MASK64 = (1n << 64n) - 1n;
const SPLIT_INCREMENT = 0x9E3779B97F4A7C15n;
const MIX1 = 0xBF58476D1CE4E5B9n;
const MIX2 = 0x94D049BB133111EBn;
const MIN_EVENTS = 5;
const MAX_EVENTS = 10000;
const MIN_REPLICATES = 999;
const MAX_REPLICATES = 100000;
const MAX_SAFE = Number.MAX_SAFE_INTEGER;

function validHex(value, length) {
  return typeof value === "string" && value.length === length && new RegExp("^[0-9a-f]{" + length + "}$").test(value);
}

function nonblank(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function wilson(successes, trials) {
  const z = 1.959963984540054;
  const n = trials;
  const p = successes / n;
  const se = Math.sqrt(p * (1 - p) / n);
  const z2 = z * z;
  const den = 1 + z2 / n;
  const center = (p + z2 / (2 * n)) / den;
  const half = z * Math.sqrt(p * (1 - p) / n + z2 / (4 * n * n)) / den;
  return {
    estimate: p,
    se,
    lower: Math.max(0, center - half),
    upper: Math.min(1, center + half)
  };
}

function cvm(values) {
  if (values.length === 0) throw new Error("empty CvM sample");
  const n = values.length;
  let result = 1 / (12 * n);
  let previous = 0;
  for (let i = 0; i < n; i += 1) {
    const value = values[i];
    if (!Number.isFinite(value) || value < 0 || value > 1 || value < previous) {
      throw new Error("invalid sorted fitted transforms");
    }
    const expected = (2 * i + 1) / (2 * n);
    result += (value - expected) ** 2;
    previous = value;
  }
  return result;
}

function estimateBetaCvm(events, endpoint) {
  if (!Number.isSafeInteger(endpoint) || endpoint <= 0 || endpoint > MAX_SAFE) {
    throw new Error("invalid exposure endpoint");
  }
  if (events.length < MIN_EVENTS || events.length > MAX_EVENTS) {
    throw new Error("event count outside supported range");
  }
  let previous = 0;
  let logSum = 0;
  for (const t of events) {
    if (!Number.isSafeInteger(t) || t <= 0 || t >= endpoint || t <= previous) {
      throw new Error("invalid event chronology/window");
    }
    logSum += Math.log(endpoint / t);
    previous = t;
  }
  if (!Number.isFinite(logSum) || logSum <= 0) throw new Error("invalid event-time sum");
  const beta = events.length / logSum;
  if (!Number.isFinite(beta) || beta <= 0) throw new Error("invalid beta");
  const transforms = events.map(t => (t / endpoint) ** beta);
  return { beta, cvm: cvm(transforms) };
}

class SplitMix64 {
  constructor(seed) { this.state = BigInt(seed) & MASK64; }
  nextU64() {
    this.state = (this.state + SPLIT_INCREMENT) & MASK64;
    let z = this.state;
    z = ((z ^ (z >> 30n)) * MIX1) & MASK64;
    z = ((z ^ (z >> 27n)) * MIX2) & MASK64;
    return (z ^ (z >> 31n)) & MASK64;
  }
  uniformOpen() {
    const numerator = Number(this.nextU64() >> 12n) + 1;
    return numerator / 4503599627370497;
  }
  exponentialUnit() { return -Math.log(this.uniformOpen()); }
}

function referenceCvm(n, rng) {
  const values = [];
  let sum = 0;
  for (let i = 0; i < n; i += 1) {
    const x = rng.exponentialUnit();
    values.push(x);
    sum += x;
  }
  if (!Number.isFinite(sum) || sum <= 0) throw new Error("conditional normalization failed");
  values.sort((a, b) => b - a);
  const fitted = values.map(x => Math.exp(-n * x / sum));
  return cvm(fitted);
}

function runGof(events, endpoint, reps, seed) {
  const fit = estimateBetaCvm(events, endpoint);
  const rng = new SplitMix64(seed);
  let exceedances = 0;
  for (let i = 0; i < reps; i += 1) {
    if (referenceCvm(events.length, rng) >= fit.cvm) exceedances += 1;
  }
  const mc = wilson(exceedances, reps);
  return {
    beta_hat: fit.beta,
    observed_cvm: fit.cvm,
    tail_exceedances: exceedances,
    monte_carlo_p_value: (exceedances + 1) / (reps + 1),
    mc
  };
}

function blocked(disposition, eventCount, code) {
  return {
    disposition, event_count: eventCount, beta_hat: null, observed_cvm: null,
    tail_exceedances: null, monte_carlo_p_value: null, blockers: [code]
  };
}

function evaluateCase(c) {
  const eventCount = c.event_times_seconds.length;
  if (c.model_profile !== "power_law_process_repairable_minimal_repair") {
    return blocked("model_not_supported", eventCount, "selected_model_profile_is_not_power_law_minimal_repair");
  }
  if (c.termination_profile !== "administrative_time_terminated_complete_window" || c.observation_start_seconds !== 0) {
    return blocked("model_not_supported", eventCount, "only_complete_fixed_time_terminated_windows_from_zero_are_supported");
  }
  if (!c.complete_event_history) {
    return blocked("indeterminate", eventCount, "complete_event_history_not_established");
  }
  if (c.repair_policy !== "minimal_repair") {
    return blocked("model_not_supported", eventCount, "repair_policy_is_not_minimal_repair");
  }
  if (!nonblank(c.subject_group_id) || !validHex(c.configuration_sha256, 64)
      || !nonblank(c.environment_id) || !nonblank(c.exposure_clock_id)
      || !nonblank(c.analysis_protocol_id) || !nonblank(c.multiplicity_policy_id)) {
    return blocked("rejected", eventCount, "subject_configuration_clock_or_protocol_binding_invalid");
  }
  if (!validHex(c.admissibility_receipt_sha256, 64) || !validHex(c.admissibility_source_commit, 40)
      || !validHex(c.capture_receipt_sha256, 64) || !validHex(c.capture_source_commit, 40)) {
    return blocked("indeterminate", eventCount, "admissibility_or_capture_receipt_pin_missing_or_malformed");
  }
  if (!Number.isFinite(c.significance_level) || c.significance_level < 0.01 || c.significance_level > 0.10) {
    return blocked("rejected", eventCount, "invalid_significance_level");
  }
  if (!Number.isSafeInteger(c.monte_carlo_replicates) || c.monte_carlo_replicates < MIN_REPLICATES
      || c.monte_carlo_replicates > MAX_REPLICATES
      || c.monte_carlo_replicates < Math.ceil(1 / c.significance_level) - 1
      || !Number.isSafeInteger(c.random_seed) || c.random_seed < 0 || c.random_seed > MAX_SAFE) {
    return blocked("rejected", eventCount, "monte_carlo_resolution_or_seed_outside_predeclared_contract");
  }
  if (eventCount < MIN_EVENTS) {
    return blocked("indeterminate", eventCount, "insufficient_event_count_for_v0_1_diagnostic");
  }
  if (eventCount * c.monte_carlo_replicates > 10000000) {
    return blocked("rejected", eventCount, "monte_carlo_total_draw_budget_exceeded");
  }
  try {
    const result = runGof(c.event_times_seconds, c.exposure_end_seconds, c.monte_carlo_replicates, c.random_seed);
    const disposition = result.monte_carlo_p_value <= c.significance_level
      ? "fit_rejected" : "fit_not_rejected_at_declared_level";
    return {
      disposition, event_count: eventCount, beta_hat: result.beta_hat,
      observed_cvm: result.observed_cvm, tail_exceedances: result.tail_exceedances,
      monte_carlo_p_value: result.monte_carlo_p_value, blockers: []
    };
  } catch (error) {
    return blocked("rejected", "event_history_or_exposure_window_invalid");
  }
}

function close(a, b) {
  if (a === null || b === null) return a === b;
  return Math.abs(a - b) <= 1e-10 * Math.max(Math.abs(b), 1);
}

function main() {
  const file = process.argv[2];
  if (!file) {
    process.stderr.write("usage: node reference_oracle.mjs <plp-gof-cases.json>\n");
    process.exitCode = 2;
    return;
  }
  const corpus = JSON.parse(fs.readFileSync(file, "utf8"));
  if (corpus.schema !== "luminous.orbital-plp-conditional-gof.corpus.v0.1"
      || corpus.evidence_class !== "synthetic"
      || corpus.claim_ceiling !== "synthetic-conditional-plp-goodness-of-fit-only"
      || !Array.isArray(corpus.cases) || corpus.cases.length === 0) {
    process.stderr.write("unsupported PLP GOF corpus contract\n");
    process.exitCode = 2;
    return;
  }

  const ids = new Set();
  let mismatches = 0;
  const distribution = Object.create(null);
  for (const c of corpus.cases) {
    if (!nonblank(c.case_id) || ids.has(c.case_id)) {
      process.stderr.write("GOF case IDs must be unique and nonblank\n");
      process.exitCode = 2;
      return;
    }
    ids.add(c.case_id);
    const actual = evaluateCase(c);
    const expected = c.expected;
    const matched = actual.disposition === expected.disposition
      && actual.event_count === expected.event_count
      && close(actual.beta_hat, expected.beta_hat)
      && close(actual.observed_cvm, expected.observed_cvm)
      && actual.tail_exceedances === expected.tail_exceedances
      && close(actual.monte_carlo_p_value, expected.monte_carlo_p_value)
      && JSON.stringify(actual.blockers) === JSON.stringify(expected.blockers);
    if (!matched) {
      mismatches += 1;
      process.stderr.write(JSON.stringify({case_id: c.case_id, actual, expected}) + "\n");
    }
    distribution[actual.disposition] = (distribution[actual.disposition] || 0) + 1;
  }
  const output = {
    oracle: "independent-javascript-conditional-plp-gof-v0.1",
    case_count: corpus.cases.length,
    expected_result_mismatches: mismatches,
    dispositions: distribution,
    fit_acceptance_granted: false,
    reliability_claim_granted: false,
    physical_qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
  if (mismatches !== 0) process.exitCode = 1;
}
main();
