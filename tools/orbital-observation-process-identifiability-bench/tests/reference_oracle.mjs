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


function logAddExp(left,right){
  if(left===-Infinity)return right;if(right===-Infinity)return left;
  const m=Math.max(left,right);return m+Math.log(Math.exp(left-m)+Math.exp(right-m));
}
function logBinomialCdfAtMost(k,n,p){
  if(k>=n)return 0;if(p<=0)return 0;if(p>=1)return -Infinity;
  const lp=Math.log(p),lq=Math.log1p(-p);let mass=n*lq,sum=mass;
  for(let j=1;j<=k;j++){mass+=Math.log(n-j+1)-Math.log(j)+lp-lq;sum=logAddExp(sum,mass);}
  return Math.min(sum,0);
}
function logBinomialTailAtLeast(k,n,p){
  if(k===0)return 0;if(k>n)return -Infinity;if(p<=0)return -Infinity;if(p>=1)return 0;
  const lp=Math.log(p),lq=Math.log1p(-p);let mass=n*lq;
  for(let j=1;j<=k;j++)mass+=Math.log(n-j+1)-Math.log(j)+lp-lq;
  let sum=mass;
  for(let j=k+1;j<=n;j++){mass+=Math.log(n-j+1)-Math.log(j)+lp-lq;sum=logAddExp(sum,mass);}
  return Math.min(sum,0);
}
function clopperPearson(successes,trials,tailAlpha){
  if(!Number.isInteger(trials)||trials<1||trials>100000||!Number.isInteger(successes)||successes<0||successes>trials||!(tailAlpha>0&&tailAlpha<0.5))throw new Error("invalid POD binomial input");
  const logAlpha=Math.log(tailAlpha);let lower=0;
  if(successes>0){let lo=0,hi=1;for(let i=0;i<100;i++){const mid=lo+(hi-lo)/2;if(logBinomialTailAtLeast(successes,trials,mid)<logAlpha)lo=mid;else hi=mid;}lower=lo+(hi-lo)/2;}
  let upper=1;
  if(successes<trials){let lo=0,hi=1;for(let i=0;i<100;i++){const mid=lo+(hi-lo)/2;if(logBinomialCdfAtMost(successes,trials,mid)>logAlpha)lo=mid;else hi=mid;}upper=lo+(hi-lo)/2;}
  return [lower,upper];
}
function podAnchorValid(a){
  return Number.isFinite(a.normalized_time_fraction)&&a.normalized_time_fraction>0&&a.normalized_time_fraction<=1&&
    Number.isInteger(a.trials)&&a.trials>0&&a.trials<=100000&&Number.isInteger(a.hits)&&a.hits>=0&&a.hits<=a.trials&&
    typeof a.event_class==="string"&&a.event_class.trim().length>0&&
    typeof a.covariate_stratum_id==="string"&&a.covariate_stratum_id.trim().length>0&&
    typeof a.sampling_unit==="string"&&a.sampling_unit.trim().length>0&&
    /^[0-9a-f]{64}$/.test(a.configuration_sha256)&&/^[0-9a-f]{64}$/.test(a.reference_evidence_sha256);
}
function analyzePodTrend(item){
  const alpha=item.familywise_alpha,allocation=alpha/2;
  const indeterminate=reason=>({case_id:item.case_id,status:"indeterminate",gamma_min:null,gamma_max:null,
    expected_result_matched:item.expected_status==="indeterminate",blockers:[reason]});
  const e=item.early_anchor,l=item.late_anchor;
  if(!(alpha>0&&alpha<0.2)||!podAnchorValid(e)||!podAnchorValid(l))return indeterminate("invalid POD anchor or family-wise alpha");
  if(!e.independent_trials_asserted||!l.independent_trials_asserted)return indeterminate("independence of POD sampling units not established");
  if(e.event_class!==l.event_class||e.covariate_stratum_id!==l.covariate_stratum_id||e.configuration_sha256!==l.configuration_sha256||e.sampling_unit!==l.sampling_unit)
    return indeterminate("non-time event/configuration/sampling strata do not match");
  if(l.normalized_time_fraction<=e.normalized_time_fraction)return indeterminate("POD anchor times are not strictly increasing");
  const tail=allocation/2,[eL,eU]=clopperPearson(e.hits,e.trials,tail),[lL,lU]=clopperPearson(l.hits,l.trials,tail);
  const logRatio=Math.log(l.normalized_time_fraction/e.normalized_time_fraction);
  const gammaMin=lL>0?Math.max(0,Math.log(lL/eU)/logRatio):0;
  const caps=[[eL,e.normalized_time_fraction],[lL,l.normalized_time_fraction]].filter(([lo,t])=>lo>0&&t<1).map(([lo,t])=>Math.log(lo)/Math.log(t));
  const p0Cap=caps.length?Math.min(...caps):null;
  const ratioMax=eL>0&&lU>0?Math.log(lU/eL)/logRatio:null;
  const finite=[ratioMax,p0Cap].filter(x=>x!==null),gammaMax=finite.length?Math.min(...finite):null;
  const status=gammaMax!==null&&(gammaMax<gammaMin||gammaMax<0)?"model_incompatible":(gammaMax===null?"unbounded":"bounded");
  let matched=status===item.expected_status;
  if(matched&&status!=="indeterminate"){
    matched=Number.isFinite(item.expected_gamma_min)&&Math.abs(gammaMin-item.expected_gamma_min)<=1e-8;
    matched=matched&&((gammaMax===null&&item.expected_gamma_max===null)||(gammaMax!==null&&item.expected_gamma_max!==null&&Math.abs(gammaMax-item.expected_gamma_max)<=1e-8));
  }
  return {case_id:item.case_id,status,early_pod:{lower:eL,upper:eU,point_estimate:e.hits/e.trials},late_pod:{lower:lL,upper:lU,point_estimate:l.hits/l.trials},
    gamma_min:gammaMin,gamma_max:gammaMax,p0_gamma_cap:p0Cap,familywise_alpha:alpha,anchor_alpha_allocation:allocation,
    expected_result_matched:matched,blockers:["two-anchor power-law POD model only; p0<=1 enforced",
    "evidence digests and independence flag are caller assertions, not authenticated by this evaluator"]};
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
  if (corpus.schema !== "luminous.orbital-observation-identifiability.corpus.v0.2"
      || corpus.evidence_class !== "synthetic"
      || corpus.claim_ceiling !== "synthetic-thinned-plp-identifiability-and-pod-bounds-only"
      || !Array.isArray(corpus.analytic_cases)
      || !Array.isArray(corpus.sensitivity_cases)
      || !Array.isArray(corpus.simulation_cases)
      || !Array.isArray(corpus.pod_trend_cases) || corpus.pod_trend_cases.length===0) {
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

  const podTrendCases=corpus.pod_trend_cases.map(analyzePodTrend);
  const podTrendMismatches=podTrendCases.filter(x=>!x.expected_result_matched).length;
  const output = {
    oracle: "independent-javascript-thinned-plp-identifiability-and-pod-bounds-v0.2",
    analytic_case_count: corpus.analytic_cases.length,
    analytic_expected_result_mismatches: analyticMismatches,
    equivalent_decomposition_count: equivalentDecompositions,
    sensitivity_case_count: corpus.sensitivity_cases.length,
    sensitivity_mismatches: sensitivityMismatches,
    simulation_case_count: simulations.length,
    simulation_failures: simulationFailures,
    pod_trend_case_count: podTrendCases.length,
    pod_trend_expected_result_mismatches: podTrendMismatches,
    pod_trend_bounded_count: podTrendCases.filter(x=>x.status==="bounded").length,
    pod_trend_unbounded_count: podTrendCases.filter(x=>x.status==="unbounded").length,
    pod_trend_incompatible_count: podTrendCases.filter(x=>x.status==="model_incompatible").length,
    pod_trend_indeterminate_count: podTrendCases.filter(x=>x.status==="indeterminate").length,
    pod_trend_cases: podTrendCases,
    simulations,
    physical_shape_identifiable_from_timestamps_alone: false,
    physical_scale_identifiable_from_timestamps_alone: false,
    physical_reliability_claim_granted: false,
    qualification_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
  if (analyticMismatches || sensitivityMismatches || simulationFailures || podTrendMismatches) {
    process.exitCode = 1;
  }
}
main();
