import fs from "node:fs";
import crypto from "node:crypto";
const file=process.argv[2]; if(!file) throw Error("usage: coalition_consequence_oracle.mjs <corpus.json>");
const c=JSON.parse(fs.readFileSync(file,"utf8"));
const h=x=>crypto.createHash("sha256").update(JSON.stringify(x)).digest("hex");
const hex=x=>crypto.createHash("sha256").update(x).digest("hex");
const sort=x=>[...x].sort();
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const assert=(v,m)=>{if(!v)throw Error(m)};
const gcd=(a,b)=>{while(b){[a,b]=[b,a%b]}return a||1n};
function modelDigest(m){
 const x=structuredClone(m);
 x.worlds.forEach(w=>w.profile_tags.sort()); x.worlds.sort((a,b)=>a.world_id.localeCompare(b.world_id));
 x.producer_models.sort((a,b)=>a.model_id.localeCompare(b.model_id)); x.declared_model_ids.sort();
 x.applicability.sort((a,b)=>(a.world_id+"|"+a.model_id).localeCompare(b.world_id+"|"+b.model_id));
 x.joint_pairs.sort((a,b)=>(a.scenario_id+"|"+a.world_id+"|"+a.model_id).localeCompare(b.scenario_id+"|"+b.world_id+"|"+b.model_id));
 for(const k of ["missing_model_ids","missing_pair_ids","unexpected_model_ids","unexpected_pair_ids"])x.expected[k].sort();
 return x;
}
function snapshotPayload(r){return {snapshot_id:r.snapshot_id,receipt_schema:r.receipt_schema,source_commit:r.source_commit,source_receipt_id:r.source_receipt_id,source_receipt_sha256:r.source_receipt_sha256,action_set_sha256:r.action_set_sha256,catalog_revision_sha256:r.catalog_revision_sha256,event_definition_sha256:r.event_definition_sha256,event_inventory_sha256:r.event_inventory_sha256,physical_world_inventory_sha256:r.physical_world_inventory_sha256,full_catalog_state:r.full_catalog_state,screening_state:r.screening_state,threshold_numerator:r.threshold_numerator,threshold_denominator:r.threshold_denominator}}
function portfolio(ids){const action_ids=sort(ids);const action_set_sha256=h({action_ids});return {portfolio_id:"portfolio-"+action_set_sha256,action_ids,action_set_sha256}}
function run(x){
 const b=x.incumbent_bridge_case,m=x.loss_model_universe_case,lm=b.loss_model_set;
 assert(b.scope_sha256===x.scope_sha256 && b.loss_unit===x.loss_unit,"bridge scope/unit mismatch");
 assert(h(sort(b.expected_world_ids))===b.expected_world_ids_sha256,"world digest mismatch");
 assert(h(sort(lm.expected_scenario_ids))===lm.expected_scenario_ids_sha256,"scenario manifest digest mismatch");
 const worlds=b.worlds.map(w=>({...w,feasible_action_ids:sort(w.feasible_action_ids),losses:[...w.losses].sort((a,b)=>a.action_id.localeCompare(b.action_id)||a.source_numerator-b.source_numerator||a.source_denominator-b.source_denominator||a.loss-b.loss)})).sort((a,b)=>a.scenario_id.localeCompare(b.scenario_id));
 assert(h(worlds)===lm.projected_scenarios_sha256,"projected source scenario digest mismatch");
 assert(same(sort(worlds.map(w=>w.scenario_id)),sort(lm.expected_scenario_ids)),"source scenario inventory mismatch");
 assert(same(sort([...new Set(worlds.map(w=>w.world_id))]),sort(b.expected_world_ids)),"source world inventory mismatch");
 assert(worlds.every(w=>w.evidence_status==="admissible"&&w.loss_unit===x.loss_unit&&w.scope_sha256===x.scope_sha256),"joint evidence not admissible");
 const req=m.mission_class==="orbital_vehicle"?[["propellant-v1","propellant_consumption",null],["thermal-v1","thermal_stress","thermal"],["propulsion-v1","propulsion_degradation","propulsion"],["avionics-v1","avionics_reliability",null]]:null;
 assert(req,"oracle fixture mission class unsupported");
 assert(same(sort(m.producer_models.map(z=>z.model_id)),sort(req.map(z=>z[0])))&&same(sort(m.declared_model_ids),sort(req.map(z=>z[0]))),"model grammar inventory mismatch");
 const worldIds=new Set(m.worlds.map(w=>w.world_id)); const expectedPairs=[];
 for(const w of m.worlds)for(const [id,family,tag] of req){const required=tag===null||w.profile_tags.includes(tag);const row=m.applicability.find(a=>a.world_id===w.world_id&&a.model_id===id);assert(row,"missing applicability");assert(required?row.declared_applicable:(!row.declared_applicable&&row.exclusion_reason==="profile_not_applicable"&&row.evidence_receipt_id),"invalid applicability/exclusion");if(required)expectedPairs.push(w.world_id+"|"+id)}
 const actualPairs=m.joint_pairs.map(p=>p.world_id+"|"+p.model_id);assert(same(sort(expectedPairs),sort(actualPairs)),"model universe pairs mismatch");
 assert(m.worlds.every(w=>worldIds.has(w.world_id)&&w.scope_sha256===x.scope_sha256),"model world mismatch");
 const candidates=[...x.candidate_actions].sort((a,b)=>a.action_id.localeCompare(b.action_id));assert(candidates.length>0&&candidates.length<=16,"action limit");
 const expected=[];
 for(let mask=0;mask<(1<<candidates.length);mask++){let ids=[],cost=0,ok=true;for(let i=0;i<candidates.length;i++)if(mask&(1<<i)){const a=candidates[i];if(!a.actionable){ok=false;break}cost+=a.resource_cost;ids.push(a.action_id)}if(ok&&cost<=x.resource_capacity)expected.push(portfolio(ids))}
 const byId=new Map(expected.map(p=>[p.portfolio_id,p]));assert(x.coalitions.length===expected.length,"coalition count mismatch");
 const rec=new Map();let catalog,event,world,threshold;
 for(const row of x.coalitions){const p=portfolio(row.action_ids);assert(same(p.action_ids,row.action_ids)&&p.portfolio_id===row.portfolio_id&&p.action_set_sha256===row.action_set_sha256,"noncanonical portfolio");assert(byId.has(p.portfolio_id)&&!rec.has(p.portfolio_id),"unknown/duplicate portfolio");rec.set(p.portfolio_id,row);const r=row.risk_snapshot;assert(r.receipt_schema==="luminous.orbital-post-action-risk-snapshot.receipt.v0.1"&&r.full_catalog_state==="COMPLETE"&&r.action_set_sha256===p.action_set_sha256&&r.physical_world_inventory_sha256===h(sort(b.expected_world_ids)),"risk snapshot identity mismatch");assert(h(snapshotPayload(r))===r.snapshot_sha256,"risk snapshot digest mismatch");assert(["BELOW_THRESHOLD","THRESHOLD_EXCEEDED"].includes(r.screening_state),"risk result ambiguous");const t=r.threshold_numerator+"/"+r.threshold_denominator;assert(catalog===undefined||catalog===r.catalog_revision_sha256,"catalog mismatch");assert(event===undefined||event===r.event_definition_sha256,"event definition mismatch");assert(world===undefined||world===r.physical_world_inventory_sha256,"world inventory mismatch");assert(threshold===undefined||threshold===t,"threshold mismatch");catalog=r.catalog_revision_sha256;event=r.event_definition_sha256;world=r.physical_world_inventory_sha256;threshold=t}
 assert(rec.size===byId.size,"missing coalition record");
 const ids=sort(expected.map(p=>p.portfolio_id));assert(same(sort(b.action_ids),ids),"bridge portfolio set mismatch");
 const base=expected.find(p=>p.action_ids.length===0).portfolio_id;assert(b.incumbent_action_id===base,"baseline portfolio mismatch");
 const loss=new Map();for(const w of worlds){const map=new Map(w.losses.map(z=>[z.action_id,z.loss]));assert(map.size===ids.length&&same(sort(map.keys()),ids),"incomplete direct portfolio loss vector");loss.set(w.scenario_id,map)}
 const common=expected.map(p=>p.portfolio_id).filter(id=>worlds.every(w=>w.feasible_action_ids.includes(id)));assert(common.includes(base),"baseline infeasible");
 const worst=id=>Math.max(...worlds.map(w=>loss.get(w.scenario_id).get(id)));
 const ref=common.map(id=>({id,loss:worst(id)})).sort((a,b)=>a.loss-b.loss||a.id.localeCompare(b.id))[0];
 const improvement=worst(base)-ref.loss,cut=b.materiality_margin+b.switching_cost;
 const bridgeDisposition=improvement>cut?"REOPTIMIZATION_WARRANTED":"RETAIN_INCUMBENT";
 const bridgeReason=improvement>cut?"COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD":"WITHIN_FINITE_WORLD_RETENTION_ENVELOPE";
 assert(b.expected.disposition===bridgeDisposition&&b.expected.trigger_reason===bridgeReason&&b.expected.worst_case_improvement===improvement,"common-action comparator mismatch");
 const safe=expected.map(p=>p.portfolio_id).filter(id=>rec.get(id).risk_snapshot.screening_state==="BELOW_THRESHOLD");assert(safe.length>0,"no safe portfolio");
 const scenarioIds=sort([...loss.keys()]);
 const portfolios=expected.map(p=>{let lower=Number.MAX_SAFE_INTEGER,worstLoss=Number.MIN_SAFE_INTEGER;for(const s of scenarioIds){const row=loss.get(s);lower=Math.min(lower,row.get(base)-row.get(p.portfolio_id));worstLoss=Math.max(worstLoss,row.get(p.portfolio_id))}const isSafe=safe.includes(p.portfolio_id);const regret=isSafe?Math.max(...scenarioIds.map(s=>{const row=loss.get(s);return row.get(p.portfolio_id)-Math.min(...safe.map(id=>row.get(id)))})):null;return {portfolio_id:p.portfolio_id,lower_bound_avoided_ticks:lower,worst_absolute_loss_ticks:worstLoss,worst_scenario_regret_ticks:regret}}).sort((a,b)=>a.portfolio_id.localeCompare(b.portfolio_id));
 assert(same(portfolios,x.expected.portfolios),"portfolio expected values mismatch");
 assert(x.expected.bridge_worst_case_improvement===improvement&&x.expected.bridge_disposition===bridgeDisposition&&x.expected.bridge_trigger_reason===bridgeReason,"bridge expectation mismatch");
 return {case_id:x.case_id,model_universe_case_sha256:h(modelDigest(m)),scenario_inventory_sha256:h(scenarioIds),bridge_worst_case_improvement:improvement,portfolios};
}
assert(c.schema==="luminous.orbital-coalition-consequence-binding.corpus.v0.1"&&c.evidence_class==="synthetic"&&c.claim_ceiling==="synthetic-finite-joint-scenario-coalition-binding-only","corpus header mismatch");
const cases=c.cases.map(run);
process.stdout.write(JSON.stringify({result:"pass",case_count:cases.length,cases},null,2)+"\n");
