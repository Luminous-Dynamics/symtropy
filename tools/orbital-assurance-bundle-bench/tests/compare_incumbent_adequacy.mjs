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

function firstDifferences(left, right, path = "$", output = [], limit = 30) {
  if (output.length >= limit) return output;
  if (Object.is(left, right)) return output;
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right)) {
      output.push({ path, rust: left, oracle: right });
      return output;
    }
    if (left.length !== right.length) {
      output.push({ path: path + ".length", rust: left.length, oracle: right.length });
    }
    for (let index = 0; index < Math.min(left.length, right.length) && output.length < limit; index += 1) {
      firstDifferences(left[index], right[index], path + "[" + index + "]", output, limit);
    }
    return output;
  }
  if (left && right && typeof left === "object" && typeof right === "object") {
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (output.length >= limit) break;
      if (!(key in left) || !(key in right)) {
        output.push({ path: path + "." + key, rust: left[key], oracle: right[key] });
      } else {
        firstDifferences(left[key], right[key], path + "." + key, output, limit);
      }
    }
    return output;
  }
  output.push({ path, rust: left, oracle: right });
  return output;
}

const rustEvaluation = canonicalize(receipt.evaluation);
const oracleEvaluation = canonicalize(oracle);
if (JSON.stringify(rustEvaluation) !== JSON.stringify(oracleEvaluation)) {
  const differences = firstDifferences(rustEvaluation, oracleEvaluation);
  process.stderr.write("incumbent adequacy Rust/JS oracle mismatch; first differing paths:\\n");
  process.stderr.write(JSON.stringify(differences, null, 2) + "\\n");
  throw new Error("incumbent adequacy Rust/JS oracle mismatch");
}
if (receipt.result !== "pass" || receipt.authority_granted !== false) {
  throw new Error("incumbent adequacy receipt failed or exceeded its claim ceiling");
}
console.log("incumbent-adequacy-rust-oracle-match");
