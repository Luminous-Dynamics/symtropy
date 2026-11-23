import fs from "node:fs";

const [receiptPath, oraclePath] = process.argv.slice(2);
if (!receiptPath || !oraclePath) {
  throw new Error("usage: compare_action_universe_closure.mjs <receipt.json> <oracle.json>");
}
const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
const oracle = JSON.parse(fs.readFileSync(oraclePath, "utf8"));

function assertEqual(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label} mismatch\nreceipt: ${JSON.stringify(actual)}\noracle:   ${JSON.stringify(expected)}`);
  }
}
assertEqual(receipt.result, oracle.result, "overall result");
assertEqual(receipt.summary, oracle.summary, "summary");
assertEqual(receipt.cases, oracle.cases, "case results");
assertEqual(receipt.physical_qualification_granted, false, "physical qualification boundary");
assertEqual(receipt.authority_granted, false, "authority boundary");
console.log("action-universe-closure-rust-oracle-match");
