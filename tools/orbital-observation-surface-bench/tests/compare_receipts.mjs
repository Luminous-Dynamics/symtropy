import assert from "node:assert/strict";
import fs from "node:fs";

const receipt = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const oracle = JSON.parse(fs.readFileSync(process.argv[3], "utf8"));
assert.equal(receipt.result, "pass");
assert.equal(receipt.physical_reliability_claim_granted, false);
assert.equal(receipt.qualification_granted, false);
assert.equal(receipt.authority_granted, false);
assert.deepEqual(receipt.report.summary, oracle.summary);
assert.equal(receipt.report.scenarios.length, oracle.scenarios.length);

function close(left, right, tolerance = 1e-12) {
  assert.ok(Number.isFinite(left) && Number.isFinite(right));
  assert.ok(Math.abs(left - right) <= tolerance * Math.max(1, Math.abs(right)),
    `numeric mismatch: ${left} vs ${right}`);
}

for (let i = 0; i < oracle.scenarios.length; i++) {
  const expectedScenario = oracle.scenarios[i];
  const actualScenario = receipt.report.scenarios[i];
  assert.equal(actualScenario.scenario_id, expectedScenario.scenario_id);
  assert.equal(actualScenario.configuration_id, expectedScenario.configuration_id);
  assert.equal(actualScenario.channels.length, expectedScenario.channels.length);
  for (let j = 0; j < expectedScenario.channels.length; j++) {
    const expected = expectedScenario.channels[j];
    const actual = actualScenario.channels[j];
    assert.equal(actual.channel_id, expected.channel_id);
    assert.equal(actual.latent_event_count, expected.latent_event_count);
    assert.equal(actual.detected_event_count, expected.detected_event_count);
    assert.deepEqual(actual.miss_counts, expected.miss_counts);
    assert.equal(actual.false_positive_count, expected.false_positive_count);
    assert.equal(actual.false_positive_times.length, expected.false_positive_times.length);
    actual.false_positive_times.forEach((value, k) => close(value, expected.false_positive_times[k]));
    assert.equal(actual.event_outcomes.length, expected.event_outcomes.length);
    for (let k = 0; k < expected.event_outcomes.length; k++) {
      const e = expected.event_outcomes[k], a = actual.event_outcomes[k];
      assert.equal(a.event_id, e.event_id);
      assert.equal(a.detected, e.detected);
      assert.equal(a.miss_reason, e.miss_reason);
      close(a.conditional_classifier_probability, e.conditional_classifier_probability);
    }
  }
}
process.stdout.write("independent observation-surface oracle agrees with Rust receipt\n");
