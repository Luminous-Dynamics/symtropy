#!/usr/bin/env node
import fs from "node:fs";

const supportedProfiles = new Set([
  "power_law_process_repairable_minimal_repair",
  "homogeneous_poisson_constant_intensity",
  "weibull_first_failure_life",
  "unknown_or_unregistered",
]);
const has = value => typeof value === "string" && value.trim().length > 0;
const digest = value => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const equalRecord = (left, right) => JSON.stringify(left) === JSON.stringify(right);

function independentEstimate(eventTimes, exposureEnd) {
  if (exposureEnd === 0 || exposureEnd > Number.MAX_SAFE_INTEGER ||
      eventTimes.length < 2 || eventTimes.length > 100000) return null;

  // Compute the sufficient statistic through sum(log(t_i)), rather than
  // the Rust implementation's sum(log(T / t_i)).
  const logTimes = eventTimes.map(time => Math.log(time));
  if (eventTimes.some((time, index) =>
      time <= 0 || time >= exposureEnd ||
      (index > 0 && eventTimes[index - 1] >= time))) return null;

  const count = eventTimes.length;
  const logEnd = Math.log(exposureEnd);
  const logTimeSum = logTimes.reduce((sum, value) => sum + value, 0);
  const beta = count / (count * logEnd - logTimeSum);
  const logEta = Math.log(count) - beta * logEnd;
  const endIntensity = beta * count / exposureEnd;
  const halfMean = count * Math.exp(-beta * Math.log(2));

  // Independent likelihood evaluation from the two maximized log-likelihoods.
  const logLPowerLaw =
    count * logEta + count * Math.log(beta) +
    (beta - 1) * logTimeSum - count;
  const logLConstant =
    count * (Math.log(count) - logEnd) - count;
  const lr = Math.max(0, 2 * (logLPowerLaw - logLConstant));

  const values = [beta, logEta, endIntensity, halfMean, lr];
  if (!values.every(Number.isFinite)) return null;
  return {
    beta_hat: beta,
    log_eta_hat: logEta,
    intensity_at_end_per_second: endIntensity,
    cumulative_mean_at_half_exposure: halfMean,
    descriptive_likelihood_ratio_statistic: lr,
    likelihood_ratio_nonnegative: lr >= 0,
  };
}

function evaluate(scenario) {
  const blockers = [];
  const add = code => { if (!blockers.includes(code)) blockers.push(code); };
  let disposition = null;
  let duplicateEvents = 0;
  let disputed = false;
  const eventsById = new Map();
  const uniqueEvents = [];

  if (scenario.selected_profile !== "power_law_process_repairable_minimal_repair") {
    add("selected_model_profile_is_not_power_law_repairable_minimal_repair");
    disposition = "model_not_supported";
  }
  if (scenario.repair_policy_id !== "minimal_repair_nhpp_v0_1") {
    add("repair_policy_is_not_the_declared_minimal_repair_profile");
    disposition = "model_not_supported";
  }
  if (![scenario.preregistration_ref, scenario.sampling_frame_ref,
        scenario.assumption_assessment_ref, scenario.maintenance_policy_ref,
        scenario.event_capture_assessment_ref, scenario.clock_calibration_ref].every(has)) {
    add("missing_preregistration_or_admissibility_reference");
  }
  if (!scenario.diagnostics_predeclared || !scenario.thresholds_frozen_before_holdout) {
    add("analysis_protocol_or_thresholds_not_predeclared");
  }
  if (scenario.knowledge_cutoff < scenario.effective_cutoff) {
    add("knowledge_cutoff_precedes_effective_cutoff");
  }

  const series = scenario.series;
  if (!has(series.series_id) || !has(series.article_serial)) {
    add("missing_series_or_article_identity");
  }
  const scopeKeys = [
    "population_id", "lot_id", "configuration_id",
    "operating_mode_id", "environment_profile_id", "failure_definition_id",
  ];
  if (scopeKeys.some(key => series[key] !== scenario.scope[key])) {
    add("series_scope_differs_from_case_scope");
  }
  if (series.exposure_end_seconds === 0 ||
      series.exposure_end_seconds > Number.MAX_SAFE_INTEGER ||
      !digest(series.raw_artifact_sha256)) {
    add("invalid_exposure_end_or_artifact_digest");
  }
  if (!series.evidence_complete || !series.exposure_clock_verified ||
      series.clock_error_bound_seconds === null ||
      series.clock_error_bound_seconds > scenario.max_clock_error_seconds ||
      !series.event_capture_complete || !series.data_loss_counter_verified ||
      !series.maintenance_history_complete) {
    add("incomplete_clock_capture_maintenance_or_artifact_evidence");
  }
  if (["intervention", "informative_withdrawal", "unknown"].includes(series.censoring)) {
    add("intervention_or_unresolved_informative_censoring");
  }

  for (const event of series.events) {
    if (!has(event.event_id)) {
      add("empty_event_id");
      continue;
    }
    if (eventsById.has(event.event_id)) {
      if (equalRecord(eventsById.get(event.event_id), event)) {
        duplicateEvents += 1;
        add("identical_duplicate_event_record");
      } else {
        disputed = true;
        add("conflicting_event_id");
      }
      continue;
    }
    eventsById.set(event.event_id, event);
    uniqueEvents.push(event);
  }

  uniqueEvents.sort((left, right) =>
    left.operating_time_seconds - right.operating_time_seconds);

  for (let index = 0; index < uniqueEvents.length; index += 1) {
    const event = uniqueEvents[index];
    if (event.failure_definition_id !== scenario.scope.failure_definition_id ||
        event.operating_time_seconds === 0 ||
        event.operating_time_seconds >= series.exposure_end_seconds ||
        (index > 0 &&
          uniqueEvents[index - 1].operating_time_seconds >= event.operating_time_seconds)) {
      add("event_time_order_window_or_failure_definition_invalid");
    }
    if (event.effective_at > scenario.effective_cutoff ||
        event.recorded_at > scenario.knowledge_cutoff) {
      add("event_not_known_at_declared_cutoff");
    }
    if (event.restoration_state !== "minimal_repair") {
      add("restoration_state_is_not_supported_by_v0_1_profile");
      disposition = "model_not_supported";
    }
  }

  if (uniqueEvents.length < 2) {
    add("fewer_than_two_unique_events_for_power_law_mle");
  }
  const estimates = independentEstimate(
    uniqueEvents.map(event => event.operating_time_seconds),
    series.exposure_end_seconds,
  );
  if (series.exposure_end_seconds > 0 &&
      uniqueEvents.length >= 2 &&
      estimates === null) {
    add("power_law_estimator_rejected_event_times");
  }

  if (disputed) {
    disposition = "disputed";
  } else if (disposition !== "model_not_supported") {
    const hardReject = new Set([
      "series_scope_differs_from_case_scope",
      "invalid_exposure_end_or_artifact_digest",
      "event_time_order_window_or_failure_definition_invalid",
      "event_not_known_at_declared_cutoff",
      "analysis_protocol_or_thresholds_not_predeclared",
    ]);
    if (blockers.some(code => hardReject.has(code))) disposition = "rejected";
    else if (blockers.length > 0) disposition = "indeterminate";
    else disposition = "estimate_produced";
  }
  blockers.sort();

  return {
    disposition,
    unique_events: uniqueEvents.length,
    duplicate_events: duplicateEvents,
    exposure_end_seconds: series.exposure_end_seconds,
    beta_hat: estimates?.beta_hat ?? null,
    log_eta_hat: estimates?.log_eta_hat ?? null,
    intensity_at_end_per_second: estimates?.intensity_at_end_per_second ?? null,
    cumulative_mean_at_half_exposure: estimates?.cumulative_mean_at_half_exposure ?? null,
    descriptive_likelihood_ratio_statistic:
      estimates?.descriptive_likelihood_ratio_statistic ?? null,
    likelihood_ratio_nonnegative: estimates?.likelihood_ratio_nonnegative ?? null,
    blockers,
  };
}

function close(actual, expected, tolerance = 1e-10) {
  if (actual === null || expected === null) return actual === expected;
  return Math.abs(actual - expected) <= tolerance * Math.max(1, Math.abs(actual));
}

const path = process.argv[2];
if (!path) {
  process.stderr.write("usage: reference_oracle.mjs <power-law-cases.json>\n");
  process.exit(2);
}
const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
if (corpus.schema !== "luminous.orbital-reliability-power-law.corpus.v0.1" ||
    corpus.evidence_class !== "synthetic" ||
    corpus.claim_ceiling !== "synthetic-power-law-parameter-estimation-only") {
  throw new Error("unsupported power-law corpus contract");
}
if (corpus.cases.length !== 67) throw new Error("expected 67 frozen power-law cases");

let mismatches = 0;
let estimateControls = 0;
for (const scenario of corpus.cases) {
  const actual = evaluate(scenario);
  const expected = scenario.expected;
  const keys = [
    "disposition", "unique_events", "duplicate_events", "exposure_end_seconds",
    "likelihood_ratio_nonnegative",
  ];
  const exactMismatch = keys.some(key => actual[key] !== expected[key]) ||
    JSON.stringify(actual.blockers) !== JSON.stringify(expected.blockers);
  const numericMismatch =
    !close(actual.beta_hat, expected.beta_hat) ||
    !close(actual.log_eta_hat, expected.log_eta_hat) ||
    !close(actual.intensity_at_end_per_second, expected.intensity_at_end_per_second) ||
    !close(actual.cumulative_mean_at_half_exposure, expected.cumulative_mean_at_half_exposure) ||
    !close(actual.descriptive_likelihood_ratio_statistic, expected.descriptive_likelihood_ratio_statistic);
  if (exactMismatch || numericMismatch ||
      expected.inferential_fit_accepted !== false ||
      expected.reliability_claim_granted !== false) {
    mismatches += 1;
    process.stderr.write(
      scenario.case_id + ": expected " + expected.disposition +
      "/" + expected.blockers.length + " blockers; got " +
      actual.disposition + "/" + actual.blockers.length + "\n",
    );
  }
  if (actual.disposition === "estimate_produced") estimateControls += 1;
}
if (estimateControls < 3) throw new Error("positive estimator controls missing");
if (mismatches > 0) process.exit(1);
process.stdout.write(
  "PASS: 67 cases; independent log-likelihood oracle; " +
  estimateControls + " estimator controls; zero mismatches; no inferential fit acceptance\n",
);
