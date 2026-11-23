import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message){throw new Error(message);}
try{
  const [receiptPath,oraclePath]=process.argv.slice(2);
  if(!receiptPath||!oraclePath) fail("usage: compare_feasibility_regret_receipts.mjs <receipt.json> <oracle.json>");
  const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8"));
  const oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
  if(receipt.receipt_schema!=="luminous.orbital-feasibility-regret.receipt.v0.1") fail("unexpected feasibility-regret receipt schema");
  if(!/^[0-9a-f]{40}$/.test(receipt.source_commit??"")) fail("receipt lacks full source commit");
  if(!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256??"")) fail("receipt lacks corpus SHA-256");
  for(const field of ["result","evidence_class","claim_ceiling","summary","cases","physical_qualification_granted","authority_granted"])
    assert.deepStrictEqual(receipt[field],oracle[field],"Rust/JavaScript mismatch in "+field);
  if(receipt.physical_qualification_granted!==false||receipt.authority_granted!==false) fail("receipt attempted to grant qualification or authority");
  const comparatorCase=receipt.cases.find(x=>x.case_id==="infeasible-zero-loss-comparator");
  if(comparatorCase?.robust_feasible_action_ids.join(",")!=="robust") fail("infeasible zero-loss negative control: only robust must remain in intersection");
  const robustMetric=comparatorCase?.action_metrics.find(x=>x.action_id==="robust");
  if(robustMetric?.worst_case_wait_and_see_regret?.numerator!==2||
     robustMetric?.worst_case_wait_and_see_regret?.denominator!==5||
     robustMetric?.worst_case_robust_set_regret?.numerator!==0) fail("infeasible zero-loss action contaminated scenario comparator");
  const disjoint=receipt.cases.find(x=>x.case_id==="disjoint-feasible-sets");
  if(disjoint?.status!=="NO_ROBUST_FEASIBLE_ACTION"||disjoint.action_metrics.length!==0) fail("disjoint feasible sets must not create a fallback action");
  const open=receipt.cases.find(x=>x.case_id==="open-action-universe");
  if(open?.status!=="ACTION_UNIVERSE_OPEN"||open.action_metrics.length!==0) fail("open action universe produced a global regret frontier");
  process.stdout.write("feasibility-regret-rust-oracle-match: exact feasible sets and regret agree\n");
}catch(error){process.stderr.write(String(error?.message??error)+"\n");process.exitCode=1;}
