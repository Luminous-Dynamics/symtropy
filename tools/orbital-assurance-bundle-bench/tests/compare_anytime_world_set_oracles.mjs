import assert from "node:assert/strict";
import fs from "node:fs";

const [nodePath, pythonPath] = process.argv.slice(2);
if (!nodePath || !pythonPath) {
  throw new Error("usage: compare_anytime_world_set_oracles.mjs <node-report.json> <python-report.json>");
}
const nodeReport = JSON.parse(fs.readFileSync(nodePath, "utf8"));
const pythonReport = JSON.parse(fs.readFileSync(pythonPath, "utf8"));

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonicalize(value[key])]));
  }
  return value;
}

for (const [label, report] of [["Node", nodeReport], ["Python", pythonReport]]) {
  assert.equal(report.result, "pass", label + " oracle result");
  assert.equal(report.case_count, 26, label + " oracle case count");
  assert.equal(report.expected_result_mismatches, 0, label + " oracle mismatch count");
  assert.equal(report.authority_grants, 0, label + " authority grants");
}
assert.deepEqual(
  canonicalize(nodeReport.cases),
  canonicalize(pythonReport.cases),
  "Node BigInt and Python Fraction per-case results must match exactly"
);
process.stdout.write("cross-language-incumbent-world-set-oracle-match: 26 cases; Node BigInt == Python Fraction; zero mismatches; zero authority\n");
