#!/usr/bin/env node
"use strict";

import fs from "node:fs";

// Deterministic smoke study for the family-wise five-statistic PLP suite.
// This is not a full size/power certification campaign.

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
const STATISTIC_COUNT = 5;
const STATISTIC_IDS = [
  "laplace-two-sided-v1",
  "greenwood-two-sided-v1",
  "fitted-plp-uniform-order-statistics-cramer-von-mises-v1",
  "anderson-darling-upper-tail-v1",
  "kolmogorov-smirnov-upper-tail-v1"
];

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

function logOneMinusExp(value) {
  if (!Number.isFinite(value) || value >= 0) throw new Error("invalid negative log transform");
  return value < -Math.LN2
    ? Math.log1p(-Math.exp(value))
    : Math.log(-Math.expm1(value));
}

function fiveStatistics(sortedLogTransforms) {
  const n = sortedLogTransforms.length;
  if (n === 0) throw new Error("empty transform sample");
  const values = [];
  let previous = -Infinity;
  for (const value of sortedLogTransforms) {
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
    adSum += (2 * rank - 1)
      * (sortedLogTransforms[i] + logOneMinusExp(sortedLogTransforms[n - rank]));
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

function estimate(eventTimes) {
  let sum = 0;
  let previous = 0;
  for (const time of eventTimes) {
    if (!Number.isSafeInteger(time) || time <= previous || time <= 0 || time >= ENDPOINT) {
      throw new Error("invalid generated event chronology");
    }
    sum += Math.log1p((ENDPOINT - time) / time);
    previous = time;
  }
  const beta = eventTimes.length / sum;
  const logs = eventTimes.map(time =>
    beta * Math.log1p(-(ENDPOINT - time) / ENDPOINT));
  return { beta, statistics: fiveStatistics(logs) };
}

function generatePlpHistory(beta, seed) {
  const rng = new SplitMix64(seed);
  const times = [];
  for (let i = 0; i < EVENT_COUNT; i += 1) {
    times.push(Math.floor(ENDPOINT * rng.uniformOpen() ** (1 / beta)));
  }
  times.sort((a, b) => a - b);
  if (new Set(times).size !== times.length) {
    throw new Error("generated duplicate event times; deterministic seed must be revised");
  }
  return times;
}

function conditionalReference(n, rng) {
  const exponentials = [];
  let total = 0;
  for (let i = 0; i < n; i += 1) {
    const value = rng.exponentialUnit();
    exponentials.push(value);
    total += value;
  }
  if (!Number.isFinite(total) || total <= 0) throw new Error("invalid conditional normalizer");
  exponentials.sort((a, b) => b - a);
  return fiveStatistics(exponentials.map(value => -(n * value / total)));
}

function conditionalSuitePValues(eventTimes, referenceSeed) {
  const observed = estimate(eventTimes);
  const rng = new SplitMix64(referenceSeed);
  const counts = {
    laplace: 0,
    greenwoodLower: 0,
    greenwoodUpper: 0,
    cvm: 0,
    ad: 0,
    ks: 0
  };
  for (let i = 0; i < INNER_REPLICATES; i += 1) {
    const sample = conditionalReference(eventTimes.length, rng);
    if (Math.abs(sample.laplace) >= Math.abs(observed.statistics.laplace)) counts.laplace += 1;
    if (sample.greenwood <= observed.statistics.greenwood) counts.greenwoodLower += 1;
    if (sample.greenwood >= observed.statistics.greenwood) counts.greenwoodUpper += 1;
    if (sample.cvm >= observed.statistics.cvm) counts.cvm += 1;
    if (sample.ad >= observed.statistics.ad) counts.ad += 1;
    if (sample.ks >= observed.statistics.ks) counts.ks += 1;
  }
  const plusOne = count => (count + 1) / (INNER_REPLICATES + 1);
  const componentPValues = [
    plusOne(counts.laplace),
    Math.min(1, 2 * plusOne(Math.min(counts.greenwoodLower, counts.greenwoodUpper))),
    plusOne(counts.cvm),
    plusOne(counts.ad),
    plusOne(counts.ks)
  ];
  return {
    betaHat: observed.beta,
    observedStatistics: observed.statistics,
    componentPValues,
    familywiseAdjustedPValue: Math.min(1, STATISTIC_COUNT * Math.min(...componentPValues))
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
    const familywiseCounts = Object.fromEntries(ALPHAS.map(alpha => [String(alpha), 0]));
    const componentCounts = Object.fromEntries(
      STATISTIC_IDS.map(id => [id, Object.fromEntries(ALPHAS.map(alpha => [String(alpha), 0]))])
    );
    let betaHatSum = 0;

    for (let replicate = 0; replicate < OUTER_REPLICATES; replicate += 1) {
      // Separate, deterministic seed domains for the outer history and inner
      // conditional reference. They do not share a generator state.
      const historySeed = BASE_SEED + betaIndex * 1_000_000 + replicate * 2;
      const referenceSeed = historySeed + 1;
      const history = generatePlpHistory(trueBeta, historySeed);
      const result = conditionalSuitePValues(history, referenceSeed);
      betaHatSum += result.betaHat;
      for (const alpha of ALPHAS) {
        if (result.familywiseAdjustedPValue <= alpha) {
          familywiseCounts[String(alpha)] += 1;
        }
        for (let i = 0; i < STATISTIC_IDS.length; i += 1) {
          if (result.componentPValues[i] <= alpha) {
            componentCounts[STATISTIC_IDS[i]][String(alpha)] += 1;
          }
        }
      }
    }

    const familywiseRates = {};
    for (const alpha of ALPHAS) {
      const rejected = familywiseCounts[String(alpha)];
      const [lower, upper] = clopperPearson95(rejected, OUTER_REPLICATES);
      familywiseRates[String(alpha)] = {
        rejected,
        trials: OUTER_REPLICATES,
        rate: rejected / OUTER_REPLICATES,
        exact_two_sided_95_interval: [lower, upper]
      };
    }

    const componentRates = {};
    for (const statisticId of STATISTIC_IDS) {
      componentRates[statisticId] = {};
      for (const alpha of ALPHAS) {
        const rejected = componentCounts[statisticId][String(alpha)];
        const [lower, upper] = clopperPearson95(rejected, OUTER_REPLICATES);
        componentRates[statisticId][String(alpha)] = {
          unadjusted_rejected: rejected,
          trials: OUTER_REPLICATES,
          rate: rejected / OUTER_REPLICATES,
          exact_two_sided_95_interval: [lower, upper]
        };
      }
    }

    scenarios.push({
      true_beta: trueBeta,
      event_count: EVENT_COUNT,
      endpoint_seconds: ENDPOINT,
      mean_fitted_beta: betaHatSum / OUTER_REPLICATES,
      familywise_rejection_rates: familywiseRates,
      component_unadjusted_rejection_rates: componentRates
    });
  }

  const output = {
    schema: "luminous.orbital-plp-outer-suite-null-calibration.v0.2",
    evidence_class: "synthetic",
    study_kind: "deterministic_outer_familywise_null_calibration_smoke_study",
    base_seed: BASE_SEED,
    outer_replicates_per_beta: OUTER_REPLICATES,
    inner_conditional_replicates: INNER_REPLICATES,
    prng_id: "splitmix64-top52-open-uniform-exp-normalize-v1",
    suite_id: "plp-five-statistic-bonferroni-suite-v0.2",
    statistic_ids: STATISTIC_IDS,
    multiplicity_method: "bonferroni",
    alphas: ALPHAS,
    scenarios,
    claim_ceiling: "finite_familywise_smoke_calibration_only",
    calibrated_for_all_sample_sizes_or_alternatives: false,
    physical_reliability_claim_granted: false,
    qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
}

main();
