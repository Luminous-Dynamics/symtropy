// Independent BigInt oracle for exact finite-world loss/regret frontiers.
import fs from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE = 9_007_199_254_740_991n;
function fail(message) { throw new Error(message); }
function codePointCompare(left, right) {
  const a = Array.from(left, value => value.codePointAt(0));
  const b = Array.from(right, value => value.codePointAt(0));
  const length = Math.min(a.length, b.length);
  for (let index = 0; index < length; index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index];
  }
  return a.length - b.length;
}
function gcd(a, b) {
  while (b !== 0n) { const r = a % b; a = b; b = r; }
  return a > 0n ? a : 1n;
}
function fraction(value, path) {
  if (!Number.isSafeInteger(value?.numerator) ||
      !Number.isSafeInteger(value?.denominator) ||
      value.numerator < 0 || value.denominator <= 0 ||
      BigInt(value.numerator) > MAX_SAFE ||
      BigInt(value.denominator) > MAX_SAFE ||
      value.numerator > value.denominator) fail(path + ": invalid exact unit-interval rational");
  return { n: BigInt(value.numerator), d: BigInt(value.denominator), original: value };
}
function compare(a, b) {
  const left = a.n * b.d, right = b.n * a.d;
  return left < right ? -1 : left > right ? 1 : 0;
}
function subtract(a, b, path) {
  const common = gcd(a.d, b.d);
  const scaleLeft = b.d / common, scaleRight = a.d / common;
  const left = a.n * scaleLeft, right = b.n * scaleRight;
  if (left < right) fail(path + ": regret underflow");
  const denominator = a.d * scaleLeft, numerator = left - right;
  const divisor = gcd(numerator, denominator);
  const n = numerator / divisor, d = denominator / divisor;
  if (n > MAX_SAFE || d > MAX_SAFE) fail(path + ": rational exceeds portable integer envelope");
  return { n, d };
}
function asJson(value) {
  return { numerator: Number(value.n), denominator: Number(value.d) };
}
function worldDigest(ids) {
  const bytes = JSON.stringify([...ids].sort());
  return createHash("sha256").update(bytes, "utf8").digest("hex");
}
function validate(corpus) {
  if (corpus?.schema !== "luminous.orbital-decision-regret-frontier.corpus.v0.1" ||
      corpus?.evidence_class !== "synthetic" ||
      corpus?.claim_ceiling !== "finite-compatible-world-loss-and-regret-algebra-only" ||
      !Array.isArray(corpus.cases) || corpus.cases.length < 1 || corpus.cases.length > 256) {
    fail("unsupported decision-regret corpus or claim ceiling");
  }
  const caseIds = new Set();
  for (const [index, item] of corpus.cases.entries()) {
    const path = "cases[" + index + "]";
    if (typeof item.case_id !== "string" || !item.case_id.trim() || caseIds.has(item.case_id)) fail(path + ": invalid or duplicate case ID");
    caseIds.add(item.case_id);
    const set = item.compatible_world_set;
    if (typeof set?.source_receipt_id !== "string" || !set.source_receipt_id.trim() ||
        !Array.isArray(set.world_ids) || set.world_ids.length < 1 || set.world_ids.length > 64 ||
        set.world_ids.some(id => typeof id !== "string" || !id.trim()) ||
        new Set(set.world_ids).size !== set.world_ids.length) fail(path + ": invalid compatible-world set");
    if (set.world_ids_sha256 !== worldDigest(set.world_ids)) fail(path + ": compatible-world ID digest mismatch");
    if (!Array.isArray(item.actions) || item.actions.length < 1 || item.actions.length > 64) fail(path + ": invalid action count");
    const actionIds = new Set();
    for (const [actionIndex, action] of item.actions.entries()) {
      const actionPath = path + ".actions[" + actionIndex + "]";
      if (typeof action.action_id !== "string" || !action.action_id.trim() || actionIds.has(action.action_id)) fail(actionPath + ": invalid or duplicate action ID");
      actionIds.add(action.action_id);
      if (!["admissible", "inadmissible", "indeterminate"].includes(action.admissibility_status)) fail(actionPath + ": invalid admissibility status");
      if (action.admissibility_status === "admissible" &&
          !(typeof action.admissibility_receipt_id === "string" && action.admissibility_receipt_id.trim())) fail(actionPath + ": admissible action lacks receipt identity");
      for (const key of ["reference_trials", "exposure_micro_units", "elapsed_millis"]) {
        if (!Number.isSafeInteger(action.cost?.[key]) || action.cost[key] < 0 || BigInt(action.cost[key]) > MAX_SAFE) fail(actionPath + ": invalid action cost " + key);
      }
      const keys = Object.keys(action.world_losses ?? {}).sort();
      if (JSON.stringify(keys) !== JSON.stringify([...set.world_ids].sort())) fail(actionPath + ": loss map must cover exactly the compatible worlds");
      for (const worldId of set.world_ids) fraction(action.world_losses[worldId], actionPath + ".world_losses." + worldId);
    }
  }
}
function minActions(metrics, key) {
  if (!metrics.length) return [];
  let best = metrics[0][key];
  for (const metric of metrics.slice(1)) if (compare(metric[key], best) < 0) best = metric[key];
  return metrics.filter(metric => compare(metric[key], best) === 0).map(metric => metric.action_id);
}
function dominates(left, right) {
  const dimensions = [
    compare(left.worst_case_loss, right.worst_case_loss),
    compare(left.worst_case_regret, right.worst_case_regret),
    left.cost.reference_trials - right.cost.reference_trials,
    left.cost.exposure_micro_units - right.cost.exposure_micro_units,
    left.cost.elapsed_millis - right.cost.elapsed_millis
  ];
  return dimensions.every(value => value <= 0) && dimensions.some(value => value < 0);
}
function evaluateCase(item) {
  const worldIds = [...item.compatible_world_set.world_ids].sort();
  const actions = [...item.actions].sort((a, b) => codePointCompare(a.action_id, b.action_id));
  const admitted = actions.filter(action => action.admissibility_status === "admissible");
  const excluded = actions.filter(action => action.admissibility_status !== "admissible")
    .map(action => ({action_id: action.action_id, admissibility_status: action.admissibility_status}));
  if (!admitted.length) {
    return {
      case_id: item.case_id,
      status: "NO_ADMISSIBLE_ACTION",
      compatible_world_set_receipt_id: item.compatible_world_set.source_receipt_id,
      compatible_world_ids_sha256: item.compatible_world_set.world_ids_sha256,
      compatible_world_count: worldIds.length,
      uniformly_optimal_action_ids: [],
      minimax_loss_action_ids: [],
      minimax_regret_action_ids: [],
      joint_pareto_frontier_action_ids: [],
      excluded_actions: excluded,
      action_metrics: []
    };
  }
  const bestLossByWorld = new Map();
  for (const worldId of worldIds) {
    let best = fraction(admitted[0].world_losses[worldId], "best loss");
    for (const action of admitted.slice(1)) {
      const loss = fraction(action.world_losses[worldId], "best loss");
      if (compare(loss, best) < 0) best = loss;
    }
    bestLossByWorld.set(worldId, best);
  }
  const metrics = admitted.map(action => {
    const perWorld = worldIds.map(worldId => {
      const loss = fraction(action.world_losses[worldId], "loss for " + action.action_id + "/" + worldId);
      const regret = subtract(loss, bestLossByWorld.get(worldId), "regret for " + action.action_id + "/" + worldId);
      return {world_id: worldId, loss: asJson(loss), regret: asJson(regret), _loss: loss, _regret: regret};
    });
    let worstLoss = perWorld[0]._loss, worstRegret = perWorld[0]._regret;
    for (const entry of perWorld.slice(1)) {
      if (compare(entry._loss, worstLoss) > 0) worstLoss = entry._loss;
      if (compare(entry._regret, worstRegret) > 0) worstRegret = entry._regret;
    }
    const worstLossWorldIds = perWorld.filter(entry => compare(entry._loss, worstLoss) === 0).map(entry => entry.world_id);
    const worstRegretWorldIds = perWorld.filter(entry => compare(entry._regret, worstRegret) === 0).map(entry => entry.world_id);
    return {
      action_id: action.action_id,
      admissibility_receipt_id: action.admissibility_receipt_id,
      worst_case_loss: worstLoss,
      worst_case_regret: worstRegret,
      worst_loss_world_ids: worstLossWorldIds,
      worst_regret_world_ids: worstRegretWorldIds,
      uniformly_optimal: perWorld.every(entry => entry._regret.n === 0n),
      cost: action.cost,
      per_world: perWorld.map(({world_id, loss, regret}) => ({world_id, loss, regret}))
    };
  });
  const uniform = metrics.filter(metric => metric.uniformly_optimal).map(metric => metric.action_id);
  const pareto = metrics.filter(candidate => !metrics.some(other => other.action_id !== candidate.action_id && dominates(other, candidate)))
    .map(metric => metric.action_id);
  return {
    case_id: item.case_id,
    status: uniform.length ? "UNIFORM_OPTIMUM_EXISTS" : "NO_UNIFORM_OPTIMUM_ROBUST_FRONTIER",
    compatible_world_set_receipt_id: item.compatible_world_set.source_receipt_id,
    compatible_world_ids_sha256: item.compatible_world_set.world_ids_sha256,
    compatible_world_count: worldIds.length,
    uniformly_optimal_action_ids: uniform,
    minimax_loss_action_ids: minActions(metrics, "worst_case_loss"),
    minimax_regret_action_ids: minActions(metrics, "worst_case_regret"),
    joint_pareto_frontier_action_ids: pareto,
    excluded_actions: excluded,
    action_metrics: metrics.map(metric => ({
      ...metric,
      worst_case_loss: asJson(metric.worst_case_loss),
      worst_case_regret: asJson(metric.worst_case_regret)
    }))
  };
}
function evaluate(corpus) {
  validate(corpus);
  const cases = corpus.cases.map(evaluateCase).sort((a, b) => codePointCompare(a.case_id, b.case_id));
  const summary = {
    case_count: cases.length,
    no_admissible_action_cases: cases.filter(item => item.status === "NO_ADMISSIBLE_ACTION").length,
    uniform_optimum_cases: cases.filter(item => item.status === "UNIFORM_OPTIMUM_EXISTS").length,
    no_uniform_optimum_cases: cases.filter(item => item.status === "NO_UNIFORM_OPTIMUM_ROBUST_FRONTIER").length,
    admissible_action_count: cases.reduce((sum, item) => sum + item.action_metrics.length, 0),
    excluded_action_count: cases.reduce((sum, item) => sum + item.excluded_actions.length, 0),
    frontier_action_count: cases.reduce((sum, item) => sum + item.joint_pareto_frontier_action_ids.length, 0)
  };
  return {
    result: "pass",
    evidence_class: corpus.evidence_class,
    claim_ceiling: corpus.claim_ceiling,
    summary,
    cases,
    physical_qualification_granted: false,
    authority_granted: false
  };
}
try {
  const [corpusPath] = process.argv.slice(2);
  if (!corpusPath) fail("usage: decision_regret_reference_oracle.mjs <corpus.json>");
  const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
  const output = evaluate(corpus);

  const permuted = structuredClone(corpus);
  permuted.cases.reverse();
  for (const item of permuted.cases) {
    item.actions.reverse();
    item.compatible_world_set.world_ids.reverse();
  }
  assert.deepStrictEqual(
    evaluate(permuted),
    output,
    "decision-regret result changed under case/action/world ordering"
  );
  process.stdout.write(JSON.stringify(output, null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
