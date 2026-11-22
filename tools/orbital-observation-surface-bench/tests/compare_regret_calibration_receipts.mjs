import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message){throw new Error(message);}
try{
  const [receiptPath,oraclePath]=process.argv.slice(2);
  if(!receiptPath||!oraclePath) fail("usage: compare_regret_calibration_receipts.mjs <receipt.json> <oracle.json>");
  const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8"));
  const oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
  if(receipt.receipt_schema!=="luminous.orbital-regret-calibration.receipt.v0.1") fail("unexpected calibration receipt schema");
  if(!/^[0-9a-f]{40}$/.test(receipt.source_commit??"")) fail("receipt not pinned to full source commit");
  if(!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256??"")) fail("receipt lacks corpus SHA-256");
  for(const field of ["result","evidence_class","claim_ceiling","calibration_split_id","score_contract_id","alpha","regime_count","total_sample_count","global_status","worst_regime_threshold","worst_regime_ids","regimes_without_finite_nontrivial_threshold","regime_results","coverage_claim","physical_qualification_granted","authority_granted"]){
    assert.deepStrictEqual(receipt[field],oracle[field],"Rust/JavaScript mismatch in "+field);
  }
  if(receipt.physical_qualification_granted!==false||receipt.authority_granted!==false) fail("calibration receipt attempted to grant qualification or authority");
  if(receipt.calibration_split_id==="split-multi-regime" &&
     (receipt.global_status!=="WORST_REGIME_EMPIRICAL_THRESHOLD"||
      receipt.worst_regime_threshold?.numerator!==8||
      receipt.worst_regime_threshold?.denominator!==10||
      receipt.worst_regime_ids.join(",")!=="dry")) fail("multi-regime worst-threshold control failed");
  if(receipt.calibration_split_id==="split-small-no-support" &&
     (receipt.global_status!=="NO_FINITE_NONTRIVIAL_THRESHOLD"||
      receipt.worst_regime_threshold!==null||
      receipt.regimes_without_finite_nontrivial_threshold.join(",")!=="sparse")) fail("rank n+1 must not substitute largest observed score");
  if(receipt.calibration_split_id==="split-small-support-bound" &&
     (receipt.global_status!=="WORST_REGIME_SUPPORT_BOUND_ONLY"||
      receipt.worst_regime_threshold?.numerator!==1||
      receipt.worst_regime_threshold?.denominator!==1)) fail("support-bound-only control failed");
  process.stdout.write("regret-calibration-rust-oracle-match: exact ranks and thresholds agree\n");
}catch(error){
  process.stderr.write(String(error?.message??error)+"\n");
  process.exitCode=1;
}
