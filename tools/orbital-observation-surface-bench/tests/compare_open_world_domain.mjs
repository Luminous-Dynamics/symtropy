import fs from "node:fs";

const [receiptPath, oraclePath] = process.argv.slice(2);
if (!receiptPath || !oraclePath) {
  throw new Error("usage: compare_open_world_domain.mjs <receipt.json> <oracle.json>");
}
const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
const oracle = JSON.parse(fs.readFileSync(oraclePath, "utf8"));
function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonicalize(value[key])]));
  }
  return value;
}
const expected = canonicalize(oracle);
const actual = canonicalize({
  result: receipt.result,
  evidence_class: receipt.evidence_class,
  claim_ceiling: receipt.claim_ceiling,
  summary: receipt.summary,
  cases: receipt.cases,
  physical_qualification_granted: receipt.physical_qualification_granted,
  authority_granted: receipt.authority_granted
});
if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error("open-world domain Rust/Node oracle mismatch");
}
if (receipt.physical_qualification_granted !== false || receipt.authority_granted !== false) {
  throw new Error("open-world domain receipt exceeded its claim ceiling");
}
console.log("open-world-domain-rust-oracle-match");
