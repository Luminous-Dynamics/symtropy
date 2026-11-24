import assert from "node:assert/strict";
import fs from "node:fs";

const [receiptPath, expectedPath] = process.argv.slice(2);
if (!receiptPath || !expectedPath) {
  throw new Error("usage: compare_incumbent_policy_adequacy.mjs <receipt.json> <expected.json>");
}
const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
const expected = JSON.parse(fs.readFileSync(expectedPath, "utf8"));

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonicalize(value[key])]));
  }
  return value;
}

assert.equal(expected.schema, "luminous.orbital-incumbent-policy-adequacy.expected.v0.1");
assert.equal(expected.evidence_class, "synthetic");
assert.equal(receipt.receipt_schema, "luminous.orbital-incumbent-policy-adequacy.receipt.v0.1");
assert.match(receipt.source_commit, /^[0-9a-f]{40}$/);
assert.match(receipt.fixture_sha256, /^[0-9a-f]{64}$/);
assert.equal(receipt.result, "pass");
assert.equal(receipt.physical_qualification_granted, false);
assert.equal(receipt.authority_granted, false);
assert.equal(receipt.evaluation.claim_ceiling, "synthetic-observable-policy-maintenance-algebra-only");
assert.equal(receipt.evaluation.authority_granted, false);
assert.equal(receipt.evaluation.summary.case_count, expected.cases.length);
assert.equal(receipt.evaluation.summary.authority_grants, 0);
for (const expectedCase of expected.cases) {
  const actual = receipt.evaluation.cases.find(item => item.case_id === expectedCase.case_id);
  assert.ok(actual, "missing case " + expectedCase.case_id);
  for (const [field, value] of Object.entries(expectedCase)) {
    assert.deepEqual(canonicalize(actual[field]), canonicalize(value),
      expectedCase.case_id + "." + field);
  }
}
process.stdout.write("incumbent-policy-adequacy-known-answer-match: "
  + expected.cases.length + " case(s); no authority\\n");
