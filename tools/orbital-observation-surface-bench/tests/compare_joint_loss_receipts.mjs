import fs from "node:fs";
import assert from "node:assert/strict";
function fail(message) { throw new Error(message); }
function rationalCompare(a,b) {
  const left = BigInt(a.numerator) * BigInt(b.denominator);
  const right = BigInt(b.numerator) * BigInt(a.denominator);
  return left < right ? -1 : left > right ? 1 : 0;
}
function serializedDominates(left,right,rectangular) {
  const regretKey = rectangular ? "worst_case_rectangular_regret" : "worst_case_joint_regret";
  const dimensions = [
    rationalCompare(left.worst_case_loss,right.worst_case_loss),
    rationalCompare(left[regretKey],right[regretKey]),
    left.cost.reference_trials - right.cost.reference_trials,
    left.cost.exposure_micro_units - right.cost.exposure_micro_units,
    left.cost.elapsed_millis - right.cost.elapsed_millis
  ];
  return dimensions.every(value=>value<=0) && dimensions.some(value=>value<0);
}
function recomputeFrontier(metrics,rectangular) {
  return metrics.filter(candidate=>!metrics.some(other=>
    other.action_id!==candidate.action_id && serializedDominates(other,candidate,rectangular)
  )).map(metric=>metric.action_id);
}
function emitFrontierDiagnostics(receipt,oracle) {
  const oracleCases = new Map(oracle.cases.map(item=>[item.case_id,item]));
  for (const actualCase of receipt.cases) {
    const expectedCase = oracleCases.get(actualCase.case_id);
    if (!expectedCase || !actualCase.action_metrics.length) continue;
    const rustJoint = actualCase.joint_scenario_pareto_frontier_action_ids;
    const jsJoint = expectedCase.joint_scenario_pareto_frontier_action_ids;
    const recomputedRustJoint = recomputeFrontier(actualCase.action_metrics,false);
    const recomputedJsJoint = recomputeFrontier(expectedCase.action_metrics,false);
    const rustRect = actualCase.rectangular_pareto_frontier_action_ids;
    const jsRect = expectedCase.rectangular_pareto_frontier_action_ids;
    const recomputedRustRect = recomputeFrontier(actualCase.action_metrics,true);
    const recomputedJsRect = recomputeFrontier(expectedCase.action_metrics,true);
    const changedMetrics = actualCase.action_metrics.map(metric=>{
      const expectedMetric=expectedCase.action_metrics.find(item=>item.action_id===metric.action_id);
      if (!expectedMetric) return {action_id:metric.action_id,missing_in_oracle:true};
      const rationalFields=["worst_case_loss","worst_case_joint_regret","worst_case_rectangular_regret","rectangular_regret_excess"];
      const mismatches=rationalFields.filter(field=>rationalCompare(metric[field],expectedMetric[field])!==0);
      for (const key of ["reference_trials","exposure_micro_units","elapsed_millis"]) {
        if (metric.cost[key] !== expectedMetric.cost[key]) mismatches.push("cost."+key);
      }
      if (metric.admissibility_receipt_id !== expectedMetric.admissibility_receipt_id) mismatches.push("admissibility_receipt_id");
      return mismatches.length ? {action_id:metric.action_id,mismatched_fields:mismatches,rust_values:Object.fromEntries(rationalFields.filter(k=>mismatches.includes(k)).map(k=>[k,metric[k]])),oracle_values:Object.fromEntries(rationalFields.filter(k=>mismatches.includes(k)).map(k=>[k,expectedMetric[k]]))} : null;
    }).filter(Boolean);
    if (JSON.stringify(rustJoint)!==JSON.stringify(jsJoint) ||
        JSON.stringify(rustRect)!==JSON.stringify(jsRect) ||
        JSON.stringify(rustJoint)!==JSON.stringify(recomputedRustJoint) ||
        JSON.stringify(jsJoint)!==JSON.stringify(recomputedJsJoint) ||
        changedMetrics.length) {
      process.stderr.write(JSON.stringify({
        case_id:actualCase.case_id,
        rust_frontier:{joint:rustJoint,rectangular:rustRect},
        js_frontier:{joint:jsJoint,rectangular:jsRect},
        recomputed_from_rust_metrics:{joint:recomputedRustJoint,rectangular:recomputedRustRect},
        recomputed_from_js_metrics:{joint:recomputedJsJoint,rectangular:recomputedJsRect},
        changedMetrics
      })+"\n");
    }
  }
}
try {
  const [receiptPath, oraclePath] = process.argv.slice(2);
  if (!receiptPath || !oraclePath) fail("usage: compare_joint_loss_receipts.mjs <rust-receipt.json> <oracle.json>");
  const receipt = JSON.parse(fs.readFileSync(receiptPath,"utf8"));
  const oracle = JSON.parse(fs.readFileSync(oraclePath,"utf8"));
  emitFrontierDiagnostics(receipt,oracle);
  if (receipt.receipt_schema !== "luminous.orbital-joint-loss-scenario.receipt.v0.1") fail("unexpected joint-loss receipt schema");
  if (!/^[0-9a-f]{40}$/.test(receipt.source_commit ?? "")) fail("receipt is not pinned to a full source commit");
  if (!/^[0-9a-f]{64}$/.test(receipt.corpus_sha256 ?? "")) fail("receipt lacks corpus SHA-256");
  for (const field of ["result","evidence_class","claim_ceiling","summary","cases","physical_qualification_granted","authority_granted"]) {
    assert.deepStrictEqual(receipt[field],oracle[field],"Rust/JavaScript mismatch in "+field);
  }
  if (receipt.physical_qualification_granted !== false || receipt.authority_granted !== false) fail("receipt attempted to grant qualification or authority");
  const correlated = receipt.cases.find(item=>item.case_id==="correlated-common-loss-shift");
  const alpha = correlated?.action_metrics?.find(item=>item.action_id==="alpha");
  if (alpha?.worst_case_joint_regret?.numerator !== 0 ||
      alpha?.worst_case_rectangular_regret?.numerator !== 1 ||
      alpha?.worst_case_rectangular_regret?.denominator !== 2) {
    fail("correlation negative control failed: expected joint regret 0 and rectangular regret 1/2");
  }
  const multiWorld = receipt.cases.find(item=>item.case_id==="multi-world-strata");
  const multiAlpha = multiWorld?.action_metrics?.find(item=>item.action_id==="alpha");
  if (multiWorld?.compatible_world_count !== 2 ||
      multiAlpha?.worst_rectangular_regret_worlds?.join(",") !== "w1") {
    fail("multi-world negative control failed: rectangular maximum must identify w1 for alpha");
  }
  const unqualified = receipt.cases.find(item=>item.case_id==="unqualified-loss-model");
  if (unqualified?.status !== "LOSS_MODEL_INDETERMINATE" || unqualified.action_metrics.length !== 0) {
    fail("unqualified loss scenario was not fail-closed");
  }
  process.stdout.write("joint-loss-rust-oracle-match: exact joint/rectangular regret outputs agree\n");
} catch(error) {
  process.stderr.write(String(error?.message ?? error)+"\n");
  process.exitCode = 1;
}
