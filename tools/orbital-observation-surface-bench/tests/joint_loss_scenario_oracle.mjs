// Independent BigInt oracle for exact joint loss-model scenario regret.
import fs from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE = 9_007_199_254_740_991n;
function fail(message) { throw new Error(message); }
function codePointCompare(left, right) {
  const a = Array.from(left, value => value.codePointAt(0));
  const b = Array.from(right, value => value.codePointAt(0));
  for (let index = 0; index < Math.min(a.length, b.length); index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index];
  }
  return a.length - b.length;
}
function canonical(value) {
  if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
  if (value !== null && typeof value === "object") {
    return "{" + Object.keys(value).sort(codePointCompare)
      .map(key => JSON.stringify(key) + ":" + canonical(value[key])).join(",") + "}";
  }
  return JSON.stringify(value);
}
function sha256(value) {
  return createHash("sha256").update(value, "utf8").digest("hex");
}
function gcd(a, b) {
  while (b !== 0n) { const remainder = a % b; a = b; b = remainder; }
  return a > 0n ? a : 1n;
}
function fraction(value, path) {
  if (!Number.isSafeInteger(value?.numerator) ||
      !Number.isSafeInteger(value?.denominator) ||
      value.numerator < 0 || value.denominator <= 0 ||
      BigInt(value.numerator) > MAX_SAFE ||
      BigInt(value.denominator) > MAX_SAFE ||
      value.numerator > value.denominator) {
    fail(path + ": invalid exact unit-interval rational");
  }
  const numerator = BigInt(value.numerator), denominator = BigInt(value.denominator);
  const divisor = gcd(numerator, denominator);
  return { n: numerator / divisor, d: denominator / divisor };
}
function compare(a, b) {
  const left = a.n * b.d, right = b.n * a.d;
  return left < right ? -1 : left > right ? 1 : 0;
}
function subtract(a, b, path) {
  const common = gcd(a.d, b.d);
  const leftScale = b.d / common, rightScale = a.d / common;
  const left = a.n * leftScale, right = b.n * rightScale;
  if (left < right) fail(path + ": exact subtraction underflow");
  const denominator = a.d * leftScale, numerator = left - right;
  const divisor = gcd(numerator, denominator);
  const n = numerator / divisor, d = denominator / divisor;
  if (n > MAX_SAFE || d > MAX_SAFE) fail(path + ": exceeds portable integer envelope");
  return { n, d };
}
function asJson(value) {
  return { numerator: Number(value.n), denominator: Number(value.d) };
}
function rationalZero(value) { return value.n === 0n; }
function validateId(value, path) {
  if (typeof value !== "string" || !value.trim() || !/^[\x00-\x7F]+$/.test(value)) {
    fail(path + ": expected a nonempty ASCII ID");
  }
}
function validate(corpus) {
  if (corpus?.schema !== "luminous.orbital-joint-loss-scenario.corpus.v0.1" ||
      corpus?.evidence_class !== "synthetic" ||
      corpus?.claim_ceiling !== "finite-joint-loss-scenario-regret-algebra-only" ||
      !Array.isArray(corpus.cases) || corpus.cases.length < 1 || corpus.cases.length > 256) {
    fail("unsupported joint-loss corpus or claim ceiling");
  }
  const caseIds = new Set();
  for (const [index, item] of corpus.cases.entries()) {
    const path = "cases[" + index + "]";
    validateId(item.case_id, path + ".case_id");
    if (caseIds.has(item.case_id)) fail(path + ": duplicate case ID");
    caseIds.add(item.case_id);
    const worldSet = item.compatible_world_set;
    validateId(worldSet?.source_receipt_id, path + ".compatible_world_set.source_receipt_id");
    if (!Array.isArray(worldSet.world_ids) || worldSet.world_ids.length < 1 || worldSet.world_ids.length > 64) fail(path + ": invalid world count");
    for (const id of worldSet.world_ids) validateId(id, path + ".world_ids");
    if (new Set(worldSet.world_ids).size !== worldSet.world_ids.length) fail(path + ": duplicate world ID");
    const expectedWorldDigest = sha256(JSON.stringify([...worldSet.world_ids].sort(codePointCompare)));
    if (worldSet.world_ids_sha256 !== expectedWorldDigest) fail(path + ": world digest mismatch");

    const manifest = item.loss_scenario_manifest;
    validateId(manifest?.source_manifest_id, path + ".loss_scenario_manifest.source_manifest_id");
    if (!Array.isArray(manifest.scenarios) || manifest.scenarios.length < 1 || manifest.scenarios.length > 4096) fail(path + ": invalid loss scenario count");
    const actions = item.actions;
    if (!Array.isArray(actions) || actions.length < 1 || actions.length > 64) fail(path + ": invalid action count");
    const actionIds = new Set();
    for (const [actionIndex, action] of actions.entries()) {
      const actionPath = path + ".actions[" + actionIndex + "]";
      validateId(action.action_id, actionPath + ".action_id");
      if (actionIds.has(action.action_id)) fail(actionPath + ": duplicate action ID");
      actionIds.add(action.action_id);
      if (!["admissible", "inadmissible", "indeterminate"].includes(action.admissibility_status)) fail(actionPath + ": invalid admissibility status");
      if (action.admissibility_status === "admissible") validateId(action.admissibility_receipt_id, actionPath + ".admissibility_receipt_id");
      for (const key of ["reference_trials", "exposure_micro_units", "elapsed_millis"]) {
        if (!Number.isSafeInteger(action.cost?.[key]) || action.cost[key] < 0 || BigInt(action.cost[key]) > MAX_SAFE) fail(actionPath + ": invalid cost " + key);
      }
    }

    const scenarioIds = new Set(), coveredWorlds = new Set();
    for (const [scenarioIndex, scenario] of manifest.scenarios.entries()) {
      const scenarioPath = path + ".scenarios[" + scenarioIndex + "]";
      validateId(scenario.scenario_id, scenarioPath + ".scenario_id");
      if (scenarioIds.has(scenario.scenario_id)) fail(scenarioPath + ": duplicate scenario ID");
      scenarioIds.add(scenario.scenario_id);
      validateId(scenario.world_id, scenarioPath + ".world_id");
      if (!worldSet.world_ids.includes(scenario.world_id)) fail(scenarioPath + ": unknown world ID");
      validateId(scenario.loss_model_id, scenarioPath + ".loss_model_id");
      if (!["admissible", "inadmissible", "indeterminate", "out_of_profile"].includes(scenario.evidence_status)) fail(scenarioPath + ": invalid evidence status");
      if (scenario.evidence_status === "admissible") validateId(scenario.evidence_receipt_id, scenarioPath + ".evidence_receipt_id");
      coveredWorlds.add(scenario.world_id);
      const keys = Object.keys(scenario.action_losses ?? {}).sort(codePointCompare);
      if (JSON.stringify(keys) !== JSON.stringify([...actionIds].sort(codePointCompare))) fail(scenarioPath + ": loss map must cover exactly the declared actions");
      for (const actionId of actionIds) fraction(scenario.action_losses[actionId], scenarioPath + ".action_losses." + actionId);
    }
    if (coveredWorlds.size !== worldSet.world_ids.length) fail(path + ": manifest omits a compatible world");
    const canonicalScenarios = canonical([...manifest.scenarios].sort((a, b) => codePointCompare(a.scenario_id, b.scenario_id)));
    if (manifest.scenarios_sha256 !== sha256(canonicalScenarios)) fail(path + ": loss scenario manifest digest mismatch");
  }
}
function argmin(metrics, key) {
  if (!metrics.length) return [];
  let best = metrics[0][key];
  for (const metric of metrics.slice(1)) if (compare(metric[key], best) < 0) best = metric[key];
  return metrics.filter(metric => compare(metric[key], best) === 0).map(metric => metric.action_id);
}
function dominates(left, right, rectangular) {
  const regret = rectangular
    ? compare(left.worst_case_rectangular_regret, right.worst_case_rectangular_regret)
    : compare(left.worst_case_joint_regret, right.worst_case_joint_regret);
  const dims = [
    compare(left.worst_case_loss, right.worst_case_loss),
    regret,
    left.cost.reference_trials - right.cost.reference_trials,
    left.cost.exposure_micro_units - right.cost.exposure_micro_units,
    left.cost.elapsed_millis - right.cost.elapsed_millis
  ];
  return dims.every(value => value <= 0) && dims.some(value => value < 0);
}
function emptyCase(item, status, issues, excluded) {
  return {
    case_id: item.case_id,
    status,
    compatible_world_set_receipt_id: item.compatible_world_set.source_receipt_id,
    compatible_world_ids_sha256: item.compatible_world_set.world_ids_sha256,
    loss_scenario_manifest_id: item.loss_scenario_manifest.source_manifest_id,
    loss_scenarios_sha256: item.loss_scenario_manifest.scenarios_sha256,
    compatible_world_count: item.compatible_world_set.world_ids.length,
    loss_scenario_count: item.loss_scenario_manifest.scenarios.length,
    loss_model_ids: [],
    uniformly_optimal_action_ids: [],
    joint_minimax_regret_action_ids: [],
    rectangular_minimax_regret_action_ids: [],
    joint_scenario_pareto_frontier_action_ids: [],
    rectangular_pareto_frontier_action_ids: [],
    excluded_actions: excluded,
    scenario_evidence_issues: issues,
    action_metrics: []
  };
}
function evaluateCase(item) {
  const worldIds = [...item.compatible_world_set.world_ids].sort(codePointCompare);
  const admitted = [...item.actions]
    .filter(action => action.admissibility_status === "admissible")
    .sort((a, b) => codePointCompare(a.action_id, b.action_id));
  const excluded = [...item.actions]
    .filter(action => action.admissibility_status !== "admissible")
    .sort((a, b) => codePointCompare(a.action_id, b.action_id))
    .map(action => ({action_id:action.action_id,admissibility_status:action.admissibility_status}));
  const scenarios = [...item.loss_scenario_manifest.scenarios]
    .sort((a, b) => codePointCompare(a.scenario_id, b.scenario_id));
  const issues = scenarios
    .filter(scenario => scenario.evidence_status !== "admissible")
    .map(scenario => ({
      scenario_id:scenario.scenario_id,world_id:scenario.world_id,loss_model_id:scenario.loss_model_id,
      evidence_status:scenario.evidence_status,evidence_receipt_id:scenario.evidence_receipt_id
    }));
  if (issues.some(issue => issue.evidence_status === "out_of_profile")) {
    return emptyCase(item,"OUT_OF_PROFILE",issues,excluded);
  }
  if (issues.length) return emptyCase(item,"LOSS_MODEL_INDETERMINATE",issues,excluded);
  if (!admitted.length) return emptyCase(item,"NO_ADMISSIBLE_ACTION",issues,excluded);

  const bestByScenario = new Map();
  for (const scenario of scenarios) {
    let best = fraction(scenario.action_losses[admitted[0].action_id],"scenario best");
    for (const action of admitted.slice(1)) {
      const value = fraction(scenario.action_losses[action.action_id],"scenario best");
      if (compare(value,best)<0) best=value;
    }
    bestByScenario.set(scenario.scenario_id,best);
  }

  const minByWorld = new Map(), maxByWorldAction = new Map();
  for (const worldId of worldIds) {
    const worldScenarios = scenarios.filter(scenario => scenario.world_id === worldId);
    for (const action of admitted) {
      let minimum=null, maximum=null;
      for (const scenario of worldScenarios) {
        const value=fraction(scenario.action_losses[action.action_id],"rectangular action loss");
        if (minimum===null || compare(value,minimum)<0) minimum=value;
        if (maximum===null || compare(value,maximum)>0) maximum=value;
      }
      if (minimum===null || maximum===null) fail("world has no loss scenario");
      maxByWorldAction.set(action.action_id+"\u0000"+worldId,maximum);
      if (!minByWorld.has(worldId) || compare(minimum,minByWorld.get(worldId))<0) minByWorld.set(worldId,minimum);
    }
  }

  const metrics=admitted.map(action=>{
    const perScenario=[];
    let bestCount=0;
    for (const scenario of scenarios) {
      const loss=fraction(scenario.action_losses[action.action_id],"action loss").n===0n
        ? {n:0n,d:1n}
        : fraction(scenario.action_losses[action.action_id],"action loss");
      const regret=subtract(loss,bestByScenario.get(scenario.scenario_id),"joint regret");
      if (rationalZero(regret)) bestCount += 1;
      perScenario.push({
        scenario_id:scenario.scenario_id,world_id:scenario.world_id,loss:asJson(loss),joint_regret:asJson(regret),
        _loss:loss,_regret:regret
      });
    }
    let worstLoss=perScenario[0]._loss,worstJoint=perScenario[0]._regret;
    for(const row of perScenario.slice(1)){
      if(compare(row._loss,worstLoss)>0) worstLoss=row._loss;
      if(compare(row._regret,worstJoint)>0) worstJoint=row._regret;
    }
    const worstJointScenarioIds=perScenario.filter(row=>compare(row._regret,worstJoint)===0).map(row=>row.scenario_id);
    const rectByWorld=worldIds.map(worldId=>{
      const upper=maxByWorldAction.get(action.action_id+"\u0000"+worldId);
      const lower=minByWorld.get(worldId);
      return {world_id:worldId,regret:subtract(upper,lower,"rectangular regret")};
    });
    let worstRect=rectByWorld[0].regret;
    for(const row of rectByWorld.slice(1)) if(compare(row.regret,worstRect)>0) worstRect=row.regret;
    const worstRectWorlds=rectByWorld.filter(row=>compare(row.regret,worstRect)===0).map(row=>row.world_id);
    const excess=subtract(worstRect,worstJoint,"rectangular excess");
    return {
      action_id:action.action_id,
      admissibility_receipt_id:action.admissibility_receipt_id,
      worst_case_loss:worstLoss,
      worst_case_joint_regret:worstJoint,
      worst_case_rectangular_regret:worstRect,
      rectangular_regret_excess:excess,
      worst_joint_regret_scenarios:worstJointScenarioIds,
      worst_rectangular_regret_worlds:worstRectWorlds,
      uniformly_optimal_across_joint_scenarios:bestCount===scenarios.length,
      joint_scenario_best_count:bestCount,
      cost:action.cost,
      per_scenario:perScenario.map(({scenario_id,world_id,loss,joint_regret})=>({scenario_id,world_id,loss,joint_regret}))
    };
  });
  for (const metric of metrics) {
    if (compare(metric.worst_case_rectangular_regret, metric.worst_case_joint_regret) < 0) {
      fail("rectangular regret fell below joint regret for " + metric.action_id);
    }
  }
  const uniform=metrics.filter(metric=>metric.uniformly_optimal_across_joint_scenarios).map(metric=>metric.action_id);
  const jointFrontier=metrics.filter(candidate=>!metrics.some(other=>other.action_id!==candidate.action_id&&!dominates(other,candidate,false))).map(metric=>metric.action_id);
  const rectangularFrontier=metrics.filter(candidate=>!metrics.some(other=>other.action_id!==candidate.action_id&&!dominates(other,candidate,true))).map(metric=>metric.action_id);
  const modelIds=[...new Set(scenarios.map(scenario=>scenario.loss_model_id))].sort(codePointCompare);
  return {
    case_id:item.case_id,
    status:uniform.length?"UNIFORM_OPTIMUM_ACROSS_JOINT_SCENARIOS":"NO_UNIFORM_OPTIMUM_JOINT_SCENARIO_FRONTIER",
    compatible_world_set_receipt_id:item.compatible_world_set.source_receipt_id,
    compatible_world_ids_sha256:item.compatible_world_set.world_ids_sha256,
    loss_scenario_manifest_id:item.loss_scenario_manifest.source_manifest_id,
    loss_scenarios_sha256:item.loss_scenario_manifest.scenarios_sha256,
    compatible_world_count:item.compatible_world_set.world_ids.length,
    loss_scenario_count:scenarios.length,
    loss_model_ids:modelIds,
    uniformly_optimal_action_ids:uniform,
    joint_minimax_regret_action_ids:argmin(metrics,"worst_case_joint_regret"),
    rectangular_minimax_regret_action_ids:argmin(metrics,"worst_case_rectangular_regret"),
    joint_scenario_pareto_frontier_action_ids:jointFrontier,
    rectangular_pareto_frontier_action_ids:rectangularFrontier,
    excluded_actions:excluded,
    scenario_evidence_issues:issues,
    action_metrics:metrics.map(metric=>({
      ...metric,
      worst_case_loss:asJson(metric.worst_case_loss),
      worst_case_joint_regret:asJson(metric.worst_case_joint_regret),
      worst_case_rectangular_regret:asJson(metric.worst_case_rectangular_regret),
      rectangular_regret_excess:asJson(metric.rectangular_regret_excess)
    }))
  };
}
function evaluate(corpus) {
  validate(corpus);
  const cases=corpus.cases.map(evaluateCase).sort((a,b)=>codePointCompare(a.case_id,b.case_id));
  return {
    result:"pass",
    evidence_class:corpus.evidence_class,
    claim_ceiling:corpus.claim_ceiling,
    summary:{
      case_count:cases.length,
      no_admissible_action_cases:cases.filter(item=>item.status==="NO_ADMISSIBLE_ACTION").length,
      loss_model_indeterminate_cases:cases.filter(item=>item.status==="LOSS_MODEL_INDETERMINATE").length,
      out_of_profile_cases:cases.filter(item=>item.status==="OUT_OF_PROFILE").length,
      uniform_optimum_cases:cases.filter(item=>item.status==="UNIFORM_OPTIMUM_ACROSS_JOINT_SCENARIOS").length,
      no_uniform_optimum_cases:cases.filter(item=>item.status==="NO_UNIFORM_OPTIMUM_JOINT_SCENARIO_FRONTIER").length,
      admissible_action_count:cases.reduce((sum,item)=>sum+item.action_metrics.length,0),
      excluded_action_count:cases.reduce((sum,item)=>sum+item.excluded_actions.length,0),
      scenario_evidence_issue_count:cases.reduce((sum,item)=>sum+item.scenario_evidence_issues.length,0),
      joint_frontier_action_count:cases.reduce((sum,item)=>sum+item.joint_scenario_pareto_frontier_action_ids.length,0),
      rectangular_frontier_action_count:cases.reduce((sum,item)=>sum+item.rectangular_pareto_frontier_action_ids.length,0)
    },
    cases,
    physical_qualification_granted:false,
    authority_granted:false
  };
}
try {
  const [corpusPath]=process.argv.slice(2);
  if(!corpusPath) fail("usage: joint_loss_scenario_oracle.mjs <corpus.json>");
  const corpus=JSON.parse(fs.readFileSync(corpusPath,"utf8"));
  const output=evaluate(corpus);
  const reversed=structuredClone(corpus);
  reversed.cases.reverse();
  for(const item of reversed.cases){
    item.actions.reverse();
    item.compatible_world_set.world_ids.reverse();
    item.loss_scenario_manifest.scenarios.reverse();
  }
  assert.deepStrictEqual(evaluate(reversed),output,"result changed under case/action/scenario/world ordering");
  process.stdout.write(JSON.stringify(output,null,2)+"\n");
}catch(error){
  process.stderr.write(String(error?.message??error)+"\n");
  process.exitCode=1;
}
