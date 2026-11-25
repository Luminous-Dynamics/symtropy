import fs from "node:fs";
const [receiptPath,oraclePath]=process.argv.slice(2);
if(!receiptPath||!oraclePath)throw Error("usage: compare_coalition_consequence.mjs <receipt> <oracle>");
const r=JSON.parse(fs.readFileSync(receiptPath,"utf8")),o=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
const need=(x,m)=>{if(!x)throw Error(m)};
need(r.receipt_schema==="luminous.orbital-coalition-consequence-binding.receipt.v0.1","wrong receipt schema");
need(r.result==="pass"&&r.authority_granted===false&&r.evaluation.authority_granted===false,"Rust receipt failed or authority invariant changed");
need(o.result==="pass"&&o.case_count===r.evaluation.summary.case_count,"oracle corpus mismatch");
need(r.evaluation.summary.expected_result_mismatches===0&&r.evaluation.summary.authority_grants===0,"Rust summary contains mismatch or authority grant");
for(const expected of o.cases){
  const actual=r.evaluation.cases.find(x=>x.case_id===expected.case_id);
  need(actual,"Rust receipt omitted oracle case");
  need(actual.model_universe_case_sha256===expected.model_universe_case_sha256,"model-universe case digest mismatch");
  need(actual.scenario_inventory_sha256===expected.scenario_inventory_sha256,"scenario inventory digest mismatch");
  need(actual.bridge_worst_case_improvement===expected.bridge_worst_case_improvement,"common-action comparator mismatch");
  need(actual.portfolios.length===expected.portfolios.length,"portfolio metric count mismatch");
  for(const p of expected.portfolios){
    const a=actual.portfolios.find(x=>x.portfolio_id===p.portfolio_id);
    need(a&&a.lower_bound_avoided_ticks===p.lower_bound_avoided_ticks&&a.worst_absolute_loss_ticks===p.worst_absolute_loss_ticks&&a.worst_scenario_regret_ticks===p.worst_scenario_regret_ticks,"independent portfolio metric mismatch: "+p.portfolio_id);
  }
}
process.stdout.write("coalition-consequence-rust-oracle-parity\n");
