#!/usr/bin/env node
"use strict";

import fs from "node:fs";

// Deterministic smoke grid for conditional null size across beta and event count.
// This is not a full Type-I/FWER certification study.

const MASK64 = (1n << 64n) - 1n;
const INCREMENT = 0x9E3779B97F4A7C15n;
const MIX1 = 0xBF58476D1CE4E5B9n;
const MIX2 = 0x94D049BB133111EBn;
const TRUE_BETAS = [0.6, 1.0, 1.6];
const EVENT_COUNTS = [5, 10, 20, 40];
const ALPHAS = [0.01, 0.05, 0.10];
const OUTER_REPLICATES = 50;
const INNER_REPLICATES = 999;
const ENDPOINT = 1_000_000_000_000;
const BASE_SEED = 20_261_123;
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

function fiveStatistics(logValues) {
  const n = logValues.length;
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
    adSum += (2 * rank - 1)
      * (logValues[i] + logOneMinusExp(logValues[n - rank]));
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

function generatePlpHistory(beta, n, seed) {
  const rng = new SplitMix64(seed);
  const times = [];
  for (let i = 0; i < n; i += 1) {
    times.push(Math.floor(ENDPOINT * rng.uniformOpen() ** (1 / beta)));
  }
  times.sort((a, b) => a - b);
  if (new Set(times).size !== times.length) throw new Error("duplicate generated event time");
  return times;
}

function estimate(eventTimes) {
  let sum = 0;
  let previous = 0;
  for (const time of eventTimes) {
    if (!Number.isSafeInteger(time) || time <= previous || time <= 0 || time >= ENDPOINT) {
      throw new Error("invalid event chronology");
    }
    sum += Math.log1p((ENDPOINT - time) / time);
    previous = time;
  }
  const beta = eventTimes.length / sum;
  const logValues = eventTimes.map(t => beta * Math.log1p(-(ENDPOINT - t) / ENDPOINT));
  return {beta, statistics: fiveStatistics(logValues)};
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

function suiteResult(eventTimes, referenceSeed) {
  const observed = estimate(eventTimes);
  const rng = new SplitMix64(referenceSeed);
  const counts = {laplace:0,greenwoodLower:0,greenwoodUpper:0,cvm:0,ad:0,ks:0};
  for (let i = 0; i < INNER_REPLICATES; i += 1) {
    const simulated = conditionalReference(eventTimes.length, rng);
    if (Math.abs(simulated.laplace) >= Math.abs(observed.statistics.laplace)) counts.laplace += 1;
    if (simulated.greenwood <= observed.statistics.greenwood) counts.greenwoodLower += 1;
    if (simulated.greenwood >= observed.statistics.greenwood) counts.greenwoodUpper += 1;
    if (simulated.cvm >= observed.statistics.cvm) counts.cvm += 1;
    if (simulated.ad >= observed.statistics.ad) counts.ad += 1;
    if (simulated.ks >= observed.statistics.ks) counts.ks += 1;
  }
  const plusOne = x => (x + 1) / (INNER_REPLICATES + 1);
  const p = [
    plusOne(counts.laplace),
    Math.min(1, 2 * plusOne(Math.min(counts.greenwoodLower, counts.greenwoodUpper))),
    plusOne(counts.cvm),
    plusOne(counts.ad),
    plusOne(counts.ks)
  ];
  return {betaHat:observed.beta,componentPValues:p,fwerP:Math.min(1,STATISTIC_COUNT*Math.min(...p))};
}

function logChoose(n,k) {
  let sum=0;
  const m=Math.min(k,n-k);
  for(let i=1;i<=m;i+=1) sum+=Math.log(n-m+i)-Math.log(i);
  return sum;
}
function logAddExp(a,b) {
  if(a===-Infinity)return b;
  if(b===-Infinity)return a;
  const m=Math.max(a,b);
  return m+Math.log(Math.exp(a-m)+Math.exp(b-m));
}
function logMass(k,n,p) {
  if(p<=0)return k===0?0:-Infinity;
  if(p>=1)return k===n?0:-Infinity;
  return logChoose(n,k)+k*Math.log(p)+(n-k)*Math.log1p(-p);
}
function logCdf(k,n,p) {
  if(k<0)return -Infinity;
  if(k>=n)return 0;
  let sum=-Infinity;
  for(let i=0;i<=k;i+=1)sum=logAddExp(sum,logMass(i,n,p));
  return Math.min(sum,0);
}
function logTail(k,n,p) {
  if(k<=0)return 0;
  if(k>n)return -Infinity;
  let sum=-Infinity;
  for(let i=k;i<=n;i+=1)sum=logAddExp(sum,logMass(i,n,p));
  return Math.min(sum,0);
}
function clopperPearson95(k,n) {
  const alpha=Math.log(0.025);
  let lo=0,hi=1;
  if(k>0)for(let i=0;i<80;i++){const mid=(lo+hi)/2;if(logTail(k,n,mid)<alpha)lo=mid;else hi=mid;}
  const lower=k===0?0:(lo+hi)/2;
  lo=0;hi=1;
  if(k<n)for(let i=0;i<80;i++){const mid=(lo+hi)/2;if(logCdf(k,n,mid)>alpha)lo=mid;else hi=mid;}
  const upper=k===n?1:(lo+hi)/2;
  return [lower,upper];
}
function rate(k) {
  const [lower,upper]=clopperPearson95(k,OUTER_REPLICATES);
  return {rejected:k,trials:OUTER_REPLICATES,rate:k/OUTER_REPLICATES,exact_two_sided_95_interval:[lower,upper]};
}

function main() {
  const cells=[];
  let cellIndex=0;
  for(const beta of TRUE_BETAS) {
    for(const n of EVENT_COUNTS) {
      const fwer=Object.fromEntries(ALPHAS.map(alpha=>[String(alpha),0]));
      const components=Object.fromEntries(STATISTIC_IDS.map(id=>[id,Object.fromEntries(ALPHAS.map(alpha=>[String(alpha),0]))]));
      let betaHatSum=0;
      for(let r=0;r<OUTER_REPLICATES;r++) {
        const seed=BASE_SEED+cellIndex*100_000+r*2;
        const events=generatePlpHistory(beta,n,seed);
        const result=suiteResult(events,seed+1);
        betaHatSum+=result.betaHat;
        for(const alpha of ALPHAS) {
          if(result.fwerP<=alpha)fwer[String(alpha)]+=1;
          result.componentPValues.forEach((p,i)=>{if(p<=alpha)components[STATISTIC_IDS[i]][String(alpha)]+=1;});
        }
      }
      cells.push({
        true_beta:beta,event_count:n,endpoint_seconds:ENDPOINT,
        mean_fitted_beta:betaHatSum/OUTER_REPLICATES,
        familywise_rejection_rates:Object.fromEntries(ALPHAS.map(a=>[String(a),rate(fwer[String(a)])])),
        component_unadjusted_rejection_rates:Object.fromEntries(
          STATISTIC_IDS.map(id=>[id,Object.fromEntries(ALPHAS.map(a=>[String(a),rate(components[id][String(a)])]))])
        )
      });
      cellIndex+=1;
    }
  }
  process.stdout.write(JSON.stringify({
    schema:"luminous.orbital-plp-outer-suite-null-strata-smoke.v0.2",
    evidence_class:"synthetic",
    study_kind:"deterministic_outer_familywise_null_strata_smoke",
    true_betas:TRUE_BETAS,event_counts:EVENT_COUNTS,
    outer_replicates_per_cell:OUTER_REPLICATES,inner_conditional_replicates:INNER_REPLICATES,
    cell_count:cells.length,
    prng_id:"splitmix64-top52-open-uniform-exp-normalize-v1",
    suite_id:"plp-five-statistic-bonferroni-suite-v0.2",
    multiplicity_method:"bonferroni",
    alphas:ALPHAS,cells,
    claim_ceiling:"finite_stratified_smoke_calibration_only",
    calibrated_for_all_sample_sizes_or_alternatives:false,
    physical_reliability_claim_granted:false,
    qualification_granted:false,authority_granted:false
  },null,2)+"\n");
}
main();
