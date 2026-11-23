import fs from "node:fs";

const [receiptPath, oraclePath] = process.argv.slice(2);
if (!receiptPath || !oraclePath) {
  throw new Error("usage: compare_incumbent_adequacy.mjs <receipt.json> <oracle.json>");
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
if (JSON.stringify(canonicalize(receipt.evaluation)) !== JSON.stringify(canonicalize(oracle))) {
  throw new Error("incumbent adequacy Rust/JS oracle mismatch");
}
if (receipt.result !== "pass" || receipt.authority_granted !== false) {
  throw new Error("incumbent adequacy receipt failed or exceeded its claim ceiling");
}
console.log("incumbent-adequacy-rust-oracle-match");
