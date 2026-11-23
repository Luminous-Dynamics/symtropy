import fs from "node:fs";

const [receiptPath, oraclePath] = process.argv.slice(2);
if (!receiptPath || !oraclePath) throw new Error("usage: compare_dependency_graph_adequacy.mjs <receipt> <oracle>");
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
  throw new Error("dependency graph adequacy Rust/JS oracle mismatch");
}
if (receipt.result !== "pass") throw new Error("dependency graph adequacy receipt failed");
console.log("dependency-graph-adequacy-rust-oracle-match");
