#!/usr/bin/env node
"use strict";

import fs from "node:fs";

const MASK64 = (1n << 64n) - 1n;
const INCREMENT = 0x9E3779B97F4A7C15n;
const MIX1 = 0xBF58476D1CE4E5B9n;
const MIX2 = 0x94D049BB133111EBn;

class SplitMix64 {
  constructor(seed) { this.state = BigInt(seed) & MASK64; }
  nextU64() {
    this.state = (this.state + INCREMENT) & MASK64;
    let value = this.state;
    value = ((value ^ (value >> 30n)) * MIX1) & MASK64;
    value = ((value ^ (value >> 27n)) * MIX2) & MASK64;
    return (value ^ (value >> 31n)) & MASK64;
  }
  uniformOpen() {
    return Number((this.nextU64() >> 12n) + 1n) / 4503599627370497;
  }
}

function observedParameters(physical, capture, horizon) {
  const betaObserved = physical.beta + capture.gamma;
  return {
    eta_observed: physical.eta * capture.p0 * physical.beta / betaObserved
      * horizon ** (-capture.gamma),
    beta_observed: betaObserved
  };
}

function inverseDecomposition(observed, capture, horizon) {
  const beta = observed.beta_observed - capture.gamma;
  return {
    eta: observed.eta_observed * observed.beta_observed / beta
      * horizon ** capture.gamma / capture.p0,
    beta
  };
}

function estimateShape(times) {
  if (times.length === 0) return null;
  let sum = 0;
  let previous = 0;
  for (const time of times) {
    if (!Number.isFinite(time) || time <= previous || time >= 1) {
      throw new Error("invalid normalized observed event times");
    }
    sum += -Math.log(time);
    previous = time;
  }
  return times.length / sum;
}

function simulate(caseData) {
  const physicalRng = new SplitMix64(caseData.physical_seed);
  const captureRng = new SplitMix64(caseData.capture_seed);
  const retained = [];
  for (let i = 0; i < caseData.physical_event_count; i += 1) {
    const time = physicalRng.uniformOpen() ** (1 / caseData.physical_beta);
    const probability = Math.min(1, caseData.capture_p0 * time ** caseData.capture_gamma);
    if (captureRng.uniformOpen() < probability) retained.push(time);
  }
  retained.sort((a, b) => a - b);
  const estimated = estimateShape(retained);
  const theoretical = caseData.physical_beta + caseData.capture_gamma;
  const error = estimated === null ? null : Math.abs(estimated - theoretical);
  return {
    captured_count: retained.length,
    estimated_observed_beta: estimated,
    theoretical_observed_beta: theoretical,
    beta_absolute_error: error,
    within_declared_tolerance: error !== null && error <= caseData.maximum_beta_error
  };
}

function close(a, b, tolerance = 1e-10) {
  return Math.abs(a - b) <= tolerance * Math.max(Math.abs(b), 1);
}

function main() {
  const file = process.argv[2];
  if (!file) {
    process.stderr.write("usage: node reference_oracle.mjs <identifiability-cases.json>\n");
    process.exitCode = 2;
    return;
  }
  const corpus = JSON.parse(fs.readFileSync(file, "utf8"));
  if (corpus.schema !== "luminous.orbital-observation-identifiability.corpus.v0.1"
      || corpus.evidence_class !== "synthetic"
      || corpus.claim_ceiling !== "synthetic-thinned-plp-identifiability-only"
      || !Array.isArray(corpus.analytic_cases)
      || !Array.isArray(corpus.sensitivity_cases)
      || !Array.isArray(corpus.simulation_cases)) {
    process.stderr.write("unsupported observation-process identifiability corpus\n");
    process.exitCode = 2;
    return;
  }

  let analyticMismatches = 0;
  let equivalentDecompositions = 0;
  for (const item of corpus.analytic_cases) {
    const observed = observedParameters(item.physical, item.capture, item.exposure_horizon);
    if (!close(observed.eta_observed, item.expected_observed.eta_observed)
        || !close(observed.beta_observed, item.expected_observed.beta_observed)) {
      analyticMismatches += 1;
      continue;
    }
    const betas = item.decomposition_gammas.map(gamma =>
      inverseDecomposition(observed,
        { p0: item.decomposition_p0, gamma },
        item.exposure_horizon).beta);
    if (betas.length !== item.expected_equivalent_physical_betas.length
        || betas.some((beta, index) => !close(beta, item.expected_equivalent_physical_betas[index]))) {
      analyticMismatches += 1;
      continue;
    }
    for (const gamma of item.decomposition_gammas) {
      const physical = inverseDecomposition(observed,
        { p0: item.decomposition_p0, gamma },
        item.exposure_horizon);
      const reproduced = observedParameters(physical,
        { p0: item.decomposition_p0, gamma },
        item.exposure_horizon);
      if (close(reproduced.eta_observed, observed.eta_observed)
          && close(reproduced.beta_observed, observed.beta_observed)) {
        equivalentDecompositions += 1;
      } else {
        analyticMismatches += 1;
      }
    }
  }

  let sensitivityMismatches = 0;
  for (const item of corpus.sensitivity_cases) {
    const lowerRaw = item.beta_observed - item.gamma_max;
    const lowerOpen = lowerRaw <= 0;
    const betaMin = lowerOpen ? 0 : lowerRaw;
    const betaMax = item.beta_observed - item.gamma_min;
    const identifiable = Math.abs(item.gamma_max - item.gamma_min) <= 1e-12;
    if (!close(betaMin, item.expected_beta_min)
        || !close(betaMax, item.expected_beta_max)
        || lowerOpen !== item.expected_lower_open
        || identifiable !== item.expected_identifiable) {
      sensitivityMismatches += 1;
    }
  }

  const simulations = corpus.simulation_cases.map(item => ({
    case_id: item.case_id,
    ...simulate(item)
  }));
  const simulationFailures = simulations.filter(item => !item.within_declared_tolerance).length;

  const output = {
    oracle: "independent-javascript-thinned-plp-identifiability-v0.1",
    analytic_case_count: corpus.analytic_cases.length,
    analytic_expected_result_mismatches: analyticMismatches,
    equivalent_decomposition_count: equivalentDecompositions,
    sensitivity_case_count: corpus.sensitivity_cases.length,
    sensitivity_mismatches: sensitivityMismatches,
    simulation_case_count: simulations.length,
    simulation_failures: simulationFailures,
    simulations,
    physical_shape_identifiable_from_timestamps_alone: false,
    physical_scale_identifiable_from_timestamps_alone: false,
    physical_reliability_claim_granted: false,
    qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
  if (analyticMismatches || sensitivityMismatches || simulationFailures) {
    process.exitCode = 1;
  }
}
main();
