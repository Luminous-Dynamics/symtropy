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
const MAX_TOTAL_DRAWS = 10000000;
const STATISTIC_COUNT = 5;

function validHex(value, length) {
  return typeof value === "string" && value.length === length
    && new RegExp("^[0-9a-f]{" + length + "}$").test(value);
}
function nonblank(value) {
  return typeof value === "string" && value.trim().length > 0;
}
function blocked(disposition, eventCount, code) {
  return {
    disposition, event_count: eventCount, beta_hat: null, observed_cvm: null,
    tail_exceedances: null, monte_carlo_p_value: null,
    component_p_values: [], bonferroni_adjusted_min_p_value: null,
    blockers: [code]
  };
}

class SplitMix64 {
  constructor(seed) { this.state = BigInt(seed) & MASK64; }
  next() {
    this.state = (this.state + SPLIT_INCREMENT) & MASK64;
    let value = this.state;
    value = ((value ^ (value >> 30n)) * MIX1) & MASK64;
    value = ((value ^ (value >> 27n)) * MIX2) & MASK64;
    return (value ^ (value >> 31n)) & MASK64;
  }
  uniformOpen() {
    const numerator = Number(this.next() >> 12n) + 1;
    return numerator / 4503599627370497;
  }
  exponentialUnit() { return -Math.log(this.uniformOpen()); }
}

function logOneMinusExp(value) {
  if (!Number.isFinite(value) || value >= 0) throw new Error("invalid log transform");
  return value < -Math.LN2
    ? Math.log1p(-Math.exp(value))
    : Math.log(-Math.expm1(value));
}

function statisticsFromSortedLogTransforms(logValues) {
  const n = logValues.length;
  if (n === 0) throw new Error("empty fitted transform sample");
  const values = [];
  let previous = -Infinity;
  for (const value of logValues) {
    if (!Number.isFinite(value) || value >= 0 || value < previous) {
      throw new Error("invalid ordered log transforms");
    }
    previous = value;
    values.push(Math.exp(value));
  }
  let laplaceSum = 0;
  let greenwood = 0;
  let cvm = 1 / (12 * n);
  let adSum = 0;
  let dPlus = 0;
  let dMinus = 0;
  let previousValue = 0;
  for (let i = 0; i < n; i += 1) {
    const rank = i + 1;
    const value = values[i];
    laplaceSum += value - 0.5;
    cvm += (value - (2 * rank - 1) / (2 * n)) ** 2;
    dPlus = Math.max(dPlus, rank / n - value);
    dMinus = Math.max(dMinus, value - (rank - 1) / n);
    adSum += (2 * rank - 1) * (logValues[i] + logOneMinusExp(logValues[n - rank]));
    const gap = value - previousValue;
    greenwood += gap * gap;
    previousValue = value;
  }
  greenwood += (1 - previousValue) ** 2;
  return {
    laplace: Math.sqrt(12 / n) * laplaceSum,
    greenwood,
    cvm,
    ad: -adSum / n - n,
    ks: Math.max(dPlus, dMinus)
  };
}

function estimate(events, endpoint) {
  if (!Number.isSafeInteger(endpoint) || endpoint <= 0 || endpoint > MAX_SAFE) {
    throw new Error("invalid endpoint");
  }
  if (events.length < MIN_EVENTS || events.length > MAX_EVENTS) {
    throw new Error("unsupported event count");
  }
  let previous = 0;
  let sumLogRatio = 0;
  for (const time of events) {
    if (!Number.isSafeInteger(time) || time <= 0 || time >= endpoint || time <= previous) {
      throw new Error("invalid event chronology/window");
    }
    sumLogRatio += Math.log1p((endpoint - time) / time);
    previous = time;
  }
  if (!Number.isFinite(sumLogRatio) || sumLogRatio <= 0) throw new Error("invalid log exposure sum");
  const beta = events.length / sumLogRatio;
  if (!Number.isFinite(beta) || beta <= 0) throw new Error("invalid fitted beta");
  const logTransforms = events.map(time => beta * Math.log1p(-(endpoint - time) / endpoint));
  return { beta, statistics: statisticsFromSortedLogTransforms(logTransforms) };
}

function conditionalReferenceStatistics(n, rng) {
  const exponentials = [];
  let total = 0;
  for (let i = 0; i < n; i += 1) {
    const value = rng.exponentialUnit();
    exponentials.push(value);
    total += value;
  }
  if (!Number.isFinite(total) || total <= 0) throw new Error("conditional normalisation failed");
  exponentials.sort((a, b) => b - a);
  const logTransforms = exponentials.map(value => -(n * value / total));
  return statisticsFromSortedLogTransforms(logTransforms);
}

function runGof(events, endpoint, replicates, seed) {
  const fit = estimate(events, endpoint);
  const rng = new SplitMix64(seed);
  const counts = { laplace: 0, greenwoodLower: 0, greenwoodUpper: 0, cvm: 0, ad: 0, ks: 0 };
  for (let i = 0; i < replicates; i += 1) {
    const simulated = conditionalReferenceStatistics(events.length, rng);
    if (Math.abs(simulated.laplace) >= Math.abs(fit.statistics.laplace)) counts.laplace += 1;
    if (simulated.greenwood <= fit.statistics.greenwood) counts.greenwoodLower += 1;
    if (simulated.greenwood >= fit.statistics.greenwood) counts.greenwoodUpper += 1;
    if (simulated.cvm >= fit.statistics.cvm) counts.cvm += 1;
    if (simulated.ad >= fit.statistics.ad) counts.ad += 1;
    if (simulated.ks >= fit.statistics.ks) counts.ks += 1;
  }
  const plusOne = count => (count + 1) / (replicates + 1);
  const greenwoodExtreme = Math.min(counts.greenwoodLower, counts.greenwoodUpper);
  const componentPValues = [
    plusOne(counts.laplace),
    Math.min(1, 2 * plusOne(greenwoodExtreme)),
    plusOne(counts.cvm),
    plusOne(counts.ad),
    plusOne(counts.ks)
  ];
  const bonferroni = Math.min(1, STATISTIC_COUNT * Math.min(...componentPValues));
  return {
    beta: fit.beta,
    statistics: fit.statistics,
    tailExceedances: counts.cvm,
    cvmPValue: componentPValues[2],
    componentPValues,
    bonferroniAdjustedMinPValue: bonferroni
  };
}

function evaluateCase(c) {
  const eventCount = c.event_times_seconds.length;
  if (c.model_profile !== "power_law_process_repairable_minimal_repair") {
    return blocked("model_not_supported", eventCount, "selected_model_profile_is_not_power_law_minimal_repair");
  }
  if (c.termination_profile !== "administrative_time_terminated_complete_window"
      || c.observation_start_seconds !== 0) {
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
      || c.monte_carlo_replicates < Math.ceil(STATISTIC_COUNT / c.significance_level) - 1
      || !Number.isSafeInteger(c.random_seed) || c.random_seed < 0 || c.random_seed > MAX_SAFE) {
    return blocked("rejected", eventCount, "familywise_monte_carlo_resolution_or_seed_outside_predeclared_contract");
  }
  if (eventCount < MIN_EVENTS) {
    return blocked("indeterminate", eventCount, "insufficient_event_count_for_v0_2_diagnostic");
  }
  if (eventCount * c.monte_carlo_replicates > MAX_TOTAL_DRAWS) {
    return blocked("rejected", eventCount, "monte_carlo_total_draw_budget_exceeded");
  }
  try {
    const result = runGof(c.event_times_seconds, c.exposure_end_seconds,
      c.monte_carlo_replicates, c.random_seed);
    const disposition = result.bonferroniAdjustedMinPValue <= c.significance_level
      ? "fit_rejected" : "fit_not_rejected_at_declared_level";
    return {
      disposition,
      event_count: eventCount,
      beta_hat: result.beta,
      observed_cvm: result.statistics.cvm,
      tail_exceedances: result.tailExceedances,
      monte_carlo_p_value: result.cvmPValue,
      component_p_values: result.componentPValues,
      bonferroni_adjusted_min_p_value: result.bonferroniAdjustedMinPValue,
      blockers: []
    };
  } catch (_error) {
    return blocked("rejected", eventCount, "event_history_or_exposure_window_invalid");
  }
}

function close(actual, expected) {
  if (actual === null || expected === null) return actual === expected;
  return Math.abs(actual - expected) <= 1.0e-10 * Math.max(Math.abs(expected), 1.0);
}

function main() {
  const file = process.argv[2];
  if (!file) {
    process.stderr.write("usage: node reference_oracle.mjs <plp-gof-cases.json>\n");
    process.exitCode = 2;
    return;
  }
  const corpus = JSON.parse(fs.readFileSync(file, "utf8"));
  if (corpus.schema !== "luminous.orbital-plp-multistat-gof.corpus.v0.2"
      || corpus.evidence_class !== "synthetic"
      || corpus.claim_ceiling !== "synthetic-conditional-plp-five-statistic-suite-only"
      || !Array.isArray(corpus.cases) || corpus.cases.length === 0) {
    process.stderr.write("unsupported PLP GOF corpus contract\n");
    process.exitCode = 2;
    return;
  }

  const ids = new Set();
  const dispositions = Object.create(null);
  let mismatches = 0;
  for (const testCase of corpus.cases) {
    if (!nonblank(testCase.case_id) || ids.has(testCase.case_id)) {
      process.stderr.write("GOF case IDs must be unique and nonblank\n");
      process.exitCode = 2;
      return;
    }
    ids.add(testCase.case_id);
    const actual = evaluateCase(testCase);
    const expected = testCase.expected;
    const componentsMatch = actual.component_p_values.length === expected.component_p_values.length
      && actual.component_p_values.every((value, index) => close(value, expected.component_p_values[index]));
    const matched = actual.disposition === expected.disposition
      && actual.event_count === expected.event_count
      && close(actual.beta_hat, expected.beta_hat)
      && close(actual.observed_cvm, expected.observed_cvm)
      && actual.tail_exceedances === expected.tail_exceedances
      && close(actual.monte_carlo_p_value, expected.monte_carlo_p_value)
      && componentsMatch
      && close(actual.bonferroni_adjusted_min_p_value, expected.bonferroni_adjusted_min_p_value)
      && JSON.stringify(actual.blockers) === JSON.stringify(expected.blockers);
    if (!matched) {
      mismatches += 1;
      process.stderr.write(JSON.stringify({case_id: testCase.case_id, actual, expected}) + "\n");
    }
    dispositions[actual.disposition] = (dispositions[actual.disposition] || 0) + 1;
  }

  const output = {
    oracle: "independent-javascript-conditional-plp-five-statistic-gof-v0.2",
    case_count: corpus.cases.length,
    expected_result_mismatches: mismatches,
    dispositions,
    fit_acceptance_granted: false,
    reliability_claim_granted: false,
    physical_qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
  if (mismatches !== 0) process.exitCode = 1;
}
main();
