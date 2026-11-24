import assert from "node:assert/strict";
import fs from "node:fs";

const [rustReceiptPath, pythonReportPath] = process.argv.slice(2);
if (!rustReceiptPath || !pythonReportPath) {
  throw new Error("usage: compare_rust_python_policy_adequacy.mjs <rust-receipt.json> <python-report.json>");
}
const rustReceipt = JSON.parse(fs.readFileSync(rustReceiptPath, "utf8"));
const pythonReport = JSON.parse(fs.readFileSync(pythonReportPath, "utf8"));

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value).sort().map(key => [key, canonicalize(value[key])])
    );
  }
  return value;
}

assert.equal(rustReceipt.result, "pass");
assert.equal(rustReceipt.authority_granted, false);
assert.equal(rustReceipt.physical_qualification_granted, false);
assert.equal(pythonReport.result, "pass");
assert.equal(pythonReport.authority_granted, false);
assert.equal(pythonReport.summary.authority_grants, 0);
assert.deepEqual(
  canonicalize(rustReceipt.evaluation),
  canonicalize(pythonReport),
  "Rust evaluator and independent Python Fraction oracle must agree field-by-field"
);
process.stdout.write(
  "rust-python-policy-adequacy-parity: exact evaluation match; Fraction arithmetic; zero authority\n"
);
