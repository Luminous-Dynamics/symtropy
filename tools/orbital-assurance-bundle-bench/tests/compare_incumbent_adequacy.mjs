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
function firstDifference(left, right, path = "$") {
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right)) {
      return { path, rust: left, oracle: right };
    }
    if (left.length !== right.length) {
      return { path: path + ".length", rust: left.length, oracle: right.length };
    }
    for (let index = 0; index < left.length; index += 1) {
      const difference = firstDifference(left[index], right[index], path + "[" + index + "]");
      if (difference) return difference;
    }
    return null;
  }
  const leftObject = left !== null && typeof left === "object";
  const rightObject = right !== null && typeof right === "object";
  if (leftObject || rightObject) {
    if (!leftObject || !rightObject) return { path, rust: left, oracle: right };
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (!(key in left) || !(key in right)) {
        return { path: path + "." + key, rust: left[key], oracle: right[key] };
      }
      const difference = firstDifference(left[key], right[key], path + "." + key);
      if (difference) return difference;
    }
    return null;
  }
  return Object.is(left, right) ? null : { path, rust: left, oracle: right };
}
const difference = firstDifference(canonicalize(receipt.evaluation), canonicalize(oracle));
if (difference) {
  throw new Error("incumbent adequacy Rust/JS oracle mismatch: " + JSON.stringify(difference));
}
if (receipt.result !== "pass" || receipt.authority_granted !== false) {
  throw new Error("incumbent adequacy receipt failed or exceeded its claim ceiling");
}
console.log("incumbent-adequacy-rust-oracle-match");
