import fs from "node:fs";
import assert from "node:assert/strict";

function fail(message) { throw new Error(message); }

try {
  const [receiptPath, oraclePath] = process.argv.slice(2);
  if (!receiptPath || !oraclePath) {
    fail("usage: compare_decision_regret_receipts.mjs <rust-receipt.json> <js-oracle.json>");
  }
  const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
  const oracle = JSON.parse(fs.readFileSync(oraclePath, "utf8"));

  if (receipt.receipt_schema !== "luminous.orbital-decision-regret-frontier.receipt.v0.1") {
    fail("unexpected Rust decision-regret receipt schema");
  }
  if (!/^[0-9a-f]{40}$/.test(receipt.source_commit ?? "")) {
    fail("decision-regret receipt is not pinned to a full lowercase source commit");
  }
  if (!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256 ?? "")) {
    fail("decision-regret receipt lacks a SHA-256 corpus digest");
  }
  for (const field of [
    "result",
    "evidence_class",
    "claim_ceiling",
    "summary",
    "cases",
    "physical_qualification_granted",
    "authority_granted"
  ]) {
    assert.deepStrictEqual(receipt[field], oracle[field], "Rust/JavaScript mismatch in " + field);
  }
  if (receipt.physical_qualification_granted !== false || receipt.authority_granted !== false) {
    fail("synthetic decision-regret receipt attempted to grant physical qualification or authority");
  }
  const tradeoff = receipt.cases.find(item => item.case_id === "loss-regret-disagreement");
  if (tradeoff?.minimax_loss_action_ids?.join(",") !== "balanced" ||
      tradeoff?.minimax_regret_action_ids?.join(",") !== "aggressive") {
    fail("negative control failed: minimax loss and minimax regret must remain distinct");
  }
  process.stdout.write("decision-regret-rust-oracle-match: exact loss/regret/frontier outputs agree\n");
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
