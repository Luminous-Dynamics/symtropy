import fs from "node:fs";

const SCHEMA = "luminous.orbital-incumbent-adequacy.corpus.v0.1";
const CLAIM_CEILING = "synthetic-finite-world-incumbent-adequacy-only";
const MAX_CASES = 128;
const MAX_ACTIONS = 4096;
const MAX_WORLDS = 4096;
const MAX_ROWS = 16384;

function cmp(a, b) { return a < b ? -1 : a > b ? 1 : 0; }
function validId(value) {
  return typeof value === "string" && value.trim().length > 0
    && [...value].every(ch => {
      const code = ch.codePointAt(0);
      return (code >= 33 && code <= 126) || code === 32;
    });
}
function validSha(value) { return typeof value === "string" && /^[0-9a-f]{64}$/.test(value); }
function result(item, disposition, triggerReason, threshold, metrics = {}, worldAssessments = []) {
  const comparison = {
    reference_common_action_id: metrics.reference_common_action_id ?? null,
    current_worst_case_loss: metrics.current_worst_case_loss ?? null,
    reference_common_worst_case_loss: metrics.reference_common_worst_case_loss ?? null,
    worst_case_improvement: metrics.worst_case_improvement ?? null
  };
  return {
    case_id: item.case_id,
    disposition,
    trigger_reason: triggerReason,
    loss_unit: item.loss_unit,
    materiality_margin: item.materiality_margin,
    switching_cost: item.switching_cost,
    retention_threshold: threshold,
    ...comparison,
    world_assessments: worldAssessments,
    authority_granted: false,
    expected_result_matched: item.expected.disposition === disposition
      && item.expected.trigger_reason === triggerReason
      && item.expected.worst_case_improvement === comparison.worst_case_improvement
  };
}
function invalidInput(item, reason) {
  return result(item, "INVALID_INPUT", reason, null, {});
}
function abstain(item, reason, threshold) {
  return result(item, "ABSTAIN", reason, threshold, {});
}
function validGateStates(item) {
  return ["CURRENT", "STALE", "UNAVAILABLE"].includes(item.world_set_state)
    && ["CLOSED_WITHIN_DECLARED_SCOPE", "OPEN", "STALE", "INADEQUATE"].includes(item.domain_closure_state)
    && ["GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR", "MISSING_DEPENDENCY_EDGE",
      "COMMON_MODE_DEPENDENCY_OMISSION", "CONSUMER_MANIFEST_OMISSION", "UNDECLARED_DEPENDENCY_EDGE",
      "DEPENDENCY_MODEL_INADEQUATE", "DEPENDENCY_SOURCE_UNAVAILABLE", "DEPENDENCY_SCOPE_MISMATCH",
      "INVALID_GRAPH", "STALE", "INDETERMINATE"].includes(item.dependency_graph_state)
    && ["ADEQUATE", "INADEQUATE"].includes(item.model_family_state)
    && ["CURRENT", "STALE", "CONTRADICTED", "INDETERMINATE"].includes(item.evidence_state);
}
function evaluateCase(item) {
  if (!validId(item.case_id) || !validId(item.loss_unit) || !validId(item.incumbent_action_id)
    || !validSha(item.scope_sha256) || !item.action_ids.length || item.action_ids.length > MAX_ACTIONS
    || item.worlds.length > MAX_WORLDS || !Number.isSafeInteger(item.materiality_margin)
    || !Number.isSafeInteger(item.switching_cost) || item.materiality_margin < 0
    || item.switching_cost < 0 || !validGateStates(item)) {
    if (Number.isSafeInteger(item.materiality_margin) && Number.isSafeInteger(item.switching_cost)) {
      return invalidInput(item, "invalid case identity, bounds, threshold or gate state");
    }
    return invalidInput(item, "materiality margin plus switching cost overflows loss units");
  }

  const threshold = item.materiality_margin + item.switching_cost;
  if (!Number.isSafeInteger(threshold)) {
    return invalidInput(item, "materiality margin plus switching cost overflows loss units");
  }

  const actions = new Set();
  for (const actionId of item.action_ids) {
    if (!validId(actionId) || actions.has(actionId)) {
      return invalidInput(item, "action universe contains an empty or duplicate action ID");
    }
    actions.add(actionId);
  }
  if (!actions.has(item.incumbent_action_id)) {
    return invalidInput(item, "incumbent action is absent from the declared finite action universe");
  }

  if (item.worlds.length === 0) return abstain(item, "NO_SURVIVING_WORLDS", threshold);
  if (item.worlds.length > MAX_WORLDS) return invalidInput(item, "surviving-world set exceeds bound");

  if (item.world_set_state === "UNAVAILABLE") return abstain(item, "WORLD_SET_UNAVAILABLE", threshold);
  if (item.world_set_state === "STALE") return abstain(item, "STALE_WORLD_SET", threshold);
  if (item.domain_closure_state === "OPEN") return abstain(item, "DOMAIN_NOT_CLOSED", threshold);
  if (item.domain_closure_state === "STALE") return abstain(item, "STALE_DOMAIN_CLOSURE", threshold);
  if (item.domain_closure_state === "INADEQUATE" || item.model_family_state === "INADEQUATE") {
    return abstain(item, "MODEL_FAMILY_INADEQUATE", threshold);
  }
  if (item.dependency_graph_state !== "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
    && item.dependency_graph_state !== "STALE"
    && item.dependency_graph_state !== "INDETERMINATE") {
    return abstain(item, "DEPENDENCY_GRAPH_INADEQUATE", threshold);
  }
  if (item.dependency_graph_state === "STALE") {
    return abstain(item, "STALE_DEPENDENCY_GRAPH", threshold);
  }
  if (item.dependency_graph_state === "INDETERMINATE") {
    return abstain(item, "INDETERMINATE_DEPENDENCY_GRAPH", threshold);
  }
  if (item.evidence_state === "STALE") return abstain(item, "STALE_EVIDENCE", threshold);
  if (item.evidence_state === "CONTRADICTED") return abstain(item, "CONTRADICTED_EVIDENCE", threshold);
  if (item.evidence_state === "INDETERMINATE") return abstain(item, "INDETERMINATE_EVIDENCE", threshold);

  const worldIds = new Set();
  let totalRows = 0;
  let incumbentInfeasible = false;
  const validatedWorlds = [];

  for (const world of item.worlds) {
    if (!validId(world.world_id) || worldIds.has(world.world_id)
      || !validId(world.loss_unit) || !validSha(world.scope_sha256)) {
      return invalidInput(item, "world identity is duplicate, empty or malformed");
    }
    worldIds.add(world.world_id);
    if (world.loss_unit !== item.loss_unit) return abstain(item, "LOSS_UNIT_MISMATCH", threshold);
    if (world.scope_sha256 !== item.scope_sha256) return abstain(item, "SCOPE_MISMATCH", threshold);
    if (world.feasible_action_ids.length === 0) {
      return abstain(item, "NO_FEASIBLE_ACTION_IN_WORLD", threshold);
    }

    const feasible = new Set();
    for (const actionId of world.feasible_action_ids) {
      if (!actions.has(actionId) || feasible.has(actionId)) {
        return invalidInput(item, "world feasible set contains an unknown or duplicate action");
      }
      feasible.add(actionId);
    }

    totalRows += world.losses.length;
    if (totalRows > MAX_ROWS) return invalidInput(item, "loss matrix exceeds row bound");

    const losses = new Map();
    for (const row of world.losses) {
      if (!validId(row.action_id) || !Number.isSafeInteger(row.loss) || row.loss < 0
        || !feasible.has(row.action_id) || losses.has(row.action_id)) {
        return invalidInput(item, "loss row is malformed, duplicated or not feasible in its world");
      }
      losses.set(row.action_id, row.loss);
    }
    for (const actionId of feasible) {
      if (!losses.has(actionId)) return abstain(item, "INCOMPLETE_LOSS_MATRIX", threshold);
    }

    const incumbentFeasible = feasible.has(item.incumbent_action_id);
    incumbentInfeasible ||= !incumbentFeasible;
    validatedWorlds.push({
      world_id: world.world_id,
      feasible_action_ids: feasible,
      losses,
      incumbent_feasible: incumbentFeasible,
      incumbent_loss: losses.has(item.incumbent_action_id) ? losses.get(item.incumbent_action_id) : null
    });
  }

  // An infeasible incumbent is a review trigger, not permission to select a replacement.
  if (incumbentInfeasible) {
    const assessments = validatedWorlds.map(world => ({
      world_id: world.world_id,
      incumbent_feasible: world.incumbent_feasible,
      incumbent_loss: world.incumbent_loss,
      reference_common_action_loss: null,
      incumbent_minus_reference_loss: null
    })).sort((a, b) => cmp(a.world_id, b.world_id));
    return result(item, "REOPTIMIZATION_WARRANTED",
      "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD", threshold, {}, assessments);
  }

  // Static non-anticipative comparator: choose one action feasible in every surviving world.
  let commonActions = [...validatedWorlds[0].feasible_action_ids];
  for (const world of validatedWorlds.slice(1)) {
    commonActions = commonActions.filter(actionId => world.feasible_action_ids.has(actionId));
  }
  commonActions.sort(cmp);
  if (!commonActions.length || !commonActions.includes(item.incumbent_action_id)) {
    return invalidInput(item, "common feasible action set is empty despite incumbent feasibility");
  }

  let currentWorstCaseLoss = 0;
  for (const world of validatedWorlds) {
    currentWorstCaseLoss = Math.max(currentWorstCaseLoss, world.losses.get(item.incumbent_action_id));
  }

  // This common action is a benchmark comparator only, not a recommendation.
  let referenceCommonActionId = null;
  let referenceCommonWorstCaseLoss = null;
  for (const actionId of commonActions) {
    let worstLoss = 0;
    for (const world of validatedWorlds) {
      const loss = world.losses.get(actionId);
      if (loss === undefined) return abstain(item, "INCOMPLETE_LOSS_MATRIX", threshold);
      worstLoss = Math.max(worstLoss, loss);
    }
    if (referenceCommonWorstCaseLoss === null || worstLoss < referenceCommonWorstCaseLoss
      || (worstLoss === referenceCommonWorstCaseLoss && cmp(actionId, referenceCommonActionId) < 0)) {
      referenceCommonActionId = actionId;
      referenceCommonWorstCaseLoss = worstLoss;
    }
  }
  const worstCaseImprovement = currentWorstCaseLoss - referenceCommonWorstCaseLoss;
  if (!Number.isSafeInteger(worstCaseImprovement) || worstCaseImprovement < 0) {
    return invalidInput(item, "reference common action is worse than incumbent in worst-case loss");
  }

  const assessments = validatedWorlds.map(world => {
    const incumbentLoss = world.losses.get(item.incumbent_action_id);
    const referenceLoss = world.losses.get(referenceCommonActionId);
    return {
      world_id: world.world_id,
      incumbent_feasible: true,
      incumbent_loss: incumbentLoss,
      reference_common_action_loss: referenceLoss,
      incumbent_minus_reference_loss: incumbentLoss - referenceLoss
    };
  }).sort((a, b) => cmp(a.world_id, b.world_id));

  const metrics = {
    reference_common_action_id: referenceCommonActionId,
    current_worst_case_loss: currentWorstCaseLoss,
    reference_common_worst_case_loss: referenceCommonWorstCaseLoss,
    worst_case_improvement: worstCaseImprovement
  };
  if (worstCaseImprovement > threshold) {
    return result(item, "REOPTIMIZATION_WARRANTED",
      "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD", threshold, metrics, assessments);
  }
  return result(item, "RETAIN_INCUMBENT",
    "WITHIN_FINITE_WORLD_RETENTION_ENVELOPE", threshold, metrics, assessments);
}
function evaluate(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM_CEILING || !Array.isArray(corpus.cases)
    || corpus.cases.length === 0 || corpus.cases.length > MAX_CASES) {
    throw new Error("unsupported incumbent-adequacy corpus or claim ceiling");
  }
  const ids = new Set();
  const cases = corpus.cases.map(item => {
    if (ids.has(item.case_id)) throw new Error("incumbent-adequacy case IDs must be unique");
    ids.add(item.case_id);
    return evaluateCase(item);
  }).sort((a, b) => cmp(a.case_id, b.case_id));
  const count = disposition => cases.filter(item => item.disposition === disposition).length;
  const summary = {
    case_count: cases.length,
    retain_incumbent_cases: count("RETAIN_INCUMBENT"),
    reoptimization_warranted_cases: count("REOPTIMIZATION_WARRANTED"),
    abstain_cases: count("ABSTAIN"),
    invalid_input_cases: count("INVALID_INPUT"),
    incumbent_infeasible_cases: cases.filter(item =>
      item.trigger_reason === "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD").length,
    threshold_exceeded_cases: cases.filter(item =>
      item.trigger_reason === "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD").length,
    expected_result_mismatches: cases.filter(item => !item.expected_result_matched).length,
    authority_grants: cases.filter(item => item.authority_granted).length
  };
  const pass = summary.expected_result_mismatches === 0
    && summary.retain_incumbent_cases >= 1
    && summary.reoptimization_warranted_cases >= 1
    && summary.abstain_cases >= 1
    && summary.authority_grants === 0;
  return {
    result: pass ? "pass" : "fail",
    evidence_class: "synthetic",
    claim_ceiling: CLAIM_CEILING,
    summary,
    cases,
    authority_granted: false
  };
}

try {
  const path = process.argv[2];
  if (!path) throw new Error("usage: incumbent_adequacy_oracle.mjs <cases.json>");
  process.stdout.write(JSON.stringify(evaluate(JSON.parse(fs.readFileSync(path, "utf8"))), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exitCode = 1;
}
