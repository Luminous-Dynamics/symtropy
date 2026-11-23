import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message){throw new Error(message);}
try{
 const [receiptPath,oraclePath]=process.argv.slice(2);
 if(!receiptPath||!oraclePath) fail("usage: compare_assurance_budget_receipts.mjs <receipt.json> <oracle.json>");
 const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8"));
 const oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
 if(receipt.receipt_schema!=="luminous.orbital-assurance-budget.receipt.v0.1") fail("unexpected assurance budget receipt schema");
 if(!/^[0-9a-f]{40}$/.test(receipt.source_commit??"")) fail("receipt lacks full source commit");
 if(!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256??"")) fail("receipt lacks corpus SHA-256");
 for(const key of ["result","evidence_class","claim_ceiling","assurance_budget_id","requested_scope","requested_scope_id","status",
  "total_alpha","alpha_allocated","alpha_used","alpha_remaining","joint_lower_confidence","missing_required_classes",
  "noncomposable_claim_ids","claim_reports","independence_assumed","physical_qualification_granted","authority_granted"])
  assert.deepStrictEqual(receipt[key],oracle[key],"Rust/JavaScript mismatch in "+key);
 if(receipt.physical_qualification_granted!==false||receipt.authority_granted!==false||receipt.independence_assumed!==false) fail("receipt overstated authority or independence");
 if(receipt.assurance_budget_id==="budget-fixed"&&
   (receipt.status!=="ASSURANCE_COMPOSED_FIXED_EVALUATION"||
    receipt.alpha_used?.numerator!==1||receipt.alpha_used?.denominator!==20||
    receipt.joint_lower_confidence?.numerator!==19||receipt.joint_lower_confidence?.denominator!==20)) fail("fixed union-bound control failed");
 if(receipt.assurance_budget_id==="budget-overrun"&&
   (receipt.status!=="ASSURANCE_BUDGET_EXCEEDED"||receipt.joint_lower_confidence!==null)) fail("budget overrun control failed");
 if(receipt.assurance_budget_id==="budget-missing-loss"&&
   (receipt.status!=="ASSURANCE_INDETERMINATE"||!receipt.missing_required_classes.includes("LOSS_MODEL_COVERAGE"))) fail("missing layer control failed");
 if(receipt.assurance_budget_id==="budget-fixed-as-anytime"&&receipt.status!=="ASSURANCE_SCOPE_MISMATCH") fail("fixed-time claim passed anytime gate");
 if(receipt.assurance_budget_id==="budget-anytime"&&receipt.status!=="ASSURANCE_COMPOSED_ANYTIME") fail("anytime claim failed anytime gate");
 process.stdout.write("assurance-budget-rust-oracle-match: exact union-bound composition agrees\n");
}catch(error){process.stderr.write(String(error?.message??error)+"\n");process.exitCode=1;}
