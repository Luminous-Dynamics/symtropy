#!/usr/bin/env node
"use strict";

const fs = require("node:fs");

function sha256(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}

function validScope(scope) {
  return scope && typeof scope.sensor_id === "string" && scope.sensor_id.trim().length > 0
    && sha256(scope.firmware_sha256) && sha256(scope.acquisition_config_sha256)
    && typeof scope.event_class === "string" && scope.event_class.trim().length > 0
    && typeof scope.operating_mode_id === "string" && scope.operating_mode_id.trim().length > 0
    && typeof scope.environment_id === "string" && scope.environment_id.trim().length > 0;
}

function sameScope(left, right) {
  const fields = ["sensor_id", "firmware_sha256", "acquisition_config_sha256",
    "event_class", "operating_mode_id", "environment_id"];
  return fields.every((field) => left[field] === right[field]);
}

function logAddExp(left, right) {
  if (left === -Infinity) return right;
  if (right === -Infinity) return left;
  const maximum = Math.max(left, right);
  return maximum + Math.log(Math.exp(left - maximum) + Math.exp(right - maximum));
}

function logBinomialCdfAtMost(misses, trials, probability) {
  if (probability <= 0) return 0;
  if (probability >= 1) return misses === trials ? 0 : -Infinity;
  const logQ = Math.log(probability);
  const logNotQ = Math.log1p(-probability);
  let logMass = trials * logNotQ;
  let logSum = logMass;
  for (let j = 1; j <= misses; j += 1) {
    logMass += Math.log(trials - j + 1) - Math.log(j) + logQ - logNotQ;
    logSum = logAddExp(logSum, logMass);
  }
  return Math.min(logSum, 0);
}

function missUpper95(misses, trials) {
  if (!Number.isSafeInteger(trials) || trials <= 0 || trials > 100000
      || !Number.isSafeInteger(misses) || misses < 0 || misses > trials) return null;
  if (misses === trials) return 1;
  const logAlpha = Math.log(0.05);
  let lower = 0;
  let upper = 1;
  for (let i = 0; i < 120; i += 1) {
    const midpoint = lower + (upper - lower) / 2;
    if (logBinomialCdfAtMost(misses, trials, midpoint) > logAlpha) lower = midpoint;
    else upper = midpoint;
  }
  return upper;
}

function sharedDomain(a, b) {
  const left = new Set(a);
  return b.some((item) => left.has(item));
}

function blocked(disposition, code) {
  return {
    disposition, injected_count: 0, captured_count: 0, missed_count: 0,
    duplicate_count: 0, unexpected_count: 0, upper_miss_probability_95: null,
    blockers: [code]
  };
}

function evaluateCase(c) {
  if (typeof c.case_id !== "string" || !c.case_id.trim() || !validScope(c.qualified_scope) || !validScope(c.test_scope)
      || !sha256(c.reference.reference_artifact_sha256)
      || !sha256(c.counters.counter_artifact_sha256)
      || !sha256(c.calibration.certificate_sha256)
      || !sha256(c.clock.synchronization_artifact_sha256)
      || !Number.isFinite(c.minimum_event_rate_per_second)
      || !Number.isFinite(c.maximum_event_rate_per_second)
      || !Number.isFinite(c.test_window_seconds)
      || c.test_window_seconds <= 0
      || c.minimum_event_rate_per_second < 0
      || c.maximum_event_rate_per_second < c.minimum_event_rate_per_second
      || c.maximum_event_rate_per_second > 1e9
 ) {
    return blocked("rejected", "malformed_scope_rate_or_evidence_digest");
  }

  const referenceEventRate = c.reference_events.length / c.test_window_seconds;
  if (!Number.isFinite(referenceEventRate) || referenceEventRate > 1e9
      || !sameScope(c.qualified_scope, c.test_scope)
      || referenceEventRate < c.minimum_event_rate_per_second
      || referenceEventRate > c.maximum_event_rate_per_second) {
    return blocked("out_of_envelope", "test_configuration_or_event_rate_outside_declared_envelope");
  }

  if (!c.reference.available || typeof c.reference.system_id !== "string" || !c.reference.system_id.trim()
      || c.reference.failure_domains.length === 0
      || c.reference.acquisition_failure_domains.length === 0
      || c.reference.failure_domains.some((domain) => !domain.trim())
      || c.reference.acquisition_failure_domains.some((domain) => !domain.trim())
      || sharedDomain(c.reference.failure_domains, c.reference.acquisition_failure_domains)) {
    return blocked("indeterminate", "independent_reference_missing_or_common_cause_unresolved");
  }

  if (!c.calibration.valid_at_test_time || !c.calibration.scope_matches) {
    return blocked("indeterminate", "calibration_missing_invalid_or_out_of_scope");
  }

  const measured = c.clock.measured_max_error_ticks;
  if (!c.clock.synchronized || !c.clock.monotonic || !Number.isSafeInteger(measured)
      || measured < 0 || !Number.isSafeInteger(c.clock.allowed_max_error_ticks)
      || measured > c.clock.allowed_max_error_ticks) {
    return blocked("indeterminate", "clock_integrity_or_error_bound_not_established");
  }

  const counterKeys = ["overflow_count", "dropped_event_count", "dropped_packet_count", "reset_count", "sequence_gap_count"];
  if (counterKeys.some((key) => !Number.isSafeInteger(c.counters[key]) || c.counters[key] < 0)) {
    return blocked("indeterminate", "required_capture_loss_counter_missing");
  }
  const loss = counterKeys.some((key) => c.counters[key] > 0);

  const references = c.reference_events;
  if (!Array.isArray(references) || references.length === 0 || references.length > 100000) {
    return blocked("rejected", "independent_reference_event_stream_invalid");
  }
  const referenceById = new Map();
  let priorSeq = null;
  let priorTime = null;
  for (const event of references) {
    if (typeof event.event_id !== "string" || !event.event_id.trim() || !sha256(event.payload_sha256) || !Number.isSafeInteger(event.sequence)
        || !Number.isSafeInteger(event.time_ticks) || referenceById.has(event.event_id)) {
      return blocked("rejected", "independent_reference_event_stream_invalid");
    }
    if ((priorSeq !== null && (priorSeq >= Number.MAX_SAFE_INTEGER || event.sequence !== priorSeq + 1))
        || (priorTime !== null && event.time_ticks <= priorTime)) {
      return blocked("rejected", "independent_reference_event_stream_invalid");
    }
    referenceById.set(event.event_id, event);
    priorSeq = event.sequence;
    priorTime = event.time_ticks;
  }

  if (!Array.isArray(c.observed_events) || c.observed_events.length > 100000) {
    return blocked("rejected", "observed_event_count_exceeds_v0_1_limit");
  }

  const recordIds = new Set();
  const capturedByReference = new Map();
  let capturedCount = 0;
  let duplicateCount = 0;
  let unexpectedCount = 0;
  let disputed = false;

  for (const observed of c.observed_events) {
    if (typeof observed.record_id !== "string" || !observed.record_id.trim()
        || typeof observed.reference_event_id !== "string" || !observed.reference_event_id.trim()
        || !sha256(observed.payload_sha256)
        || !Number.isSafeInteger(observed.time_ticks) || recordIds.has(observed.record_id)) {
      const result = blocked("disputed", "duplicate_record_identity_or_malformed_observation");
      return result;
    }
    recordIds.add(observed.record_id);
    const reference = referenceById.get(observed.reference_event_id);
    if (!reference) {
      unexpectedCount += 1;
      continue;
    }
    const count = (capturedByReference.get(observed.reference_event_id) || 0) + 1;
    capturedByReference.set(observed.reference_event_id, count);
    if (count > 1) duplicateCount += 1;
    if (observed.payload_sha256 !== reference.payload_sha256
        || Math.abs(observed.time_ticks - reference.time_ticks) > measured) {
      disputed = true;
    }
  }

  let missedCount = 0;
  for (const event of references) {
    const count = capturedByReference.get(event.event_id) || 0;
    if (count === 0) missedCount += 1;
    else capturedCount += 1;
  }

  const disposition = (disputed || duplicateCount > 0 || unexpectedCount > 0)
    ? "disputed"
    : (missedCount > 0 || loss) ? "incomplete" : "reconciled_fixture_only";
  const blockers = [];
  if (missedCount > 0) blockers.push("independently_injected_events_not_captured");
  if (duplicateCount > 0) blockers.push("reference_event_captured_more_than_once");
  if (unexpectedCount > 0) blockers.push("observation_has_no_matching_independent_reference");
  if (disputed) blockers.push("payload_or_timestamp_disagrees_with_independent_reference");
  if (loss) blockers.push("capture_loss_counter_nonzero");

  return {
    disposition, injected_count: references.length, captured_count: capturedCount,
    missed_count: missedCount, duplicate_count: duplicateCount, unexpected_count: unexpectedCount,
    upper_miss_probability_95: !loss && duplicateCount === 0 && unexpectedCount === 0 && !disputed
      ? missUpper95(missedCount, references.length) : null,
    blockers
  };
}

function close(a, b) {
  if (a === null || b === null) return a === b;
  return Math.abs(a - b) <= Math.max(1e-12, Math.abs(b) * 1e-10);
}

function main() {
  const path = process.argv[2];
  if (!path) {
    process.stderr.write("usage: node reference_oracle.mjs <capture-cases.json>\n");
    process.exitCode = 2;
    return;
  }
  const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
  if (corpus.schema !== "luminous.orbital-event-capture.corpus.v0.1"
      || corpus.evidence_class !== "synthetic"
      || corpus.claim_ceiling !== "synthetic-capture-reconciliation-only"
      || !Array.isArray(corpus.cases) || corpus.cases.length === 0) {
    process.stderr.write("unsupported capture corpus contract\n");
    process.exitCode = 2;
    return;
  }

  const ids = new Set();
  let mismatches = 0;
  const counts = Object.create(null);
  for (const c of corpus.cases) {
    if (typeof c.case_id !== "string" || !c.case_id.trim() || ids.has(c.case_id)) {
      process.stderr.write("capture case IDs must be unique and non-empty\n");
      process.exitCode = 2;
      return;
    }
    ids.add(c.case_id);
    const actual = evaluateCase(c);
    const expected = c.expected;
    const matched = actual.disposition === expected.disposition
      && actual.injected_count === expected.injected_count
      && actual.captured_count === expected.captured_count
      && actual.missed_count === expected.missed_count
      && actual.duplicate_count === expected.duplicate_count
      && actual.unexpected_count === expected.unexpected_count
      && close(actual.upper_miss_probability_95, expected.upper_miss_probability_95);
    if (!matched) {
      mismatches += 1;
      process.stderr.write(c.case_id + ": oracle mismatch\n");
    }
    counts[actual.disposition] = (counts[actual.disposition] || 0) + 1;
  }

  const result = {
    oracle: "independent-javascript-event-capture-reconciliation-v0.1",
    case_count: corpus.cases.length,
    expected_result_mismatches: mismatches,
    dispositions: counts,
    physical_completeness_claim_granted: false,
    authority_granted: false
  };
  process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  if (mismatches !== 0) process.exitCode = 1;
}

main();
