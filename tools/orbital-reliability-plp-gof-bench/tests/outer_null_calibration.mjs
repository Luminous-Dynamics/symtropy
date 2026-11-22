#!/usr/bin/env node
"use strict";

// Deterministic outer null-calibration smoke study. This is not the full
// operating-characteristic evidence programme: increase OUTER_REPLICATES
// only in a separately pre-registered campaign with an independent seed set.

const MASK64 = (1n << 64n) - 1n;
const INCREMENT = 0x9E3779B97F4A7C15n;
const MIX1 = 0xBF58476D1CE4E5B9n;
const MIX2 = 0x94D049BB133111EBn;
const TRUE_BETAS = [0.6, 1.0, 1.6];
const ALPHAS = [0.01, 0.05, 0.10];
const OUTER_REPLICATES = 250;
const INNER_REPLICATES = 999;
const EVENT_COUNT = 10;
const ENDPOINT = 1_000_000_000;
const BASE_SEED = 20_261_122;

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
  exponentialUnit() { return -Math.log(this.uniformOpen()); }
}

function cvm(sortedValues) {
  const n = sortedValues.length;
  let statistic = 1 / (12 * n);
  let previous = 0;
  for (let i = 0; i < n; i += 1) {
    const value = sortedValues[i];
    if (!Number.isFinite(value) || value < 0 || value > 1 || value < previous) {
      throw new Error("invalid fitted transform");
    }
    statistic += (value - (2 * i + 1) / (2 * n)) ** 2;
    previous = value;
  }
  return statistic;
}

function referenceCvm(n, rng) {
  const exponentials = [];
  let total = 0;
  for (let i = 0; i < n; i += 1) {
    const value = rng.exponentialUnit();
    exponentials.push(value);
    total += value;
  }
  exponentials.sort((a, b) => b - a);
  return cvm(exponentials.map(value => Math.exp(-n * value / total)));
}

function fitCvm(eventTimes, endpoint) {
  let sum = 0;
  let previous = 0;
  for (const time of eventTimes) {
    if (!Number.isSafeInteger(time) || time <= previous || time <= 0 || time >= endpoint) {
      throw new Error("invalid generated chronology");
    }
    sum += Math.log(endpoint / time);
    previous = time;
  }
  const betaHat = eventTimes.length / sum;
  const transformed = eventTimes.map(time => (time / endpoint) ** betaHat);
  return { betaHat, statistic: cvm(transformed) };
}

function generatePlpHistory(beta, rng) {
  const eventTimes = [];
  for (let i = 0; i < EVENT_COUNT; i += 1) {
    eventTimes.push(Math.floor(ENDPOINT * rng.uniformOpen() ** (1 / beta)));
  }
  eventTimes.sort((a, b) => a - b);
  return eventTimes;
}

function conditionalPValue(eventTimes, beta, referenceSeed) {
  const observed = fitCvm(eventTimes, ENDPOINT);
  const referenceRng = new SplitMix64(referenceSeed);
  let exceedances = 0;
  for (let i = 0; i < INNER_REPLICATES; i += 1) {
    if (referenceCvm(eventTimes.length, referenceRng) >= observed.statistic) {
      exceedances += 1;
    }
  }
  return {
    pValue: (exceedances + 1) / (INNER_REPLICATES + 1),
    betaHat: observed.betaHat,
    observedCvm: observed.statistic,
    exceedances
  };
}

function logChoose(n, k) {
  let result = 0;
  const m = Math.min(k, n - k);
  for (let i = 1; i <= m; i += 1) {
    result += Math.log(n - m + i) - Math.log(i);
  }
  return result;
}

function logAddExp(a, b) {
  if (a === -Infinity) return b;
  if (b === -Infinity) return a;
  const maximum = Math.max(a, b);
  return maximum + Math.log(Math.exp(a - maximum) + Math.exp(b - maximum));
}

function logBinomialMass(k, n, p) {
  if (p <= 0) return k === 0 ? 0 : -Infinity;
  if (p >= 1) return k === n ? 0 : -Infinity;
  return logChoose(n, k) + k * Math.log(p) + (n - k) * Math.log1p(-p);
}

function logBinomialCdfAtMost(k, n, p) {
  if (k < 0) return -Infinity;
  if (k >= n) return 0;
  let sum = -Infinity;
  for (let j = 0; j <= k; j += 1) {
    sum = logAddExp(sum, logBinomialMass(j, n, p));
  }
  return Math.min(sum, 0);
}

function logBinomialTailAtLeast(k, n, p) {
  if (k <= 0) return 0;
  if (k > n) return -Infinity;
  let sum = -Infinity;
  for (let j = k; j <= n; j += 1) {
    sum = logAddExp(sum, logBinomialMass(j, n, p));
  }
  return Math.min(sum, 0);
}

function clopperPearson95(successes, trials) {
  if (successes < 0 || successes > trials || trials <= 0) {
    throw new Error("invalid binomial count");
  }
  const logTail = Math.log(0.025);
  let lower = 0;
  let upper = 1;
  if (successes > 0) {
    for (let i = 0; i < 80; i += 1) {
      const mid = (lower + upper) / 2;
      if (logBinomialTailAtLeast(successes, trials, mid) < logTail) lower = mid;
      else upper = mid;
    }
  }
  const lowerBound = successes === 0 ? 0 : (lower + upper) / 2;
  lower = 0;
  upper = 1;
  if (successes < trials) {
    for (let i = 0; i < 80; i += 1) {
      const mid = (lower + upper) / 2;
      if (logBinomialCdfAtMost(successes, trials, mid) > logTail) lower = mid;
      else upper = mid;
    }
  }
  const upperBound = successes === trials ? 1 : (lower + upper) / 2;
  return [lowerBound, upperBound];
}

function main() {
  const scenarios = [];
  for (let betaIndex = 0; betaIndex < TRUE_BETAS.length; betaIndex += 1) {
    const trueBeta = TRUE_BETAS[betaIndex];
    const outerRng = new SplitMix64(BASE_SEED + betaIndex * 100_000);
    const counts = Object.fromEntries(ALPHAS.map(alpha => [String(alpha), 0]));
    let betaHatSum = 0;
    for (let replicate = 0; replicate < OUTER_REPLICATES; replicate += 1) {
      const history = generatePlpHistory(trueBeta, outerRng);
      const referenceSeed = Number(outerRng.nextU64() & BigInt(Number.MAX_SAFE_INTEGER));
      const result = conditionalPValue(history, trueBeta, referenceSeed);
      betaHatSum += result.betaHat;
      for (const alpha of ALPHAS) {
        if (result.pValue <= alpha) counts[String(alpha)] += 1;
      }
    }

    const rejectionRates = {};
    for (const alpha of ALPHAS) {
      const count = counts[String(alpha)];
      const [lower, upper] = clopperPearson95(count, OUTER_REPLICATES);
      rejectionRates[String(alpha)] = {
        rejected: count,
        trials: OUTER_REPLICATES,
        rate: count / OUTER_REPLICATES,
        exact_two_sided_95_interval: [lower, upper]
      };
    }

    scenarios.push({
      true_beta: trueBeta,
      event_count: EVENT_COUNT,
      endpoint_seconds: ENDPOINT,
      mean_fitted_beta: betaHatSum / OUTER_REPLICATES,
      rejection_rates: rejectionRates
    });
  }

  const output = {
    schema: "luminous.orbital-plp-outer-null-calibration.v0.1",
    evidence_class: "synthetic",
    study_kind: "deterministic_outer_null_calibration_smoke_study",
    base_seed: BASE_SEED,
    outer_replicates_per_beta: OUTER_REPLICATES,
    inner_conditional_replicates: INNER_REPLICATES,
    prng_id: "splitmix64-top52-open-uniform-exp-normalize-v1",
    statistic_id: "fitted-plp-uniform-order-statistics-cramer-von-mises-v1",
    alphas: ALPHAS,
    scenarios,
    claim_ceiling: "finite_smoke_calibration_only",
    calibrated_for_all_sample_sizes_or_alternatives: false,
    physical_reliability_claim_granted: false,
    qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
}

main();
