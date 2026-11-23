// Independent BigInt oracle for finite non-anticipative feasibility-discovery policies.
import fs from "node:fs";
import {createHash} from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE=9_007_199_254_740_991n;
const MAX_POLICIES=4096;
const NO_QUERY_OUTCOME="__NO_QUERY__";
function fail(message){throw new Error(message);}
function strCmp(a,b){
  const x=Array.from(a,v=>v.codePointAt(0)),y=Array.from(b,v=>v.codePointAt(0));
  for(let i=0;i<Math.min(x.length,y.length);i+=1) if(x[i]!==y[i]) return x[i]-y[i];
  return x.length-y.length;
}
function canonical(value){
  if(Array.isArray(value)) return "["+value.map(canonical).join(",")+"]";
  if(value!==null&&typeof value==="object")
    return "{"+Object.keys(value).sort(strCmp).map(k=>JSON.stringify(k)+":"+canonical(value[k])).join(",")+"}";
  return JSON.stringify(value);
}
function sha(value){return createHash("sha256").update(value,"utf8").digest("hex");}
function digestIds(ids){return sha(JSON.stringify([...ids].sort(strCmp)));}
function gcd(a,b){while(b!==0n){const r=a%b;a=b;b=r;}return a>0n?a:1n;}
function rat(value,path){
  if(!Number.isSafeInteger(value?.numerator)||!Number.isSafeInteger(value?.denominator)||
     value.numerator<0||value.denominator<=0||
     BigInt(value.numerator)>MAX_SAFE||BigInt(value.denominator)>MAX_SAFE||
     value.numerator>value.denominator) fail(path+": invalid exact rational");
  const n=BigInt(value.numerator),d=BigInt(value.denominator),g=gcd(n,d);
  return {n:n/g,d:d/g};
}
function cmpR(a,b){const x=a.n*b.d,y=b.n*a.d;return x<y?-1:x>y?1:0;}
function subR(a,b,path){
  const n=a.n*b.d-b.n*a.d,d=a.d*b.d;
  if(n<0n) fail(path+": regret underflow");
  const g=gcd(n,d);return {n:n/g,d:d/g};
}
function jsonR(value){return {numerator:Number(value.n),denominator:Number(value.d)};}
function validId(value){return typeof value==="string"&&value.trim().length>0&&/^[\x00-\x7F]+$/.test(value);}
function validateCost(value,path){
  for(const key of ["reference_trials","exposure_micro_units","elapsed_millis"])
    if(!Number.isSafeInteger(value?.[key])||value[key]<0||BigInt(value[key])>MAX_SAFE) fail(path+": invalid "+key);
}
function addCost(a,b){
  const result={};
  for(const key of ["reference_trials","exposure_micro_units","elapsed_millis"]){
    const x=BigInt(a[key])+BigInt(b[key]);
    if(x>MAX_SAFE) fail("policy cost overflow "+key);
    result[key]=Number(x);
  }
  return result;
}
function maxCost(a,b){
  return {
    reference_trials:Math.max(a.reference_trials,b.reference_trials),
    exposure_micro_units:Math.max(a.exposure_micro_units,b.exposure_micro_units),
    elapsed_millis:Math.max(a.elapsed_millis,b.elapsed_millis)
  };
}
function validateBase(base){
  const worlds=base.compatible_world_set;
  if(!validId(worlds.source_receipt_id)||!Array.isArray(worlds.world_ids)||!worlds.world_ids.length) fail("invalid compatible world set");
  const worldIds=[...worlds.world_ids].sort(strCmp);
  if(worldIds.some(x=>!validId(x))||new Set(worldIds).size!==worldIds.length||worlds.world_ids_sha256!==digestIds(worldIds)) fail("world ID inventory/digest mismatch");
  const universe=base.action_universe;
  if(universe.scope_id!==base.decision_scope_id||!validId(universe.source_receipt_id)||!validId(universe.enumeration_method)) fail("invalid action-universe scope or receipt");
  if(!["finite_exhaustive","open","indeterminate"].includes(universe.closure_kind)||!["admissible","indeterminate","out_of_scope"].includes(universe.evidence_status)) fail("invalid action-universe status");
  const plannedActions=[...universe.expected_action_ids].sort(strCmp);
  if(!plannedActions.length||new Set(plannedActions).size!==plannedActions.length||universe.expected_action_ids_sha256!==digestIds(plannedActions)) fail("planned action inventory/digest mismatch");
  const actions=new Map();
  for(const action of base.actions){
    if(!validId(action.action_id)||actions.has(action.action_id)||!["eligible","prohibited","indeterminate"].includes(action.eligibility_status)) fail("invalid/duplicate action");
    if(!validId(action.eligibility_receipt_id)) fail("action lacks eligibility receipt");
    validateCost(action.cost,"action cost");
    actions.set(action.action_id,action);
  }
  if(JSON.stringify([...actions.keys()].sort(strCmp))!==JSON.stringify(plannedActions)) fail("observed action set differs from planned inventory");

  const manifest=base.scenario_manifest;
  const plannedScenarios=[...manifest.expected_scenario_ids].sort(strCmp);
  if(!validId(manifest.source_manifest_id)||!plannedScenarios.length||new Set(plannedScenarios).size!==plannedScenarios.length||
     manifest.expected_scenario_ids_sha256!==digestIds(plannedScenarios)) fail("planned scenario inventory/digest mismatch");
  const scenarios=[...manifest.scenarios].sort((a,b)=>strCmp(a.scenario_id,b.scenario_id));
  if(JSON.stringify(scenarios.map(x=>x.scenario_id))!==JSON.stringify(plannedScenarios)) fail("observed scenario IDs differ from plan");
  const covered=new Set();
  for(const scenario of scenarios){
    if(!validId(scenario.scenario_id)||!worldIds.includes(scenario.world_id)||!validId(scenario.loss_model_id)||
       !["admissible","indeterminate","out_of_profile"].includes(scenario.evidence_status)) fail("invalid scenario metadata");
    if(scenario.evidence_status==="admissible"&&!validId(scenario.evidence_receipt_id)) fail("scenario lacks evidence receipt");
    covered.add(scenario.world_id);
    const keys=Object.keys(scenario.action_outcomes??{}).sort(strCmp);
    if(JSON.stringify(keys)!==JSON.stringify(plannedActions)) fail("scenario action outcomes are incomplete");
    for(const action of actions.values()){
      const row=scenario.action_outcomes[action.action_id];
      if(action.eligibility_status==="prohibited"){
        if(row.feasibility_status!=="prohibited"||row.loss!==null) fail("prohibited outcome mismatch");
      }else if(action.eligibility_status==="indeterminate"){
        if(row.feasibility_status!=="indeterminate"||row.loss!==null) fail("indeterminate eligibility outcome mismatch");
      }else if(row.feasibility_status==="feasible"){
        if(row.loss===null) fail("feasible action missing loss");rat(row.loss,"loss");
      }else if(row.feasibility_status==="infeasible"||row.feasibility_status==="indeterminate"){
        if(row.loss!==null) fail("non-feasible outcome must have null loss");
      }else fail("invalid scenario feasibility status");
    }
  }
  if(JSON.stringify([...covered].sort(strCmp))!==JSON.stringify(worldIds)) fail("scenario rows do not cover every world");
  if(manifest.scenarios_sha256!==sha(canonical([...manifest.scenarios].sort((a,b)=>strCmp(a.scenario_id,b.scenario_id))))) fail("full scenario manifest digest mismatch");

  let status;
  if(universe.closure_kind==="open") status="ACTION_UNIVERSE_OPEN";
  else if(universe.closure_kind==="indeterminate"||universe.evidence_status!=="admissible") status="ACTION_UNIVERSE_INDETERMINATE";
  else if(base.actions.some(a=>a.eligibility_status==="indeterminate")) status="ACTION_ADMISSIBILITY_INDETERMINATE";
  else if(scenarios.some(s=>s.evidence_status==="out_of_profile")) status="OUT_OF_PROFILE";
  else if(scenarios.some(s=>s.evidence_status!=="admissible")) status="LOSS_MODEL_INDETERMINATE";
  else if(scenarios.some(s=>base.actions.some(a=>a.eligibility_status==="eligible"&&s.action_outcomes[a.action_id].feasibility_status==="indeterminate"))) status="FEASIBILITY_INDETERMINATE";
  else if(scenarios.some(s=>!base.actions.some(a=>a.eligibility_status==="eligible"&&s.action_outcomes[a.action_id].feasibility_status==="feasible"))) status="NO_FEASIBLE_ACTION_IN_SCENARIO";
  else{
    const common=[...actions.values()].filter(a=>a.eligibility_status==="eligible"&&scenarios.every(s=>s.action_outcomes[a.action_id].feasibility_status==="feasible"));
    status=common.length?"ROBUST_FEASIBLE_DECISION_SET_AVAILABLE":"NO_ROBUST_FEASIBLE_ACTION";
  }
  return {worldIds,actions,scenarios,status};
}
function validateQueryUniverse(item,baseInfo){
  const u=item.query_universe;
  if(!validId(u.source_receipt_id)||u.scope_id!==item.base_decision.decision_scope_id||
     !Array.isArray(u.queries)||!u.queries.length||u.queries.length>64||
     !Array.isArray(u.expected_query_ids)||u.expected_query_ids.length!==u.queries.length) fail("invalid query universe");
  const planned=[...u.expected_query_ids].sort(strCmp);
  if(planned.some(x=>!validId(x))||new Set(planned).size!==planned.length||u.expected_query_ids_sha256!==digestIds(planned)) fail("query ID inventory/digest mismatch");
  const observed=new Set();
  for(const query of u.queries){
    if(!validId(query.query_id)||observed.has(query.query_id)||query.query_kind!=="passive_observation"||
       !["admissible","prohibited","indeterminate"].includes(query.admissibility_status)||
       !["admissible","indeterminate","out_of_profile"].includes(query.evidence_status)) fail("invalid query metadata");
    observed.add(query.query_id);
    if(query.admissibility_status==="admissible"&&query.evidence_status==="admissible"&&
       (!validId(query.admissibility_receipt_id)||!validId(query.evidence_receipt_id))) fail("admissible query lacks receipts");
    const expected=[...query.expected_world_ids].sort(strCmp);
    if(JSON.stringify(expected)!==JSON.stringify(baseInfo.worldIds)||query.expected_world_ids_sha256!==digestIds(expected)) fail("query expected-world inventory mismatch");
    const keys=Object.keys(query.world_observations??{}).sort(strCmp);
    if(JSON.stringify(keys)!==JSON.stringify(baseInfo.worldIds)) fail("query observation map does not cover every world");
    for(const obs of Object.values(query.world_observations)) validId(obs)||fail("invalid query outcome ID");
    validateCost(query.cost,"query cost");
  }
  if(JSON.stringify([...observed].sort(strCmp))!==JSON.stringify(planned)) fail("observed queries differ from planned inventory");
  const canonicalQueries=u.queries.map(query=>({
    ...query,
    expected_world_ids:[...query.expected_world_ids].sort(strCmp)
  })).sort((a,b)=>strCmp(a.query_id,b.query_id));
  if(u.queries_sha256!==sha(canonical(canonicalQueries))) fail("full query manifest digest mismatch");
}
function classMap(worldObservations){
  const classes=new Map();
  for(const [world,outcome] of Object.entries(worldObservations)){
    if(!classes.has(outcome)) classes.set(outcome,[]);
    classes.get(outcome).push(world);
  }
  for(const worlds of classes.values()) worlds.sort(strCmp);
  return new Map([...classes.entries()].sort((a,b)=>strCmp(a[0],b[0])));
}
function classScenarios(scenarios,worlds){
  const ids=new Set(worlds);
  return scenarios
    .filter(s=>ids.has(s.world_id))
    .sort((left,right)=>strCmp(left.scenario_id,right.scenario_id));
}
function commonActions(actions,rows){
  if(!rows.length) fail("outcome class lacks scenarios");
  return actions.filter(a=>a.eligibility_status==="eligible"&&rows.every(s=>s.action_outcomes[a.action_id].feasibility_status==="feasible"))
    .map(a=>a.action_id).sort(strCmp);
}
function scenarioBest(scenario){
  const vals=Object.values(scenario.action_outcomes).filter(o=>o.feasibility_status==="feasible").map(o=>rat(o.loss,"scenario best"));
  if(!vals.length) fail("scenario has no feasible action");
  return vals.reduce((a,b)=>cmpR(a,b)<=0?a:b);
}
function policyId(queryId,branches){
  return sha(canonical({query_id:queryId??null,branch_actions:branches}));
}
function signedDiff(base,candidate){
  const cmp=cmpR(base,candidate);
  return {direction:cmp>0?"positive_improvement":cmp<0?"negative_improvement":"no_change",
    magnitude:jsonR(cmp>0?subR(base,candidate,"improvement"):cmp<0?subR(candidate,base,"loss delta"):{n:0n,d:1n})};
}
function buildPolicy(base,queryId,queryCost,observations,classes,classOptions,branches,baseInfo){
  let actionCost={reference_trials:0,exposure_micro_units:0,elapsed_millis:0};
  const actionMap=baseInfo.actions;
  for(const actionId of Object.values(branches)){
    const action=actionMap.get(actionId);
    if(!action) fail("unknown selected action "+actionId);
    actionCost=maxCost(actionCost,action.cost);
  }
  const totalCost=addCost(queryCost,actionCost);
  const scenarios=baseInfo.scenarios;
  const classRegretByOutcome=new Map();
  // Select one comparator action that is robust across the complete public
  // observation class. Do not let the comparator see the hidden scenario.
  for(const [outcomeId,worlds] of Object.entries(classes)){
    const rows=classScenarios(scenarios,worlds);
    const options=classOptions[outcomeId];
    let bestWorst=null,selectedWorst=null;
    for(const candidateId of options){
      let worstForAction=null;
      for(const scenario of rows){
        const candidate=scenario.action_outcomes[candidateId];
        if(candidate.feasibility_status!=="feasible"||candidate.loss===null)
          fail("class comparator not feasible throughout outcome class");
        const loss=rat(candidate.loss,"outcome class loss");
        const regret=subR(loss,scenarioBest(scenario),"class action regret");
        if(worstForAction===null||cmpR(regret,worstForAction)>0) worstForAction=regret;
      }
      if(worstForAction===null) fail("empty outcome class");
      if(candidateId===branches[outcomeId]) selectedWorst=worstForAction;
      if(bestWorst===null||cmpR(worstForAction,bestWorst)<0) bestWorst=worstForAction;
    }
    if(bestWorst===null||selectedWorst===null) fail("missing selected or comparator action");
    classRegretByOutcome.set(outcomeId,subR(selectedWorst,bestWorst,"outcome-class minimax regret"));
  }

  const perScenario=[];
  for(const scenario of scenarios){
    const outcomeId=observations[scenario.world_id];
    const actionId=branches[outcomeId];
    if(!actionId) fail("policy missing observed outcome branch "+outcomeId);
    const row=scenario.action_outcomes[actionId];
    if(row.feasibility_status!=="feasible"||row.loss===null)
      fail("policy action infeasible in scenario "+scenario.scenario_id);
    const loss=rat(row.loss,"policy loss");
    const wsBest=scenarioBest(scenario);
    const classRegret=classRegretByOutcome.get(outcomeId);
    if(!classRegret) fail("missing outcome-class regret");
    perScenario.push({
      scenario_id:scenario.scenario_id,world_id:scenario.world_id,loss_model_id:scenario.loss_model_id,
      observed_outcome_id:outcomeId,selected_action_id:actionId,loss:jsonR(loss),
      wait_and_see_regret:jsonR(subR(loss,wsBest,"wait and see")),
      outcome_class_regret:jsonR(classRegret),
      _loss:loss,_ws:subR(loss,wsBest,"wait and see"),_class:classRegret
    });
  }
  const maxKey=key=>perScenario.reduce((best,row)=>cmpR(row[key],best)>0?row[key]:best,perScenario[0][key]);
  const worstLoss=maxKey("_loss"),worstWs=maxKey("_ws"),worstClass=maxKey("_class");
  const publicRows=perScenario.map(({_loss,_ws,_class,...row})=>row);
  return {
    policy_id:policyId(queryId,branches),query_id:queryId??null,
    branch_actions:branches,observation_class_world_ids:classes,
    worst_case_loss:jsonR(worstLoss),worst_case_wait_and_see_regret:jsonR(worstWs),
    worst_case_outcome_class_regret:jsonR(worstClass),
    worst_loss_scenario_ids:perScenario.filter(x=>cmpR(x._loss,worstLoss)===0).map(x=>x.scenario_id),
    worst_wait_and_see_scenario_ids:perScenario.filter(x=>cmpR(x._ws,worstWs)===0).map(x=>x.scenario_id),
    worst_outcome_class_regret_scenario_ids:perScenario.filter(x=>cmpR(x._class,worstClass)===0).map(x=>x.scenario_id),
    worst_loss_reduction_vs_best_no_query:null,
    query_cost:queryCost,branch_action_cost_upper_bound:actionCost,total_cost_upper_bound:totalCost,
    per_scenario:publicRows
  };
}

function enumerate(choices){
  const output=[];
  function rec(index,current){
    if(index===choices.length){output.push({...current});return;}
    const [outcome,actions]=choices[index];
    for(const action of actions){
      current[outcome]=action;rec(index+1,current);
      if(output.length>=MAX_POLICIES) return;
    }
    delete current[outcome];
  }
  rec(0,{});
  return output;
}
function paretoDominates(a,b){
  const dims=[
    cmpR(rat(a.worst_case_loss,"loss"),rat(b.worst_case_loss,"loss")),
    cmpR(rat(a.worst_case_wait_and_see_regret,"wait-and-see regret"),rat(b.worst_case_wait_and_see_regret,"wait-and-see regret")),
    cmpR(rat(a.worst_case_outcome_class_regret,"class regret"),rat(b.worst_case_outcome_class_regret,"class regret")),
    a.total_cost_upper_bound.reference_trials-b.total_cost_upper_bound.reference_trials,
    a.total_cost_upper_bound.exposure_micro_units-b.total_cost_upper_bound.exposure_micro_units,
    a.total_cost_upper_bound.elapsed_millis-b.total_cost_upper_bound.elapsed_millis
  ];
  return dims.every(x=>x<=0)&&dims.some(x=>x<0);
}
function evalOneQuery(base,query,baseInfo){
  const report={query_id:query.query_id,status:"READY",observable_outcome_ids:[],observation_class_world_ids:{},
    common_feasible_action_ids_by_outcome:{},generated_policy_count:0,policy_ids:[]};
  if(query.admissibility_status==="prohibited"){report.status="QUERY_PROHIBITED";return {report,policies:[]};}
  if(query.admissibility_status!=="admissible"){report.status="QUERY_ADMISSIBILITY_INDETERMINATE";return {report,policies:[]};}
  if(query.evidence_status==="out_of_profile"){report.status="QUERY_OUT_OF_PROFILE";return {report,policies:[]};}
  if(query.evidence_status!=="admissible"){report.status="QUERY_EVIDENCE_INDETERMINATE";return {report,policies:[]};}
  const classes=classMap(query.world_observations);
  report.observable_outcome_ids=[...classes.keys()];
  report.observation_class_world_ids=Object.fromEntries(classes);
  const options={};
  const choices=[];
  for(const [outcome,worlds] of classes){
    const rows=classScenarios(baseInfo.scenarios,worlds);
    const common=commonActions(base.actions,rows);
    report.common_feasible_action_ids_by_outcome[outcome]=common;
    if(!common.length){report.status="NO_SAFE_POLICY_FOR_OUTCOME";return {report,policies:[]};}
    options[outcome]=common;choices.push([outcome,common]);
  }
  const product=choices.reduce((p,x)=>p*x[1].length,1);
  if(product>MAX_POLICIES){report.status="POLICY_ENUMERATION_LIMIT_EXCEEDED";return {report,policies:[]};}
  const policies=enumerate(choices).map(branches=>buildPolicy(base,query.query_id,query.cost,query.world_observations,Object.fromEntries(classes),options,branches,baseInfo));
  report.generated_policy_count=policies.length;
  report.policy_ids=policies.map(p=>p.policy_id);
  return {report,policies};
}
function noQueryPolicies(base,baseInfo){
  const observations=Object.fromEntries(baseInfo.worldIds.map(world=>[world,NO_QUERY_OUTCOME]));
  const classes={[NO_QUERY_OUTCOME]:[...baseInfo.worldIds]};
  const rows=classScenarios(baseInfo.scenarios,baseInfo.worldIds);
  const common=commonActions(base.actions,rows);
  if(!common.length) return [];
  const options={[NO_QUERY_OUTCOME]:common};
  const choices=[[NO_QUERY_OUTCOME,common]];
  const zero={reference_trials:0,exposure_micro_units:0,elapsed_millis:0};
  return enumerate(choices).map(branches=>buildPolicy(base,null,zero,observations,classes,options,branches,baseInfo));
}
function emptyCase(item,status,baseStatus){
  return {
    case_id:item.case_id,status,base_feasibility_status:baseStatus,
    compatible_world_ids_sha256:item.base_decision.compatible_world_set.world_ids_sha256,
    action_ids_sha256:item.base_decision.action_universe.expected_action_ids_sha256,
    scenario_manifest_sha256:item.base_decision.scenario_manifest.scenarios_sha256,
    query_universe_receipt_id:item.query_universe.source_receipt_id,
    query_universe_scope_id:item.query_universe.scope_id,
    expected_query_ids_sha256:item.query_universe.expected_query_ids_sha256,
    queries_sha256:item.query_universe.queries_sha256,
    no_query_policy_ids:[],query_reports:[],policy_metrics:[],policy_pareto_frontier_ids:[],
    best_no_query_worst_case_loss:null
  };
}
function evaluateCase(item){
  const baseInfo=validateBase(item.base_decision);
  const allowed=new Set(["ACTION_UNIVERSE_OPEN","ACTION_UNIVERSE_INDETERMINATE","ACTION_ADMISSIBILITY_INDETERMINATE","OUT_OF_PROFILE","LOSS_MODEL_INDETERMINATE","FEASIBILITY_INDETERMINATE","NO_FEASIBLE_ACTION_IN_SCENARIO"]);
  if(allowed.has(baseInfo.status)) return emptyCase(item,"BASE_DECISION_INDETERMINATE",baseInfo.status);
  const clairvoyantWorstCaseLoss=item.base_decision.scenario_manifest.scenarios
    .map(scenario=>scenarioBest(scenario))
    .reduce((worst,value)=>worst===null||cmpR(value,worst)>0?value:worst,null);
  const noQuery=noQueryPolicies(item.base_decision,baseInfo);
  const noQueryIds=noQuery.map(p=>p.policy_id);
  let bestNoQuery=null;
  for(const policy of noQuery){
    const value=rat(policy.worst_case_loss,"no query worst loss");
    if(bestNoQuery===null||cmpR(value,bestNoQuery)<0) bestNoQuery=value;
  }
  const queries=[...item.query_universe.queries].sort((a,b)=>strCmp(a.query_id,b.query_id));
  const reports=[],policies=[...noQuery];
  for(const query of queries){
    const result=evalOneQuery(item.base_decision,query,baseInfo);
    reports.push(result.report);policies.push(...result.policies);
  }
  for(const policy of policies){
    if(policy.query_id!==null&&bestNoQuery!==null)
      policy.worst_loss_reduction_vs_best_no_query=signedDiff(bestNoQuery,rat(policy.worst_case_loss,"candidate worst loss"));
  }
  policies.sort((a,b)=>strCmp(a.policy_id,b.policy_id));
  const frontier=policies.filter(candidate=>!policies.some(other=>other.policy_id!==candidate.policy_id&&paretoDominates(other,candidate))).map(p=>p.policy_id);
  const queryCount=policies.filter(p=>p.query_id!==null).length;
  const status=!policies.length?"NO_FEASIBLE_QUERY_OR_STATIC_POLICY":
    (!noQuery.length&&queryCount>0?"QUERY_CONDITIONED_POLICIES_AVAILABLE_NO_STATIC_BASELINE":"POLICY_FRONTIER_AVAILABLE");
  return {
    case_id:item.case_id,status,base_feasibility_status:baseInfo.status,
    compatible_world_ids_sha256:item.base_decision.compatible_world_set.world_ids_sha256,
    action_ids_sha256:item.base_decision.action_universe.expected_action_ids_sha256,
    scenario_manifest_sha256:item.base_decision.scenario_manifest.scenarios_sha256,
    query_universe_receipt_id:item.query_universe.source_receipt_id,
    query_universe_scope_id:item.query_universe.scope_id,
    expected_query_ids_sha256:item.query_universe.expected_query_ids_sha256,
    queries_sha256:item.query_universe.queries_sha256,
    no_query_policy_ids:noQueryIds,query_reports:reports,policy_metrics:policies,
    policy_pareto_frontier_ids:frontier,
    best_no_query_worst_case_loss:bestNoQuery?jsonR(bestNoQuery):null,
    clairvoyant_worst_case_loss:clairvoyantWorstCaseLoss?jsonR(clairvoyantWorstCaseLoss):null
  };
}
function validateCorpus(input){
  if(input?.schema!=="luminous.orbital-feasibility-discovery.corpus.v0.1"||
     input.evidence_class!=="synthetic"||
     input.claim_ceiling!=="finite-passive-query-policy-tree-algebra-only"||
     !Array.isArray(input.cases)||!input.cases.length||input.cases.length>128) fail("unsupported feasibility discovery corpus");
  const ids=new Set();
  for(const item of input.cases){
    if(!validId(item.case_id)||ids.has(item.case_id)) fail("duplicate discovery case ID");
    ids.add(item.case_id);
    const baseInfo=validateBase(item.base_decision);
    validateQueryUniverse(item,baseInfo);
  }
}
function evaluate(input){
  validateCorpus(input);
  const cases=input.cases.map(evaluateCase).sort((a,b)=>strCmp(a.case_id,b.case_id));
  return {
    result:"pass",evidence_class:input.evidence_class,claim_ceiling:input.claim_ceiling,
    summary:{
      case_count:cases.length,
      base_decision_indeterminate_cases:cases.filter(c=>c.status==="BASE_DECISION_INDETERMINATE").length,
      no_query_policy_cases:cases.filter(c=>c.no_query_policy_ids.length>0).length,
      no_safe_query_policy_cases:cases.flatMap(c=>c.query_reports).filter(q=>q.status==="NO_SAFE_POLICY_FOR_OUTCOME").length,
      query_evidence_indeterminate_count:cases.flatMap(c=>c.query_reports).filter(q=>q.status==="QUERY_EVIDENCE_INDETERMINATE"||q.status==="QUERY_OUT_OF_PROFILE").length,
      query_prohibited_count:cases.flatMap(c=>c.query_reports).filter(q=>q.status==="QUERY_PROHIBITED").length,
      generated_policy_count:cases.reduce((n,c)=>n+c.policy_metrics.length,0),
      pareto_frontier_policy_count:cases.reduce((n,c)=>n+c.policy_pareto_frontier_ids.length,0),
      positive_worst_loss_reduction_policy_count:cases.flatMap(c=>c.policy_metrics).filter(p=>p.worst_loss_reduction_vs_best_no_query?.direction==="positive_improvement").length,
      policy_enumeration_limit_cases:cases.flatMap(c=>c.query_reports).filter(q=>q.status==="POLICY_ENUMERATION_LIMIT_EXCEEDED").length
    },
    cases,physical_qualification_granted:false,authority_granted:false
  };
}
try{
  const [path]=process.argv.slice(2);
  if(!path)fail("usage: feasibility_discovery_oracle.mjs <corpus.json>");
  const input=JSON.parse(fs.readFileSync(path,"utf8"));
  const output=evaluate(input);

  // Negative control: a scenario-wise oracle wants different actions in each
  // hidden world, while one shared public observation forces one branch action.
  const nonAnticipative=structuredClone(input);
  const conflictCase=nonAnticipative.cases.find(x=>x.case_id==="no_query_dominates_paid_query");
  const north=conflictCase.base_decision.scenario_manifest.scenarios.find(x=>x.scenario_id==="v0-s");
  const south=conflictCase.base_decision.scenario_manifest.scenarios.find(x=>x.scenario_id==="v1-s");
  north.action_outcomes.safe.loss={numerator:1,denominator:10};
  north.action_outcomes.alternate.loss={numerator:6,denominator:10};
  south.action_outcomes.safe.loss={numerator:9,denominator:10};
  south.action_outcomes.alternate.loss={numerator:6,denominator:10};
  conflictCase.base_decision.scenario_manifest.scenarios_sha256=sha(
    canonical([...conflictCase.base_decision.scenario_manifest.scenarios]
      .sort((a,b)=>strCmp(a.scenario_id,b.scenario_id)))
  );
  const conflictResult=evaluate(nonAnticipative).cases.find(x=>x.case_id==="no_query_dominates_paid_query");
  const alternatePolicy=conflictResult.policy_metrics.find(x=>
    x.query_id==="redundant-query" && x.branch_actions.same==="alternate");
  const safePolicy=conflictResult.policy_metrics.find(x=>
    x.query_id==="redundant-query" && x.branch_actions.same==="safe");
  if(alternatePolicy?.worst_case_wait_and_see_regret?.numerator!==1 ||
     alternatePolicy?.worst_case_wait_and_see_regret?.denominator!==2 ||
     alternatePolicy?.worst_case_outcome_class_regret?.numerator!==1 ||
     alternatePolicy?.worst_case_outcome_class_regret?.denominator!==5) {
    fail("clairvoyant and non-anticipative regret controls collapsed or were miscomputed");
  }
  if(safePolicy?.worst_case_outcome_class_regret?.numerator!==0 ||
     safePolicy?.worst_case_outcome_class_regret?.denominator!==1) {
    fail("outcome-class robust comparator failed to identify the best common action");
  }
  const reversed=structuredClone(input);
  reversed.cases.reverse();
  for(const item of reversed.cases){
    item.query_universe.expected_query_ids.reverse();
    item.query_universe.queries.reverse();
    for(const query of item.query_universe.queries) query.expected_world_ids.reverse();
    item.base_decision.actions.reverse();
    item.base_decision.compatible_world_set.world_ids.reverse();
    item.base_decision.scenario_manifest.scenarios.reverse();
  }
  assert.deepStrictEqual(evaluate(reversed),output,"policy enumeration changed under case/query/world/action/scenario ordering");
  process.stdout.write(JSON.stringify(output,null,2)+"\n");
}catch(error){process.stderr.write(String(error?.stack??error)+"\n");process.exitCode=1;}
