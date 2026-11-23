// Independent BigInt oracle for joint feasibility and regret frontiers.
import fs from "node:fs";
import {createHash} from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE=9_007_199_254_740_991n;
function fail(message){throw new Error(message);}
function stringCompare(a,b){
  const x=Array.from(a,v=>v.codePointAt(0)),y=Array.from(b,v=>v.codePointAt(0));
  for(let i=0;i<Math.min(x.length,y.length);i+=1) if(x[i]!==y[i]) return x[i]-y[i];
  return x.length-y.length;
}
function canonical(value){
  if(Array.isArray(value)) return "["+value.map(canonical).join(",")+"]";
  if(value!==null&&typeof value==="object")
    return "{"+Object.keys(value).sort(stringCompare).map(k=>JSON.stringify(k)+":"+canonical(value[k])).join(",")+"}";
  return JSON.stringify(value);
}
function sha(value){return createHash("sha256").update(value,"utf8").digest("hex");}
function digestIds(ids){return sha(JSON.stringify([...ids].sort(stringCompare)));}
function gcd(a,b){while(b!==0n){const r=a%b;a=b;b=r;}return a>0n?a:1n;}
function rat(value,path){
  if(!Number.isSafeInteger(value?.numerator)||!Number.isSafeInteger(value?.denominator)||
     value.numerator<0||value.denominator<=0||
     BigInt(value.numerator)>MAX_SAFE||BigInt(value.denominator)>MAX_SAFE||
     value.numerator>value.denominator) fail(path+": invalid exact unit-interval rational");
  const n=BigInt(value.numerator),d=BigInt(value.denominator),g=gcd(n,d);
  return {n:n/g,d:d/g};
}
function cmpR(a,b){const x=a.n*b.d,y=b.n*a.d;return x<y?-1:x>y?1:0;}
function subR(a,b,path){
  const n=a.n*b.d-b.n*a.d,d=a.d*b.d;
  if(n<0n) fail(path+": regret underflow");
  const g=gcd(n,d);return {n:n/g,d:d/g};
}
function jsonR(x){return {numerator:Number(x.n),denominator:Number(x.d)};}
function id(value,path){
  if(typeof value!=="string"||!value.trim()||!/^[\x00-\x7F]+$/.test(value)) fail(path+": expected nonempty ASCII ID");
}
function validate(input){
  if(input?.schema!=="luminous.orbital-feasibility-regret.corpus.v0.1"||
     input.evidence_class!=="synthetic"||
     input.claim_ceiling!=="finite-joint-feasibility-and-regret-algebra-only"||
     !Array.isArray(input.cases)||input.cases.length<1||input.cases.length>256) fail("unsupported feasibility corpus");
  const caseIds=new Set();
  for(const [ci,item] of input.cases.entries()){
    const path="cases["+ci+"]";
    id(item.case_id,path+".case_id");
    id(item.decision_scope_id,path+".decision_scope_id");
    if(caseIds.has(item.case_id)) fail("duplicate case ID");
    caseIds.add(item.case_id);
    const worlds=item.compatible_world_set;
    id(worlds.source_receipt_id,path+".world receipt");
    if(!Array.isArray(worlds.world_ids)||worlds.world_ids.length<1||worlds.world_ids.length>64) fail("invalid world count");
    const worldIds=[...worlds.world_ids].sort(stringCompare);
    for(const value of worldIds) id(value,path+".world ID");
    if(new Set(worldIds).size!==worldIds.length||worlds.world_ids_sha256!==digestIds(worldIds)) fail("world ID inventory/digest mismatch");
    const universe=item.action_universe;
    for(const [v,k] of [[universe.source_receipt_id,"action universe receipt"],[universe.scope_id,"action universe scope"],[universe.enumeration_method,"enumeration method"]]) id(v,path+"."+k);
    if(universe.scope_id!==item.decision_scope_id) fail("action universe scope mismatch");
    if(!["finite_exhaustive","open","indeterminate"].includes(universe.closure_kind)||
       !["admissible","indeterminate","out_of_scope"].includes(universe.evidence_status)) fail("invalid action universe status");
    if(!Array.isArray(universe.expected_action_ids)||universe.expected_action_ids.length<1||universe.expected_action_ids.length>128) fail("invalid planned action inventory");
    const expectedActions=[...universe.expected_action_ids].sort(stringCompare);
    for(const value of expectedActions) id(value,path+".expected action ID");
    if(new Set(expectedActions).size!==expectedActions.length||universe.expected_action_ids_sha256!==digestIds(expectedActions)) fail("planned action inventory digest mismatch");
    if(!Array.isArray(item.actions)||item.actions.length<1||item.actions.length>128) fail("invalid action count");
    const actionMap=new Map();
    for(const action of item.actions){
      id(action.action_id,path+".action_id");
      if(actionMap.has(action.action_id)) fail("duplicate action ID");
      if(!["eligible","prohibited","indeterminate"].includes(action.eligibility_status)) fail("invalid static eligibility status");
      id(action.eligibility_receipt_id,path+".eligibility receipt");
      for(const key of ["reference_trials","exposure_micro_units","elapsed_millis"])
        if(!Number.isSafeInteger(action.cost?.[key])||action.cost[key]<0||BigInt(action.cost[key])>MAX_SAFE) fail("invalid native action cost");
      actionMap.set(action.action_id,action);
    }
    if(JSON.stringify([...actionMap.keys()].sort(stringCompare))!==JSON.stringify(expectedActions)) fail("observed action inventory differs from planned exhaustive universe");
    const manifest=item.scenario_manifest;
    id(manifest.source_manifest_id,path+".scenario manifest ID");
    if(!Array.isArray(manifest.expected_scenario_ids)||manifest.expected_scenario_ids.length<1||manifest.expected_scenario_ids.length>4096||
       !Array.isArray(manifest.scenarios)||manifest.scenarios.length<1||manifest.scenarios.length>4096) fail("invalid scenario inventory");
    const expectedScenarios=[...manifest.expected_scenario_ids].sort(stringCompare);
    for(const value of expectedScenarios) id(value,path+".expected scenario ID");
    if(new Set(expectedScenarios).size!==expectedScenarios.length||manifest.expected_scenario_ids_sha256!==digestIds(expectedScenarios)) fail("planned scenario inventory digest mismatch");
    const observed=new Set(),coveredWorlds=new Set();
    for(const scenario of manifest.scenarios){
      id(scenario.scenario_id,path+".scenario_id");id(scenario.world_id,path+".world_id");id(scenario.loss_model_id,path+".loss_model_id");
      if(observed.has(scenario.scenario_id)) fail("duplicate scenario ID");
      observed.add(scenario.scenario_id);
      if(!worldIds.includes(scenario.world_id)) fail("scenario references unknown world");
      if(!["admissible","indeterminate","out_of_profile"].includes(scenario.evidence_status)) fail("invalid scenario evidence status");
      if(scenario.evidence_status==="admissible") id(scenario.evidence_receipt_id,path+".scenario evidence receipt");
      coveredWorlds.add(scenario.world_id);
      const keys=Object.keys(scenario.action_outcomes??{}).sort(stringCompare);
      if(JSON.stringify(keys)!==JSON.stringify(expectedActions)) fail("scenario action outcome map does not exactly cover action universe");
      for(const actionId of expectedActions){
        const action=actionMap.get(actionId),outcome=scenario.action_outcomes[actionId];
        const status=outcome?.feasibility_status;
        if(action.eligibility_status==="prohibited"){
          if(status!=="prohibited"||outcome.loss!==null) fail("prohibited action must have prohibited null-loss outcome");
        }else if(action.eligibility_status==="indeterminate"){
          if(status!=="indeterminate"||outcome.loss!==null) fail("indeterminate action must have indeterminate null-loss outcome");
        }else if(status==="feasible"){
          if(outcome.loss===null) fail("feasible action has no loss");
          rat(outcome.loss,"scenario action loss");
        }else if(status==="infeasible"||status==="indeterminate"){
          if(outcome.loss!==null) fail("non-feasible action carries loss");
        }else fail("invalid per-scenario feasibility status");
      }
    }
    if(JSON.stringify([...observed].sort(stringCompare))!==JSON.stringify(expectedScenarios)) fail("observed scenario inventory differs from plan");
    if(JSON.stringify([...coveredWorlds].sort(stringCompare))!==JSON.stringify(worldIds)) fail("scenario inventory does not cover every world");
    if(manifest.scenarios_sha256!==sha(canonical([...manifest.scenarios].sort((a,b)=>stringCompare(a.scenario_id,b.scenario_id)))) ) fail("full scenario manifest digest mismatch");
  }
}
function argmin(metrics,key){
  if(!metrics.length)return [];
  let best=metrics[0][key];
  for(const metric of metrics.slice(1)) if(cmpR(metric[key],best)<0) best=metric[key];
  return metrics.filter(metric=>cmpR(metric[key],best)===0).map(metric=>metric.action_id);
}
function dominates(a,b){
  const dims=[
    cmpR(a.worst_case_loss,b.worst_case_loss),
    cmpR(a.worst_case_wait_and_see_regret,b.worst_case_wait_and_see_regret),
    cmpR(a.worst_case_robust_set_regret,b.worst_case_robust_set_regret),
    a.cost.reference_trials-b.cost.reference_trials,
    a.cost.exposure_micro_units-b.cost.exposure_micro_units,
    a.cost.elapsed_millis-b.cost.elapsed_millis
  ];
  return dims.every(x=>x<=0)&&dims.some(x=>x<0);
}
function emptyCase(item,status){
  return {
    case_id:item.case_id,status,
    compatible_world_set_receipt_id:item.compatible_world_set.source_receipt_id,
    compatible_world_ids_sha256:item.compatible_world_set.world_ids_sha256,
    action_universe_receipt_id:item.action_universe.source_receipt_id,
    action_universe_scope_id:item.action_universe.scope_id,
    action_universe_closure_kind:item.action_universe.closure_kind,
    action_universe_enumeration_method:item.action_universe.enumeration_method,
    action_ids_sha256:item.action_universe.expected_action_ids_sha256,
    scenario_manifest_id:item.scenario_manifest.source_manifest_id,
    expected_scenario_ids_sha256:item.scenario_manifest.expected_scenario_ids_sha256,
    scenarios_sha256:item.scenario_manifest.scenarios_sha256,
    compatible_world_count:item.compatible_world_set.world_ids.length,
    action_count:item.actions.length,scenario_count:item.scenario_manifest.scenarios.length,
    robust_feasible_action_ids:[],scenario_feasible_action_ids:[],action_feasibility_classes:[],scenario_feasible_sets:[],
    minimax_wait_and_see_regret_action_ids:[],minimax_robust_set_regret_action_ids:[],pareto_frontier_action_ids:[],action_metrics:[]
  };
}
function evaluateCase(item){
  if(item.action_universe.closure_kind==="open") return emptyCase(item,"ACTION_UNIVERSE_OPEN");
  if(item.action_universe.closure_kind==="indeterminate"||item.action_universe.evidence_status!=="admissible")
    return emptyCase(item,"ACTION_UNIVERSE_INDETERMINATE");
  if(item.actions.some(a=>a.eligibility_status==="indeterminate")) return emptyCase(item,"ACTION_ADMISSIBILITY_INDETERMINATE");
  if(item.scenario_manifest.scenarios.some(s=>s.evidence_status==="out_of_profile")) return emptyCase(item,"OUT_OF_PROFILE");
  if(item.scenario_manifest.scenarios.some(s=>s.evidence_status!=="admissible")) return emptyCase(item,"LOSS_MODEL_INDETERMINATE");
  const actions=[...item.actions].sort((a,b)=>stringCompare(a.action_id,b.action_id));
  const scenarios=[...item.scenario_manifest.scenarios].sort((a,b)=>stringCompare(a.scenario_id,b.scenario_id));
  const feasibleCount=new Map(),scenarioSets=[],scenarioBest=new Map();
  let robust=null;
  for(const scenario of scenarios){
    const feasible=[],infeasible=[],prohibited=[];
    let best=null;
    for(const action of actions){
      const status=scenario.action_outcomes[action.action_id].feasibility_status;
      if(status==="feasible"){
        feasible.push(action.action_id);
        feasibleCount.set(action.action_id,(feasibleCount.get(action.action_id)??0)+1);
        const loss=rat(scenario.action_outcomes[action.action_id].loss,"scenario loss");
        if(best===null||cmpR(loss,best)<0) best=loss;
      }else if(status==="infeasible") infeasible.push(action.action_id);
      else if(status==="prohibited") prohibited.push(action.action_id);
      else if(status==="indeterminate") return emptyCase(item,"FEASIBILITY_INDETERMINATE");
      else fail("validated unknown feasibility status");
    }
    if(!feasible.length) return emptyCase(item,"NO_FEASIBLE_ACTION_IN_SCENARIO");
    scenarioBest.set(scenario.scenario_id,best);
    const set=new Set(feasible);
    robust=robust===null?set:new Set([...robust].filter(id=>set.has(id)));
    scenarioSets.push({scenario_id:scenario.scenario_id,world_id:scenario.world_id,loss_model_id:scenario.loss_model_id,
      feasible_action_ids:feasible,infeasible_action_ids:infeasible,prohibited_action_ids:prohibited});
  }
  const classes=actions.map(action=>{
    const count=feasibleCount.get(action.action_id)??0;
    let classification,feasibleScenarioCount,infeasibleScenarioCount;
    if(action.eligibility_status==="prohibited"){classification="PROHIBITED";feasibleScenarioCount=0;infeasibleScenarioCount=0;}
    else if(count===scenarios.length){classification="ALWAYS_FEASIBLE";feasibleScenarioCount=count;infeasibleScenarioCount=0;}
    else if(count===0){classification="NEVER_FEASIBLE";feasibleScenarioCount=0;infeasibleScenarioCount=scenarios.length;}
    else{classification="SOMETIMES_FEASIBLE";feasibleScenarioCount=count;infeasibleScenarioCount=scenarios.length-count;}
    return {action_id:action.action_id,classification,feasible_scenario_count:feasibleScenarioCount,infeasible_scenario_count:infeasibleScenarioCount,
      eligibility_status:action.eligibility_status,eligibility_receipt_id:action.eligibility_receipt_id};
  });
  const scenarioUnion=[...new Set(scenarioSets.flatMap(x=>x.feasible_action_ids))].sort(stringCompare);
  const robustIds=[...robust].sort(stringCompare);
  if(!robust.size){
    const result=emptyCase(item,"NO_ROBUST_FEASIBLE_ACTION");
    result.action_feasibility_classes=classes;
    result.scenario_feasible_sets=scenarioSets;
    result.scenario_feasible_action_ids=scenarioUnion;
    return result;
  }
  const robustActions=actions.filter(a=>robust.has(a.action_id));
  const robustBest=new Map();
  for(const scenario of scenarios){
    let best=null;
    for(const action of robustActions){
      const value=rat(scenario.action_outcomes[action.action_id].loss,"robust-set loss");
      if(best===null||cmpR(value,best)<0) best=value;
    }
    robustBest.set(scenario.scenario_id,best);
  }
  const metrics=robustActions.map(action=>{
    const perScenario=scenarios.map(scenario=>{
      const loss=rat(scenario.action_outcomes[action.action_id].loss,"robust action loss");
      return {scenario_id:scenario.scenario_id,world_id:scenario.world_id,loss_model_id:scenario.loss_model_id,
        loss:jsonR(loss),wait_and_see_regret:jsonR(subR(loss,scenarioBest.get(scenario.scenario_id),"wait and see")),
        robust_set_regret:jsonR(subR(loss,robustBest.get(scenario.scenario_id),"robust set")),
        _loss:loss,_wait:subR(loss,scenarioBest.get(scenario.scenario_id),"wait and see"),
        _robust:subR(loss,robustBest.get(scenario.scenario_id),"robust set")};
    });
    const maxOf=key=>{
      let best=perScenario[0][key];
      for(const row of perScenario.slice(1)) if(cmpR(row[key],best)>0) best=row[key];
      return best;
    };
    const worstLoss=maxOf("_loss"),worstWait=maxOf("_wait"),worstRobust=maxOf("_robust");
    return {
      action_id:action.action_id,eligibility_receipt_id:action.eligibility_receipt_id,
      worst_case_loss:jsonR(worstLoss),worst_case_wait_and_see_regret:jsonR(worstWait),worst_case_robust_set_regret:jsonR(worstRobust),
      worst_loss_scenario_ids:perScenario.filter(x=>cmpR(x._loss,worstLoss)===0).map(x=>x.scenario_id),
      worst_wait_and_see_scenario_ids:perScenario.filter(x=>cmpR(x._wait,worstWait)===0).map(x=>x.scenario_id),
      worst_robust_set_scenario_ids:perScenario.filter(x=>cmpR(x._robust,worstRobust)===0).map(x=>x.scenario_id),
      uniformly_best_against_scenario_feasible_sets:perScenario.every(x=>x._wait.n===0n),
      cost:action.cost,per_scenario:perScenario.map(({scenario_id,world_id,loss_model_id,loss,wait_and_see_regret,robust_set_regret})=>({scenario_id,world_id,loss_model_id,loss,wait_and_see_regret,robust_set_regret})),
      _wait:worstWait,_robust:worstRobust
    };
  });
  const publicMetrics=metrics.map(({_wait,_robust,...metric})=>metric);
  const frontier=metrics.filter(candidate=>!metrics.some(other=>other.action_id!==candidate.action_id&&!dominates(other,candidate))).map(m=>m.action_id);
  return {
    case_id:item.case_id,status:"ROBUST_FEASIBLE_DECISION_SET_AVAILABLE",
    compatible_world_set_receipt_id:item.compatible_world_set.source_receipt_id,
    compatible_world_ids_sha256:item.compatible_world_set.world_ids_sha256,
    action_universe_receipt_id:item.action_universe.source_receipt_id,
    action_universe_scope_id:item.action_universe.scope_id,
    action_universe_closure_kind:item.action_universe.closure_kind,
    action_universe_enumeration_method:item.action_universe.enumeration_method,
    action_ids_sha256:item.action_universe.expected_action_ids_sha256,
    scenario_manifest_id:item.scenario_manifest.source_manifest_id,
    expected_scenario_ids_sha256:item.scenario_manifest.expected_scenario_ids_sha256,
    scenarios_sha256:item.scenario_manifest.scenarios_sha256,
    compatible_world_count:item.compatible_world_set.world_ids.length,action_count:item.actions.length,scenario_count:item.scenario_manifest.scenarios.length,
    robust_feasible_action_ids:robustIds,scenario_feasible_action_ids:scenarioUnion,
    action_feasibility_classes:classes,scenario_feasible_sets:scenarioSets,
    minimax_wait_and_see_regret_action_ids:argmin(metrics,"_wait"),
    minimax_robust_set_regret_action_ids:argmin(metrics,"_robust"),
    pareto_frontier_action_ids:frontier,action_metrics:publicMetrics
  };
}
function evaluate(input){
  validate(input);
  const cases=input.cases.map(evaluateCase).sort((a,b)=>stringCompare(a.case_id,b.case_id));
  return {
    result:"pass",evidence_class:input.evidence_class,claim_ceiling:input.claim_ceiling,
    summary:{
      case_count:cases.length,
      action_universe_open_cases:cases.filter(x=>x.status==="ACTION_UNIVERSE_OPEN").length,
      action_universe_indeterminate_cases:cases.filter(x=>x.status==="ACTION_UNIVERSE_INDETERMINATE").length,
      action_admissibility_indeterminate_cases:cases.filter(x=>x.status==="ACTION_ADMISSIBILITY_INDETERMINATE").length,
      feasibility_indeterminate_cases:cases.filter(x=>x.status==="FEASIBILITY_INDETERMINATE").length,
      loss_model_indeterminate_cases:cases.filter(x=>x.status==="LOSS_MODEL_INDETERMINATE"||x.status==="OUT_OF_PROFILE").length,
      no_feasible_scenario_cases:cases.filter(x=>x.status==="NO_FEASIBLE_ACTION_IN_SCENARIO").length,
      no_robust_feasible_action_cases:cases.filter(x=>x.status==="NO_ROBUST_FEASIBLE_ACTION").length,
      robust_decision_cases:cases.filter(x=>x.status==="ROBUST_FEASIBLE_DECISION_SET_AVAILABLE").length,
      declared_action_count:cases.reduce((sum,x)=>sum+x.action_count,0),
      robust_feasible_action_count:cases.reduce((sum,x)=>sum+x.robust_feasible_action_ids.length,0),
      sometimes_feasible_action_count:cases.reduce((sum,x)=>sum+x.action_feasibility_classes.filter(y=>y.classification==="SOMETIMES_FEASIBLE").length,0),
      never_feasible_action_count:cases.reduce((sum,x)=>sum+x.action_feasibility_classes.filter(y=>y.classification==="NEVER_FEASIBLE").length,0),
      pareto_frontier_action_count:cases.reduce((sum,x)=>sum+x.pareto_frontier_action_ids.length,0)
    },
    cases,physical_qualification_granted:false,authority_granted:false
  };
}
try{
  const [inputPath]=process.argv.slice(2);
  if(!inputPath) fail("usage: feasibility_regret_oracle.mjs <corpus.json>");
  const input=JSON.parse(fs.readFileSync(inputPath,"utf8"));
  const output=evaluate(input);
  const reversed=structuredClone(input);
  reversed.cases.reverse();
  for(const item of reversed.cases){
    item.actions.reverse();
    item.compatible_world_set.world_ids.reverse();
    item.action_universe.expected_action_ids.reverse();
    item.scenario_manifest.scenarios.reverse();
    item.scenario_manifest.expected_scenario_ids.reverse();
  }
  assert.deepStrictEqual(evaluate(reversed),output,"feasibility regret output changed under input ordering");
  process.stdout.write(JSON.stringify(output,null,2)+"\n");
}catch(error){process.stderr.write(String(error?.message??error)+"\n");process.exitCode=1;}
