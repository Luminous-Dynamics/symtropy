import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message){throw new Error(message);}
try{
 const [receiptPath,oraclePath]=process.argv.slice(2);
 if(!receiptPath||!oraclePath)fail("usage: compare_feasibility_discovery_receipts.mjs <receipt.json> <oracle.json>");
 const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8"));
 const oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
 if(receipt.receipt_schema!=="luminous.orbital-feasibility-discovery.receipt.v0.1")fail("unexpected receipt schema");
 if(!/^[0-9a-f]{40}$/.test(receipt.source_commit??"")||!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256??""))fail("receipt lacks source/corpus digest");
 for(const field of ["result","evidence_class","claim_ceiling","summary","cases","physical_qualification_granted","authority_granted"])
  assert.deepStrictEqual(receipt[field],oracle[field],"Rust/JavaScript mismatch in "+field);
 if(receipt.physical_qualification_granted!==false||receipt.authority_granted!==false)fail("receipt attempted to grant qualification or authority");
 const resolved=receipt.cases.find(c=>c.case_id==="query-resolves-disjoint-worlds");
 if(resolved?.status!=="QUERY_CONDITIONED_POLICIES_AVAILABLE_NO_STATIC_BASELINE"||resolved.no_query_policy_ids.length!==0)fail("query should resolve disjoint static feasible sets");
 const safe=resolved.query_reports.find(q=>q.query_id==="safe-identifying-query");
 if(safe?.generated_policy_count!==1)fail("safe identifying query should have exactly one finite policy");
 const policy=resolved.policy_metrics.find(p=>p.query_id==="safe-identifying-query");
 if(policy?.branch_actions?.north!=="north_action"||policy?.branch_actions?.south!=="south_action")fail("safe query policy branches on wrong actions");
 if(policy?.worst_case_loss?.numerator!==3||policy?.worst_case_loss?.denominator!==10)fail("safe query policy worst loss mismatch");
 const blind=resolved.query_reports.find(q=>q.query_id==="blind-query");
 if(blind?.status!=="NO_SAFE_POLICY_FOR_OUTCOME"||blind.generated_policy_count!==0)fail("blind query must not branch on hidden world IDs");
 const prohibited=resolved.query_reports.find(q=>q.query_id==="prohibited-query");
 if(prohibited?.status!=="QUERY_PROHIBITED"||prohibited.policy_ids.length!==0)fail("inadmissible query was allowed");
 const dominated=receipt.cases.find(c=>c.case_id==="no_query_dominates_paid_query");
 const queryPolicies=dominated.policy_metrics.filter(p=>p.query_id==="redundant-query").map(p=>p.policy_id);
 if(!queryPolicies.length||queryPolicies.some(id=>dominated.policy_pareto_frontier_ids.includes(id)))fail("positive-cost redundant query was not dominated by no-query baseline");
 process.stdout.write("feasibility-discovery-rust-oracle-match: exact non-anticipative policy frontier agrees\n");
}catch(error){process.stderr.write(String(error?.message??error)+"\n");process.exitCode=1;}
