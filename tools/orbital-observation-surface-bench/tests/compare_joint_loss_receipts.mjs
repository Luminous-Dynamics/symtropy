import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message) { throw new Error(message); }
try {
  const [receiptPath, oraclePath] = process.argv.slice(2);
  if (!receiptPath || !oraclePath) fail("usage: compare_joint_loss_receipts.mjs <rust-receipt.json> <oracle.json>");
  const receipt = JSON.parse(fs.readFileSync(receiptPath,"utf8"));
  const oracle = JSON.parse(fs.readFileSync(oraclePath,"utf8"));
  if (receipt.receipt_schema !== "luminous.orbital-joint-loss-scenario.receipt.v0.1") fail("unexpected joint-loss receipt schema");
  if (!/^[0-9a-f]{40}$/.test(receipt.source_commit ?? "")) fail("receipt is not pinned to a full source commit");
  if (!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256 ?? "")) fail("receipt lacks corpus SHA-256");
  for (const field of ["result","evidence_class","claim_ceiling","summary","cases","physical_qualification_granted","authority_granted"]) {
    assert.deepStrictEqual(receipt[field],oracle[field],"Rust/JavaScript mismatch in "+field);
  }
  if (receipt.physical_qualification_granted !== false || receipt.authority_granted !== false) fail("receipt attempted to grant qualification or authority");
  const correlated = receipt.cases.find(item=>item.case_id==="correlated-common-loss-shift");
  const alpha = correlated?.action_metrics?.find(item=>item.action_id==="alpha");
  if (alpha?.worst_case_joint_regret?.numerator !== 0 ||
      alpha?.worst_case_rectangular_regret?.numerator !== 1 ||
      alpha?.worst_case_rectangular_regret?.denominator !== 2) {
    fail("correlation negative control failed: expected joint regret 0 and rectangular regret 1/2");
  }
  const multiWorld = receipt.cases.find(item=>item.case_id==="multi-world-strata");
  const multiAlpha = multiWorld?.action_metrics?.find(item=>item.action_id==="alpha");
  if (multiWorld?.compatible_world_count !== 2 ||
      multiAlpha?.worst_rectangular_regret_worlds?.join(",") !== "w1") {
    fail("multi-world negative control failed: rectangular maximum must identify w1 for alpha");
  }
  const unqualified = receipt.cases.find(item=>item.case_id==="unqualified-loss-model");
  if (unqualified?.status !== "LOSS_MODEL_INDETERMINATE" || unqualified.action_metrics.length !== 0) {
    fail("unqualified loss scenario was not fail-closed");
  }
  process.stdout.write("joint-loss-rust-oracle-match: exact joint/rectangular regret outputs agree\n");
} catch(error) {
  process.stderr.write(String(error?.message ?? error)+"\n");
  process.exitCode = 1;
}
