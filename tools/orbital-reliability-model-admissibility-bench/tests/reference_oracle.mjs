#!/usr/bin/env node
import fs from "node:fs";

const supportedProfiles = new Set([
  "homogeneous_poisson_constant_intensity",
  "power_law_recurrent_minimal_repair",
  "weibull_first_failure_life",
  "nonhomogeneous_poisson_recurrent_events",
  "accelerated_life",
  "degradation_path",
  "unknown_or_unregistered",
]);
const fail = message => {
  process.stderr.write(message + "\n");
  process.exitCode = 1;
};
const has = value => typeof value === "string" && value.trim().length > 0;
const digest = value => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

function oracle(caseData) {
  const blockers = new Set();
  let disposition = null;
  const uniqueArticles = [];
  const records = new Map();
  const serials = new Set();
  const clusters = new Set();
  const eventsSeen = new Map();
  let eventLevelRecords = 0;
  let recurrentArticles = 0;

  const add = code => blockers.add(code);

  if (caseData.articles.length === 0) add("empty_article_cohort");
  if (caseData.effective_cutoff > caseData.knowledge_cutoff) {
    add("knowledge_cutoff_precedes_effective_cutoff");
  }
  if (![caseData.preregistration_ref, caseData.sampling_frame_ref,
        caseData.dependence_assessment_ref, caseData.assumption_assessment_ref].every(has)) {
    add("missing_preregistration_sampling_or_assumption_reference");
  }
  if (!caseData.diagnostics_predeclared || !caseData.thresholds_frozen_before_holdout) {
    add("diagnostics_or_thresholds_not_predeclared");
  }
  if (caseData.analysis_request === "assumption_challenge" &&
      ![caseData.diagnostics_protocol_ref, caseData.multiplicity_policy_ref,
        caseData.holdout_split_ref].every(has)) {
    add("missing_diagnostic_protocol_multiplicity_or_holdout");
  }

  if (!supportedProfiles.has(caseData.selected_profile)) {
    throw new Error("fixture contains unknown profile enum");
  }
  if (["nonhomogeneous_poisson_recurrent_events", "accelerated_life",
       "degradation_path", "unknown_or_unregistered"].includes(caseData.selected_profile)) {
    add("selected_model_profile_not_implemented_in_v0_1");
    disposition = "model_not_supported";
  }
  if (caseData.selected_profile === "weibull_first_failure_life") {
    if (caseData.analysis_request !== "first_failure_life") {
      add("weibull_profile_requires_first_failure_life_request");
      disposition = "model_not_supported";
    }
    if (caseData.input_resolution !== "event_level") {
      add("weibull_first_failure_requires_event_level_input");
      disposition = "indeterminate";
    }
  }
  if (caseData.selected_profile === "power_law_recurrent_minimal_repair") {
    if (caseData.analysis_request !== "power_law_recurrent_estimate") {
      add("power_law_profile_requires_power_law_recurrent_estimate_request");
      disposition = "model_not_supported";
    }
    if (caseData.input_resolution !== "event_level") {
      add("power_law_profile_requires_event_level_input");
      disposition = "model_not_supported";
    }
  }
  if (caseData.selected_profile === "homogeneous_poisson_constant_intensity") {
    if (caseData.analysis_request === "first_failure_life") {
      add("constant_intensity_profile_cannot_answer_first_failure_life_request");
      disposition = "model_not_supported";
    }
    if (caseData.analysis_request === "assumption_challenge" &&
        caseData.input_resolution !== "event_level") {
      add("aggregate_only_input_cannot_challenge_time_constant_assumption");
    }
  }

  if (caseData.input_resolution === "event_level" &&
      ![caseData.event_capture_assessment_ref, caseData.clock_calibration_ref,
        caseData.maintenance_policy_ref].every(has)) {
    add("missing_event_capture_clock_or_maintenance_reference");
  }

  for (const article of caseData.articles) {
    if (!has(article.record_id) || !has(article.article_serial) ||
        !has(article.sampling_cluster_id)) {
      add("missing_record_article_or_cluster_identity");
      continue;
    }
    if (records.has(article.record_id)) {
      if (JSON.stringify(records.get(article.record_id)) === JSON.stringify(article)) {
        add("duplicate_record_id");
      } else {
        disposition = "disputed";
        add("conflicting_record_id");
      }
      continue;
    }
    if (serials.has(article.article_serial)) {
      disposition = "disputed";
      add("multiple_rows_for_same_article_require_prior_aggregation");
      continue;
    }
    records.set(article.record_id, article);
    serials.add(article.article_serial);
    clusters.add(article.sampling_cluster_id);
    uniqueArticles.push(article);

    const scopeKeys = ["population_id", "lot_id", "configuration_id",
      "operating_mode_id", "environment_profile_id"];
    if (scopeKeys.some(key => article[key] !== caseData[key])) {
      add("population_lot_configuration_mode_or_environment_mismatch");
    }
    if (article.exposure_seconds === 0 ||
        article.reported_failure_events > 100000 ||
        !digest(article.raw_artifact_sha256)) {
      add("invalid_exposure_event_count_or_artifact_digest");
    }
    if (!article.evidence_complete || !article.exposure_clock_verified ||
        article.clock_error_bound_seconds === null ||
        article.clock_error_bound_seconds > caseData.max_clock_error_seconds ||
        !article.event_capture_complete || !article.data_loss_counter_verified) {
      add("incomplete_measurement_clock_or_event_capture_evidence");
    }
    if (["intervention", "informative_withdrawal", "unknown"].includes(article.censoring)) {
      add("intervention_or_unresolved_informative_censoring");
    }
    if (article.reported_failure_events > 0 && !article.maintenance_history_complete) {
      add("incomplete_maintenance_or_restoration_history");
    }

    if (caseData.input_resolution === "aggregate_count_exposure") {
      if (article.events.length > 0) add("aggregate_resolution_contains_unscoped_event_rows");
    } else {
      if (article.events.length !== article.reported_failure_events) {
        add("event_rows_do_not_match_reported_failure_count");
      }
      eventLevelRecords += 1;
      if (article.reported_failure_events > 1) recurrentArticles += 1;
      let previousTime = null;
      for (const event of article.events) {
        if (!has(event.event_id)) add("empty_event_id");
        if (eventsSeen.has(event.event_id)) {
          if (JSON.stringify(eventsSeen.get(event.event_id)) === JSON.stringify(event)) {
            add("duplicate_event_id");
          } else {
            disposition = "disputed";
            add("conflicting_event_id");
          }
        } else {
          eventsSeen.set(event.event_id, event);
        }
        if (event.failure_definition_id !== caseData.failure_definition_id ||
            event.event_time_seconds === 0 ||
            event.event_time_seconds > article.exposure_seconds ||
            (previousTime !== null && event.event_time_seconds <= previousTime)) {
          add("event_time_order_window_or_definition_invalid");
        }
        if (event.effective_at > caseData.effective_cutoff ||
            event.recorded_at > caseData.knowledge_cutoff) {
          add("event_not_known_at_declared_cutoff");
        }
        if (caseData.selected_profile === "power_law_recurrent_minimal_repair" &&
            event.restoration_state !== "minimal_repair") {
          add("power_law_profile_requires_minimal_repair_event_history");
          disposition = "model_not_supported";
        } else if (article.reported_failure_events > 1 &&
            ["unknown", "imperfect_repair"].includes(event.restoration_state)) {
          add("recurrent_event_restoration_semantics_unresolved_or_unsupported");
        }
        previousTime = event.event_time_seconds;
      }
    }
  }

  if (clusters.size < uniqueArticles.length) {
    add("shared_sampling_cluster_prevents_independent_article_claim");
  }
  if (caseData.selected_profile === "power_law_recurrent_minimal_repair" &&
      caseData.analysis_request === "power_law_recurrent_estimate" &&
      caseData.input_resolution === "event_level") {
    if (uniqueArticles.length !== 1) {
      add("power_law_profile_requires_exactly_one_repairable_series");
      disposition = "model_not_supported";
    }
  } else if (uniqueArticles.length < 2) {
    add("fewer_than_two_distinct_articles");
  }

  if (caseData.selected_profile === "weibull_first_failure_life" &&
      uniqueArticles.some(article => article.reported_failure_events > 1)) {
    add("recurrent_event_history_not_admissible_for_first_failure_weibull_profile");
    disposition = "model_not_supported";
  }
  if (caseData.selected_profile === "homogeneous_poisson_constant_intensity" &&
      uniqueArticles.some(article => article.reported_failure_events > 1)) {
    if (!has(caseData.maintenance_policy_ref)) {
      add("missing_maintenance_policy_for_recurrent_process");
    }
    if (uniqueArticles.some(article => article.reported_failure_events > 1 &&
        !article.maintenance_history_complete)) {
      add("repair_restoration_history_missing_for_recurrent_process");
    }
  }

  const hardDispute = disposition === "disputed";
  const modelUnsupported = disposition === "model_not_supported";
  const hardRejectCodes = new Set([
    "population_lot_configuration_mode_or_environment_mismatch",
    "invalid_exposure_event_count_or_artifact_digest",
    "event_rows_do_not_match_reported_failure_count",
    "event_time_order_window_or_definition_invalid",
    "event_not_known_at_declared_cutoff",
    "aggregate_resolution_contains_unscoped_event_rows",
    "diagnostics_or_thresholds_not_predeclared",
  ]);
  const hardReject = [...blockers].some(code => hardRejectCodes.has(code));

  if (hardDispute) disposition = "disputed";
  else if (modelUnsupported) disposition = "model_not_supported";
  else if (hardReject) disposition = "rejected";
  else if (blockers.size > 0) disposition = "indeterminate";
  else disposition = "supported_for_analysis";

  const diagnosticsReady =
    caseData.analysis_request === "assumption_challenge" &&
    caseData.input_resolution === "event_level" &&
    disposition === "supported_for_analysis" &&
    eventLevelRecords === uniqueArticles.length &&
    has(caseData.diagnostics_protocol_ref) &&
    has(caseData.multiplicity_policy_ref) &&
    has(caseData.holdout_split_ref);

  const sortedBlockers = [...blockers].sort();
  return {
    disposition,
    article_count: uniqueArticles.length,
    cluster_count: clusters.size,
    total_exposure_seconds: uniqueArticles.reduce((sum, a) => sum + a.exposure_seconds, 0),
    total_failure_events: uniqueArticles.reduce((sum, a) => sum + a.reported_failure_events, 0),
    event_level_records: eventLevelRecords,
    recurrent_event_articles: recurrentArticles,
    assumption_diagnostics_ready: diagnosticsReady,
    blockers: sortedBlockers,
  };
}

const path = process.argv[2];
if (!path) {
  process.stderr.write("usage: reference_oracle.mjs <admissibility-cases.json>\n");
  process.exit(2);
}
const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
if (corpus.schema !== "luminous.orbital-reliability-model-admissibility.corpus.v0.1" ||
    corpus.evidence_class !== "synthetic" ||
    corpus.claim_ceiling !== "synthetic-model-input-admissibility-only") {
  throw new Error("unsupported model-admissibility corpus contract");
}
if (corpus.cases.length !== 63) throw new Error("expected 63 frozen model-admissibility cases");

let mismatches = 0;
let supported = 0;
for (const scenario of corpus.cases) {
  const actual = oracle(scenario);
  const expected = scenario.expected;
  const keys = [
    "disposition", "article_count", "cluster_count", "total_exposure_seconds",
    "total_failure_events", "event_level_records", "recurrent_event_articles",
    "assumption_diagnostics_ready",
  ];
  const different = keys.some(key => actual[key] !== expected[key]) ||
    JSON.stringify(actual.blockers) !== JSON.stringify(expected.blockers);
  if (different) {
    mismatches += 1;
    process.stderr.write(
      scenario.case_id + ": expected " + expected.disposition +
      "/" + expected.blockers.length + " blockers; got " +
      actual.disposition + "/" + actual.blockers.length + "\n",
    );
  }
  if (actual.disposition === "supported_for_analysis") supported += 1;
}
if (supported < 1) throw new Error("reject-all behavior is not accepted: no positive control");
if (mismatches > 0) process.exit(1);
process.stdout.write("PASS: 63 cases; independent admissibility oracle; " +
  supported + " supported-for-analysis controls; zero mismatches; no hazard confirmation\n");
