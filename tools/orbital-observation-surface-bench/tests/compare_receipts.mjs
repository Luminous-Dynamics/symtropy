import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";

const receiptPath = process.argv[2];
const oraclePath = process.argv[3];
const observationsPath = process.argv[4];
if (!receiptPath || !oraclePath || !observationsPath) {
  throw new Error("usage: compare_receipts.mjs <receipt.json> <oracle.json> <observations.json>");
}
const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
const oracle = JSON.parse(fs.readFileSync(oraclePath, "utf8"));
const observationsBytes = fs.readFileSync(observationsPath);
const observations = JSON.parse(observationsBytes.toString("utf8"));

assert.equal(receipt.result, "pass");
assert.equal(receipt.physical_reliability_claim_granted, false);
assert.equal(receipt.qualification_granted, false);
assert.equal(receipt.authority_granted, false);
assert.deepEqual(receipt.report.summary, oracle.summary);
assert.equal(receipt.report.scenarios.length, oracle.scenarios.length);
assert.equal(receipt.observations_sha256,
  createHash("sha256").update(observationsBytes).digest("hex"));
assert.equal(observations.schema, "luminous.orbital-observation-surface.observations.v0.1");
assert.equal(observations.evidence_class, "synthetic");
assert.equal(observations.claim_ceiling, "synthetic-observation-stream-only");
assert.equal(observations.source_commit, receipt.source_commit);
assert.equal(observations.corpus_sha256, receipt.corpus_sha256);
assert.equal(observations.scenarios.length, oracle.observations.scenarios.length);

const observationText = observationsBytes.toString("utf8");
for (const forbidden of ["event_id", "miss_reason", "conditional_classifier_probability",
  "false_positive_times", "detected", "audit"]) {
  assert.ok(!observationText.includes(forbidden),
    `declassified observations unexpectedly contain ${forbidden}`);
}

function close(left, right, tolerance = 1e-12) {
  assert.ok(Number.isFinite(left) && Number.isFinite(right));
  assert.ok(Math.abs(left - right) <= tolerance * Math.max(1, Math.abs(right)),
    `numeric mismatch: ${left} vs ${right}`);
}

for (let i = 0; i < oracle.scenarios.length; i++) {
  const expectedScenario = oracle.scenarios[i];
  const actualScenario = receipt.report.scenarios[i];
  const expectedObservations = oracle.observations.scenarios[i];
  const actualObservations = observations.scenarios[i];

  assert.equal(actualScenario.scenario_id, expectedScenario.scenario_id);
  assert.equal(actualScenario.configuration_id, expectedScenario.configuration_id);
  assert.equal(actualScenario.channels.length, expectedScenario.channels.length);
  assert.equal(actualObservations.scenario_id, expectedObservations.scenario_id);
  assert.equal(actualObservations.configuration_id, expectedObservations.configuration_id);
  assert.equal(actualObservations.channels.length, expectedObservations.channels.length);

  for (let j = 0; j < expectedScenario.channels.length; j++) {
    const expected = expectedScenario.channels[j];
    const actual = actualScenario.channels[j];
    const expectedObsChannel = expectedObservations.channels[j];
    const actualObsChannel = actualObservations.channels[j];

    assert.equal(actual.channel_id, expected.channel_id);
    assert.equal(actualObsChannel.channel_id, expectedObsChannel.channel_id);
    assert.equal(actual.observed_event_stream.length, expected.observed_event_stream.length);
    assert.equal(actualObsChannel.events.length, expectedObsChannel.events.length);
    assert.equal(actualObsChannel.events.length, actual.observed_event_stream.length);

    for (let k = 0; k < expected.observed_event_stream.length; k++) {
      close(actual.observed_event_stream[k].time_fraction,
        expected.observed_event_stream[k].time_fraction);
      close(actualObsChannel.events[k].time_fraction,
        expectedObsChannel.events[k].time_fraction);
      close(actualObsChannel.events[k].time_fraction,
        actual.observed_event_stream[k].time_fraction);
    }

    const expectedAudit = expected.audit;
    const actualAudit = actual.audit;
    assert.equal(actualAudit.latent_event_count, expectedAudit.latent_event_count);
    assert.equal(actualAudit.detected_event_count, expectedAudit.detected_event_count);
    assert.deepEqual(actualAudit.miss_counts, expectedAudit.miss_counts);
    assert.equal(actualAudit.false_positive_count, expectedAudit.false_positive_count);
    assert.equal(actualAudit.false_positive_times.length, expectedAudit.false_positive_times.length);
    actualAudit.false_positive_times.forEach((value, k) =>
      close(value, expectedAudit.false_positive_times[k]));
    assert.equal(actualAudit.event_outcomes.length, expectedAudit.event_outcomes.length);
    for (let k = 0; k < expectedAudit.event_outcomes.length; k++) {
      const e = expectedAudit.event_outcomes[k], a = actualAudit.event_outcomes[k];
      assert.equal(a.event_id, e.event_id);
      assert.equal(a.detected, e.detected);
      assert.equal(a.miss_reason, e.miss_reason);
      close(a.conditional_classifier_probability, e.conditional_classifier_probability);
    }
  }
}
process.stdout.write("independent oracle and declassified observation stream agree with Rust receipt\n");
