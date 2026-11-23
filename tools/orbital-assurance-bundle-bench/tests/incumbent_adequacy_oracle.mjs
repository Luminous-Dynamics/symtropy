import fs from "node:fs";
import { createHash } from "node:crypto";

const SCHEMA = "luminous.orbital-incumbent-adequacy.corpus.v0.1";
const CLAIM_CEILING = "synthetic-finite-world-incumbent-adequacy-only";
const JOINT_LOSS_RECEIPT_SCHEMA = "luminous.orbital-joint-loss-scenario.receipt.v0.1";
const JOINT_LOSS_CLAIM_CEILING = "finite-joint-loss-scenario-regret-algebra-only";
const MAX_CASES = 128;
const MAX_ACTIONS = 4096;
const MAX_WORLDS = 4096;
const MAX_SCENARIOS = 16384;
const MAX_ROWS = 65536;

function cmp(a, b) { return a < b ? -1 : a > b ? 1 : 0; }
function validId(value) {
  return typeof value === "string" && value.trim().length > 0
    && [...value].every(ch => {
      const code = ch.codePointAt(0);
      return (code >= 33 && code <= 126) || code === 32;
    });
}
function validSha(value) { return typeof value === "string" && /^[0-9a-f]{64}$/.test(value); }
function validCommit(value) { return typeof value === "string" && /^[0-9a-f]{40}$/.test(value); }
function sha256(value) {
  return createHash("sha256").update(value, "utf8").digest("hex");
}
function digestIds(ids) { return sha256(JSON.stringify([...ids].sort(cmp))); }
function projectedScenariosDigest(scenarios) {
  const rows = scenarios.map(scenario => ({
    scenario_id: scenario.scenario_id,
    world_id: scenario.world_id,
    loss_model_id: scenario.loss_model_id,
    evidence_status: scenario.evidence_status,
    evidence_receipt_id: scenario.evidence_receipt_id,
    loss_unit: scenario.loss_unit,
    scope_sha256: scenario.scope_sha256,
    feasible_action_ids: [...scenario.feasible_action_ids].sort(cmp),
    losses: [...scenario.losses].sort((a, b) => cmp(a.action_id, b.action_id)).map(loss => ({
      action_id: loss.action_id,
      source_numerator: loss.source_numerator,
      source_denominator: loss.source_denominator,
      loss: loss.loss
    }))
  })).sort((a, b) => cmp(a.scenario_id, b.scenario_id));
  return sha256(JSON.stringify(rows));
}
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
    world_set_receipt_schema: item.world_set_receipt_schema,
    world_set_claim_ceiling: item.world_set_claim_ceiling,
    world_set_source_commit: item.world_set_source_commit,
    world_set_source_case_id: item.world_set_source_case_id,
    world_set_receipt_id: item.world_set_receipt_id,
    world_set_receipt_sha256: item.world_set_receipt_sha256,
    world_ids_sha256: item.expected_world_ids_sha256,
    world_set_state: item.world_set_state,
    loss_model_state: item.loss_model_set.state,
    loss_model_source_case_id: item.loss_model_set.source_case_id,
    loss_model_source_commit: item.loss_model_set.source_commit,
    loss_model_source_unit: item.loss_model_set.source_loss_unit,
    loss_projection_id: item.loss_model_set.projection_id,
    loss_projection_scale_numerator: item.loss_model_set.projection_scale_numerator,
    loss_projection_scale_denominator: item.loss_model_set.projection_scale_denominator,
    loss_model_source_receipt_sha256: item.loss_model_set.source_receipt_sha256,
    loss_model_manifest_id: item.loss_model_set.source_manifest_id,
    source_loss_scenarios_sha256: item.loss_model_set.source_scenarios_sha256,
    projected_scenarios_sha256: item.loss_model_set.projected_scenarios_sha256,
    joint_scenario_count: item.worlds.length,
    loss_model_ids: [...new Set(item.worlds.map(world => world.loss_model_id))].sort(cmp),
    ...comparison,
    world_assessments: worldAssessments,
    authority_granted: false,
    expected_result_matched: item.expected.disposition === disposition
      && item.expected.trigger_reason === triggerReason
      && item.expected.worst_case_improvement === comparison.worst_case_improvement
  };
}
function invalidInput(item, reason) { return result(item, "INVALID_INPUT", reason, null); }
function abstain(item, reason, threshold) { return result(item, "ABSTAIN", reason, threshold); }

function validGateStates(item) {
  return ["CURRENT", "STALE", "UNAVAILABLE"].includes(item.world_set_state)
    && ["CURRENT", "STALE", "INDETERMINATE", "OUT_OF_PROFILE", "INADEQUATE"].includes(item.loss_model_set?.state)
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
    || !validSha(item.scope_sha256) || !Array.isArray(item.action_ids) || !item.action_ids.length
    || item.action_ids.length > MAX_ACTIONS || !Array.isArray(item.expected_world_ids)
    || !item.expected_world_ids.length || item.expected_world_ids.length > MAX_WORLDS
    || !Array.isArray(item.worlds) || item.worlds.length > MAX_SCENARIOS
    || !Array.isArray(item.loss_model_set?.expected_scenario_ids)
    || item.loss_model_set.expected_scenario_ids.length > MAX_SCENARIOS
    || !Number.isSafeInteger(item.materiality_margin) || !Number.isSafeInteger(item.switching_cost)
    || item.materiality_margin < 0 || item.switching_cost < 0 || !validGateStates(item)) {
    return invalidInput(item, "invalid case identity, bounds, threshold or gate state");
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

  const modelSet = item.loss_model_set;
  if (item.world_set_receipt_schema !== "luminous.orbital-decision-identifiability.receipt.v0.1"
    || item.world_set_claim_ceiling !== "exact-finite-world-decision-set-algebra-only"
    || !validCommit(item.world_set_source_commit)
    || !validId(item.world_set_source_case_id)
    || !validId(item.world_set_receipt_id) || !validSha(item.world_set_receipt_sha256)
    || !validCommit(modelSet.source_commit) || !validSha(modelSet.source_receipt_sha256)
    || !validId(modelSet.source_case_id) || !validId(modelSet.source_manifest_id)
    || !validSha(modelSet.source_scenarios_sha256)
    || !validSha(modelSet.expected_scenario_ids_sha256)
    || !validSha(modelSet.projected_scenarios_sha256)
    || !validId(modelSet.source_loss_unit) || !validId(modelSet.projection_id)
    || !Number.isSafeInteger(modelSet.projection_scale_numerator)
    || !Number.isSafeInteger(modelSet.projection_scale_denominator)
    || modelSet.projection_scale_numerator <= 0 || modelSet.projection_scale_denominator <= 0
    || modelSet.source_receipt_schema !== JOINT_LOSS_RECEIPT_SCHEMA
    || modelSet.source_claim_ceiling !== JOINT_LOSS_CLAIM_CEILING) {
    return invalidInput(item, "upstream world or loss-model receipt pins are malformed");
  }

  const expectedWorlds = new Set(item.expected_world_ids);
  if (expectedWorlds.size !== item.expected_world_ids.length
    || [...expectedWorlds].some(id => !validId(id))
    || digestIds(item.expected_world_ids) !== item.expected_world_ids_sha256) {
    return invalidInput(item, "planned physical-world inventory or digest is invalid");
  }
  const expectedScenarios = new Set(modelSet.expected_scenario_ids);
  if (expectedScenarios.size !== modelSet.expected_scenario_ids.length
    || [...expectedScenarios].some(id => !validId(id))
    || digestIds(modelSet.expected_scenario_ids) !== modelSet.expected_scenario_ids_sha256) {
    return invalidInput(item, "planned joint loss-scenario inventory or digest is invalid");
  }
  if (!item.worlds.length) return abstain(item, "NO_SURVIVING_WORLDS", threshold);
  if (projectedScenariosDigest(item.worlds) !== modelSet.projected_scenarios_sha256) {
    return invalidInput(item, "projected joint loss-scenario digest mismatch");
  }

  const actualScenarioIds = new Set(item.worlds.map(world => world.scenario_id));
  const actualWorldIds = new Set(item.worlds.map(world => world.world_id));
  if (actualScenarioIds.size !== item.worlds.length || actualScenarioIds.size !== expectedScenarios.size
    || [...expectedScenarios].some(id => !actualScenarioIds.has(id))) {
    return invalidInput(item, "observed joint scenarios do not match planned loss-model inventory");
  }
  if (actualWorldIds.size !== expectedWorlds.size
    || [...expectedWorlds].some(id => !actualWorldIds.has(id))) {
    return invalidInput(item, "joint loss scenarios do not cover the exact surviving physical-world inventory");
  }

  if (item.world_set_state === "UNAVAILABLE") return abstain(item, "WORLD_SET_UNAVAILABLE", threshold);
  if (item.world_set_state === "STALE") return abstain(item, "STALE_WORLD_SET", threshold);
  if (modelSet.state === "STALE") return abstain(item, "STALE_LOSS_MODEL_SET", threshold);
  if (modelSet.state === "INDETERMINATE") return abstain(item, "INDETERMINATE_LOSS_MODEL_SET", threshold);
  if (modelSet.state === "OUT_OF_PROFILE") return abstain(item, "LOSS_MODEL_SET_OUT_OF_PROFILE", threshold);
  if (modelSet.state === "INADEQUATE") return abstain(item, "LOSS_MODEL_SET_INADEQUATE", threshold);
  if (item.domain_closure_state === "OPEN") return abstain(item, "DOMAIN_NOT_CLOSED", threshold);
  if (item.domain_closure_state === "STALE") return abstain(item, "STALE_DOMAIN_CLOSURE", threshold);
  if (item.domain_closure_state === "INADEQUATE" || item.model_family_state === "INADEQUATE") {
    return abstain(item, "MODEL_FAMILY_INADEQUATE", threshold);
  }
  if (item.dependency_graph_state !== "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
    && item.dependency_graph_state !== "STALE" && item.dependency_graph_state !== "INDETERMINATE") {
    return abstain(item, "DEPENDENCY_GRAPH_INADEQUATE", threshold);
  }
  if (item.dependency_graph_state === "STALE") return abstain(item, "STALE_DEPENDENCY_GRAPH", threshold);
  if (item.dependency_graph_state === "INDETERMINATE") {
    return abstain(item, "INDETERMINATE_DEPENDENCY_GRAPH", threshold);
  }
  if (item.evidence_state === "STALE") return abstain(item, "STALE_EVIDENCE", threshold);
  if (item.evidence_state === "CONTRADICTED") return abstain(item, "CONTRADICTED_EVIDENCE", threshold);
  if (item.evidence_state === "INDETERMINATE") return abstain(item, "INDETERMINATE_EVIDENCE", threshold);

  let totalRows = 0;
  let incumbentInfeasible = false;
  const validated = [];
  const scenarioIds = new Set();
  for (const scenario of item.worlds) {
    if (!validId(scenario.scenario_id) || scenarioIds.has(scenario.scenario_id)
      || !validId(scenario.world_id) || !expectedWorlds.has(scenario.world_id)
      || !validId(scenario.loss_model_id) || !validId(scenario.loss_unit)
      || !validSha(scenario.scope_sha256)
      || !["admissible", "inadmissible", "indeterminate", "out_of_profile"].includes(scenario.evidence_status)) {
      return invalidInput(item, "joint loss-scenario identity, model, status or scope is malformed");
    }
    scenarioIds.add(scenario.scenario_id);
    if (scenario.evidence_status === "out_of_profile") {
      return abstain(item, "LOSS_SCENARIO_OUT_OF_PROFILE", threshold);
    }
    if (scenario.evidence_status === "indeterminate" || scenario.evidence_status === "inadmissible") {
      return abstain(item, "LOSS_SCENARIO_EVIDENCE_INDETERMINATE", threshold);
    }
    if (scenario.evidence_receipt_id === null || scenario.evidence_receipt_id === undefined) {
      return abstain(item, "LOSS_SCENARIO_RECEIPT_MISSING", threshold);
    }
    if (!validId(scenario.evidence_receipt_id)) {
      return invalidInput(item, "joint loss-scenario evidence receipt ID is malformed");
    }
    if (scenario.loss_unit !== item.loss_unit) return abstain(item, "LOSS_UNIT_MISMATCH", threshold);
    if (scenario.scope_sha256 !== item.scope_sha256) return abstain(item, "SCOPE_MISMATCH", threshold);
    if (!Array.isArray(scenario.feasible_action_ids) || !scenario.feasible_action_ids.length) {
      return abstain(item, "NO_FEASIBLE_ACTION_IN_WORLD", threshold);
    }

    const feasible = new Set();
    for (const actionId of scenario.feasible_action_ids) {
      if (!actions.has(actionId) || feasible.has(actionId)) {
        return invalidInput(item, "world feasible set contains an unknown or duplicate action");
      }
      feasible.add(actionId);
    }
    totalRows += scenario.losses.length;
    if (totalRows > MAX_ROWS) return invalidInput(item, "loss matrix exceeds row bound");
    const losses = new Map();
    for (const row of scenario.losses) {
      if (!validId(row.action_id) || !Number.isSafeInteger(row.loss) || row.loss < 0
        || !Number.isSafeInteger(row.source_numerator) || row.source_numerator < 0
        || !Number.isSafeInteger(row.source_denominator) || row.source_denominator <= 0
        || !feasible.has(row.action_id) || losses.has(row.action_id)) {
        return invalidInput(item, "loss row is malformed, duplicated or not feasible in its joint scenario");
      }
      const scaledNumerator = BigInt(row.source_numerator)
        * BigInt(modelSet.projection_scale_numerator);
      const scaledDenominator = BigInt(row.source_denominator)
        * BigInt(modelSet.projection_scale_denominator);
      if (scaledNumerator % scaledDenominator !== 0n) {
        return invalidInput(item, "source rational loss cannot be projected exactly into the declared loss unit");
      }
      const exactProjection = scaledNumerator / scaledDenominator;
      if (exactProjection > 9223372036854775807n) {
        return invalidInput(item, "source rational loss cannot be projected exactly into the declared loss unit");
      }
      if (BigInt(row.loss) !== exactProjection) {
        return invalidInput(item, "integer loss does not match exact source rational projection");
      }
      losses.set(row.action_id, row.loss);
    }
    for (const actionId of feasible) {
      if (!losses.has(actionId)) return abstain(item, "INCOMPLETE_LOSS_MATRIX", threshold);
    }

    const incumbentFeasible = feasible.has(item.incumbent_action_id);
    incumbentInfeasible ||= !incumbentFeasible;
    validated.push({
      scenario_id: scenario.scenario_id,
      world_id: scenario.world_id,
      loss_model_id: scenario.loss_model_id,
      evidence_receipt_id: scenario.evidence_receipt_id,
      feasible_action_ids: feasible,
      losses,
      incumbent_feasible: incumbentFeasible
    });
  }
  validated.sort((a, b) => cmp(a.scenario_id, b.scenario_id));

  if (incumbentInfeasible) {
    const assessments = validated.map(scenario => ({
      scenario_id: scenario.scenario_id,
      world_id: scenario.world_id,
      loss_model_id: scenario.loss_model_id,
      evidence_receipt_id: scenario.evidence_receipt_id,
      incumbent_feasible: scenario.incumbent_feasible,
      incumbent_loss: scenario.losses.has(item.incumbent_action_id)
        ? scenario.losses.get(item.incumbent_action_id) : null,
      reference_common_action_loss: null,
      incumbent_minus_reference_loss: null
    }));
    return result(item, "REOPTIMIZATION_WARRANTED",
      "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD", threshold, {}, assessments);
  }

  // One common action must be feasible across every joint physical/loss scenario.
  let commonActions = [...validated[0].feasible_action_ids];
  for (const scenario of validated.slice(1)) {
    commonActions = commonActions.filter(actionId => scenario.feasible_action_ids.has(actionId));
  }
  commonActions.sort(cmp);
  if (!commonActions.length || !commonActions.includes(item.incumbent_action_id)) {
    return invalidInput(item, "common feasible action set is empty despite incumbent feasibility");
  }
  let currentWorstCaseLoss = 0;
  for (const scenario of validated) {
    currentWorstCaseLoss = Math.max(currentWorstCaseLoss, scenario.losses.get(item.incumbent_action_id));
  }
  let referenceCommonActionId = null;
  let referenceCommonWorstCaseLoss = null;
  for (const actionId of commonActions) {
    let worstLoss = 0;
    for (const scenario of validated) {
      const loss = scenario.losses.get(actionId);
      if (loss === undefined) return abstain(item, "INCOMPLETE_LOSS_MATRIX", threshold);
      worstLoss = Math.max(worstLoss, loss);
    }
    if (referenceCommonWorstCaseLoss === null || worstLoss < referenceCommonWorstCaseLoss
      || (worstLoss === referenceCommonWorstCaseLoss && cmp(actionId, referenceCommonActionId) < 0)) {
      referenceCommonActionId = actionId;
      referenceCommonWorstCaseLoss = worstLoss;
    }
  }
  const improvement = currentWorstCaseLoss - referenceCommonWorstCaseLoss;
  if (!Number.isSafeInteger(improvement) || improvement < 0) {
    return invalidInput(item, "reference common action is worse than incumbent in worst-case loss");
  }
  const assessments = validated.map(scenario => {
    const incumbentLoss = scenario.losses.get(item.incumbent_action_id);
    const referenceLoss = scenario.losses.get(referenceCommonActionId);
    return {
      scenario_id: scenario.scenario_id,
      world_id: scenario.world_id,
      loss_model_id: scenario.loss_model_id,
      evidence_receipt_id: scenario.evidence_receipt_id,
      incumbent_feasible: true,
      incumbent_loss: incumbentLoss,
      reference_common_action_loss: referenceLoss,
      incumbent_minus_reference_loss: incumbentLoss - referenceLoss
    };
  });
  const metrics = {
    reference_common_action_id: referenceCommonActionId,
    current_worst_case_loss: currentWorstCaseLoss,
    reference_common_worst_case_loss: referenceCommonWorstCaseLoss,
    worst_case_improvement: improvement
  };
  if (improvement > threshold) {
    return result(item, "REOPTIMIZATION_WARRANTED",
      "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD", threshold, metrics, assessments);
  }
  return result(item, "RETAIN_INCUMBENT",
    "WITHIN_FINITE_WORLD_RETENTION_ENVELOPE", threshold, metrics, assessments);
}

function evaluate(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM_CEILING || !Array.isArray(corpus.cases)
    || !corpus.cases.length || corpus.cases.length > MAX_CASES) {
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
    loss_model_set_blocked_cases: cases.filter(item =>
      ["STALE_LOSS_MODEL_SET","INDETERMINATE_LOSS_MODEL_SET","LOSS_MODEL_SET_OUT_OF_PROFILE","LOSS_MODEL_SET_INADEQUATE"]
        .includes(item.trigger_reason)).length,
    loss_scenario_evidence_blocked_cases: cases.filter(item =>
      ["LOSS_SCENARIO_OUT_OF_PROFILE","LOSS_SCENARIO_EVIDENCE_INDETERMINATE","LOSS_SCENARIO_RECEIPT_MISSING"]
        .includes(item.trigger_reason)).length,
    joint_scenario_inventory_blocked_cases: cases.filter(item =>
      item.trigger_reason.includes("joint scenario") || item.trigger_reason.includes("joint loss-scenario")
        || item.trigger_reason.includes("physical-world inventory")).length,
    expected_result_mismatches: cases.filter(item => !item.expected_result_matched).length,
    authority_grants: cases.filter(item => item.authority_granted).length
  };
  const pass = summary.expected_result_mismatches === 0
    && summary.retain_incumbent_cases >= 1
    && summary.reoptimization_warranted_cases >= 1
    && summary.abstain_cases >= 1
    && summary.loss_model_set_blocked_cases >= 1
    && summary.loss_scenario_evidence_blocked_cases >= 1
    && summary.joint_scenario_inventory_blocked_cases >= 1
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
