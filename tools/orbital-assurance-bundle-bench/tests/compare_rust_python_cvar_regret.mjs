import assert from "node:assert/strict";
import fs from "node:fs";

const [rustReceiptPath, pythonReportPath] = process.argv.slice(2);
if (!rustReceiptPath || !pythonReportPath) {
  throw new Error("usage: compare_rust_python_cvar_regret.mjs <rust-receipt.json> <python-report.json>");
}
const receipt = JSON.parse(fs.readFileSync(rustReceiptPath, "utf8"));
const python = JSON.parse(fs.readFileSync(pythonReportPath, "utf8"));

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonicalize(value[key])]));
  }
  return value;
}

assert.equal(receipt.receipt_schema, "luminous.orbital-cvar-regret.receipt.v0.1");
assert.match(receipt.source_commit, /^[0-9a-f]{40}$/);
assert.match(receipt.fixture_sha256, /^[0-9a-f]{64}$/);
assert.equal(receipt.result, "pass");
assert.equal(receipt.probability_qualification_granted, false);
assert.equal(receipt.authority_granted, false);
assert.equal(python.result, "pass");
assert.equal(python.authority_granted, false);
assert.deepEqual(
  canonicalize(receipt.evaluation),
  canonicalize(python),
  "Rust exact CVaR evaluator and Python Fraction oracle must agree field-by-field"
);

const evaluation = receipt.evaluation;
assert.deepEqual(evaluation.summary, {
  case_count: 4,
  pass_cases: 1,
  abstain_cases: 3,
  invalid_input_cases: 0,
  authority_grants: 0
});
const qualified = evaluation.cases.find(item => item.case_id === "cvar-case-1");
assert.equal(qualified.status, "PASS");
const incumbent = qualified.policies.find(item => item.policy_id === "incumbent-static");
const candidate = qualified.policies.find(item => item.policy_id === "candidate-observable");
assert.deepEqual(incumbent.wait_and_see_regret.expected_regret, { numerator: 16, denominator: 25 });
assert.deepEqual(incumbent.wait_and_see_regret.worst_case_regret, { numerator: 10, denominator: 1 });
assert.deepEqual(
  incumbent.wait_and_see_regret.cvar.map(item => item.cvar),
  [{ numerator: 22, denominator: 5 }, { numerator: 29, denominator: 5 }, { numerator: 10, denominator: 1 }]
);
assert.deepEqual(candidate.wait_and_see_regret.expected_regret, { numerator: 19, denominator: 50 });
assert.deepEqual(
  candidate.wait_and_see_regret.cvar.map(item => item.cvar),
  [{ numerator: 14, denominator: 5 }, { numerator: 18, denominator: 5 }, { numerator: 6, denominator: 1 }]
);
assert.equal(evaluation.cases.find(item => item.case_id === "cvar-case-no-probability").status, "ABSTAIN");
assert.equal(evaluation.cases.find(item => item.case_id === "cvar-case-stale-model").status, "ABSTAIN");
assert.equal(evaluation.cases.find(item => item.case_id === "cvar-case-unqualified-calibration").status, "ABSTAIN");
process.stdout.write(
  "rust-python-cvar-parity: exact finite-support CVaR match; 4 cases; 1 qualified, 3 abstained; no authority\n"
);
