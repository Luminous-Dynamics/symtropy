import fs from "node:fs";
import { createHash } from "node:crypto";

const [schemaPath, corpusPath] = process.argv.slice(2);
if (!schemaPath || !corpusPath) {
  throw new Error("usage: validate_incumbent_adequacy_contract.mjs <schema> <corpus>");
}
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
const sha256 = value => createHash("sha256").update(value, "utf8").digest("hex");
const digestIds = ids => sha256(JSON.stringify([...ids].sort()));
function projectedDigest(scenarios) {
  const canonical = scenarios.map(scenario => ({
    scenario_id: scenario.scenario_id,
    world_id: scenario.world_id,
    loss_model_id: scenario.loss_model_id,
    evidence_status: scenario.evidence_status,
    evidence_receipt_id: scenario.evidence_receipt_id,
    loss_unit: scenario.loss_unit,
    scope_sha256: scenario.scope_sha256,
    feasible_action_ids: [...scenario.feasible_action_ids].sort(),
    losses: [...scenario.losses].sort((a, b) =>
      (a.action_id < b.action_id ? -1 : a.action_id > b.action_id ? 1 : 0)
      || a.source_numerator - b.source_numerator
      || a.source_denominator - b.source_denominator
      || a.loss - b.loss).map(loss => ({
        action_id: loss.action_id,
        source_numerator: loss.source_numerator,
        source_denominator: loss.source_denominator,
        loss: loss.loss
      }))
  })).sort((a, b) => a.scenario_id < b.scenario_id ? -1 : a.scenario_id > b.scenario_id ? 1 : 0);
  return sha256(JSON.stringify(canonical));
}

if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema"
  || schema.properties?.schema?.const !== "luminous.orbital-incumbent-adequacy.corpus.v0.1"
  || schema.properties?.evidence_class?.const !== "synthetic"
  || schema.properties?.claim_ceiling?.const !== "synthetic-finite-world-incumbent-adequacy-only") {
  throw new Error("incumbent adequacy corpus schema contract is malformed");
}
if (corpus.schema !== schema.properties.schema.const
  || corpus.evidence_class !== schema.properties.evidence_class.const
  || corpus.claim_ceiling !== schema.properties.claim_ceiling.const
  || !Array.isArray(corpus.cases) || corpus.cases.length !== 30) {
  throw new Error("incumbent adequacy corpus does not match its v0.1 contract");
}
const caseIds = new Set();
for (const item of corpus.cases) {
  if (!item.case_id || caseIds.has(item.case_id) || !Array.isArray(item.action_ids)
    || !Array.isArray(item.expected_world_ids) || !Array.isArray(item.worlds)
    || !item.loss_model_set || !item.expected) {
    throw new Error("invalid incumbent adequacy case structure");
  }
  caseIds.add(item.case_id);
  const modelSet = item.loss_model_set;

  if (new Set(item.expected_world_ids).size !== item.expected_world_ids.length
    || digestIds(item.expected_world_ids) !== item.expected_world_ids_sha256
    || new Set(modelSet.expected_scenario_ids).size !== modelSet.expected_scenario_ids.length
    || digestIds(modelSet.expected_scenario_ids) !== modelSet.expected_scenario_ids_sha256) {
    throw new Error("physical-world or joint-scenario inventory digest mismatch in " + item.case_id);
  }

  const identityProjection = modelSet.projection_id === "exact-rational-to-integer-identity-v1"
    && modelSet.source_loss_unit === item.loss_unit
    && modelSet.projection_scale_numerator === 1
    && modelSet.projection_scale_denominator === 1;
  const normalizedProjection = modelSet.projection_id === "normalized-loss-times-1000-exact-v1"
    && modelSet.source_loss_unit === "normalized_loss"
    && item.loss_unit === "loss_micro_units"
    && modelSet.projection_scale_numerator === 1000
    && modelSet.projection_scale_denominator === 1;
  const unsupportedProjectionControl = item.case_id === "unknown_projection_id"
    && item.expected?.disposition === "INVALID_INPUT"
    && item.expected?.trigger_reason === "unsupported loss unit projection contract";
  if (!identityProjection && !normalizedProjection && !unsupportedProjectionControl) {
    throw new Error("unapproved loss-unit projection in " + item.case_id);
  }

  if (item.world_set_receipt_schema !== "luminous.orbital-decision-identifiability.receipt.v0.1"
    || item.world_set_claim_ceiling !== "exact-finite-world-decision-set-algebra-only"
    || !/^[0-9a-f]{40}$/.test(item.world_set_source_commit)
    || !item.world_set_source_case_id
    || !/^[0-9a-f]{64}$/.test(item.world_set_receipt_sha256)
    || !/^[0-9a-f]{40}$/.test(modelSet.source_commit)
    || !/^[0-9a-f]{64}$/.test(modelSet.source_receipt_sha256)
    || !/^[0-9a-f]{64}$/.test(modelSet.source_scenarios_sha256)
    || !/^[0-9a-f]{64}$/.test(modelSet.projected_scenarios_sha256)
    || typeof modelSet.source_loss_unit !== "string" || !modelSet.source_loss_unit
    || typeof modelSet.projection_id !== "string" || !modelSet.projection_id
    || !Number.isSafeInteger(modelSet.projection_scale_numerator) || modelSet.projection_scale_numerator <= 0
    || !Number.isSafeInteger(modelSet.projection_scale_denominator) || modelSet.projection_scale_denominator <= 0
    || modelSet.source_receipt_schema !== "luminous.orbital-joint-loss-scenario.receipt.v0.1"
    || modelSet.source_claim_ceiling !== "finite-joint-loss-scenario-regret-algebra-only") {
    throw new Error("joint loss-model source receipt or projection pins invalid in " + item.case_id);
  }

  const scenarioIds = new Set();
  const worldIds = new Set();
  for (const scenario of item.worlds) {
    if (!scenario.scenario_id || scenarioIds.has(scenario.scenario_id)
      || !scenario.world_id || !scenario.loss_model_id
      || !["admissible", "inadmissible", "indeterminate", "out_of_profile"].includes(scenario.evidence_status)
      || !/^[0-9a-f]{64}$/.test(scenario.scope_sha256)
      || typeof scenario.loss_unit !== "string" || !scenario.loss_unit
      || !Array.isArray(scenario.feasible_action_ids) || !Array.isArray(scenario.losses)) {
      throw new Error("invalid joint physical-world/loss-model row in " + item.case_id);
    }
    scenarioIds.add(scenario.scenario_id);
    worldIds.add(scenario.world_id);

    const lossActionIds = new Set();
    for (const row of scenario.losses) {
      if (!Number.isSafeInteger(row.loss) || row.loss < 0 || !row.action_id
        || !item.action_ids.includes(row.action_id) || lossActionIds.has(row.action_id)
        || !Number.isSafeInteger(row.source_numerator) || row.source_numerator < 0
        || !Number.isSafeInteger(row.source_denominator) || row.source_denominator <= 0) {
        throw new Error("invalid source or projected loss row in " + item.case_id + "/" + scenario.scenario_id);
      }
      lossActionIds.add(row.action_id);
      const numerator = BigInt(row.source_numerator) * BigInt(modelSet.projection_scale_numerator);
      const denominator = BigInt(row.source_denominator) * BigInt(modelSet.projection_scale_denominator);
      const exact = numerator % denominator === 0n ? numerator / denominator : null;
      if (exact === null) {
        if (item.case_id !== "inexact_loss_projection"
          || item.expected?.trigger_reason !== "source rational loss cannot be projected exactly into the declared loss unit") {
          throw new Error("unexpected non-integral source rational projection in " + item.case_id + "/" + scenario.scenario_id);
        }
      } else if (exact > 9223372036854775807n || exact !== BigInt(row.loss)) {
        if (item.case_id !== "projected_loss_mismatch"
          || item.expected?.trigger_reason !== "integer loss does not match exact source rational projection") {
          throw new Error("integer loss differs from exact source rational projection in " + item.case_id);
        }
      }
    }

    const missingLossActions = item.action_ids.filter(actionId => !lossActionIds.has(actionId));
    if (missingLossActions.length) {
      if (item.case_id !== "incomplete_loss_matrix"
        || item.expected?.trigger_reason !== "INCOMPLETE_LOSS_MATRIX") {
        throw new Error("joint loss vector omits declared actions in " + item.case_id);
      }
    } else if (item.expected?.trigger_reason === "INCOMPLETE_LOSS_MATRIX") {
      throw new Error("incomplete loss-matrix negative control is no longer incomplete");
    }
  }

  if (scenarioIds.size !== modelSet.expected_scenario_ids.length
    || modelSet.expected_scenario_ids.some(id => !scenarioIds.has(id))) {
    throw new Error("observed joint scenarios differ from planned inventory in " + item.case_id);
  }
  if (worldIds.size !== item.expected_world_ids.length
    || item.expected_world_ids.some(id => !worldIds.has(id))) {
    throw new Error("joint scenarios do not cover exact physical-world inventory in " + item.case_id);
  }
  if (projectedDigest(item.worlds) !== modelSet.projected_scenarios_sha256) {
    throw new Error("projected scenario digest mismatch in " + item.case_id);
  }
}

const multi = corpus.cases.find(item => item.case_id === "multi_loss_models_same_world");
if (!multi || multi.worlds.length !== 2
  || new Set(multi.worlds.map(world => world.world_id)).size !== 1
  || new Set(multi.worlds.map(world => world.loss_model_id)).size !== 2) {
  throw new Error("multi-loss-model / one-physical-world positive control is missing");
}
const infeasible = corpus.cases.find(item => item.case_id === "incumbent_infeasible");
const excludedAction = infeasible?.worlds.find(world => world.feasible_action_ids.length === 1
  && !world.feasible_action_ids.includes("A"));
if (!excludedAction || !excludedAction.losses.some(loss => loss.action_id === "A")) {
  throw new Error("excluded action loss must remain present in the complete source loss vector");
}
const fractional = corpus.cases.find(item => item.case_id === "fractional_loss_projection");
if (!fractional || fractional.loss_model_set.source_loss_unit !== "normalized_loss"
  || fractional.loss_model_set.projection_scale_numerator !== 1000
  || fractional.loss_model_set.projection_scale_denominator !== 1) {
  throw new Error("exact fractional rational projection positive control is missing");
}
const stale = corpus.cases.find(item => item.case_id === "loss_model_set_stale");
if (!stale || stale.world_set_state !== "CURRENT" || stale.loss_model_set.state !== "STALE") {
  throw new Error("loss-model staleness must be independent of world-set currentness");
}
process.stdout.write("incumbent-adequacy-corpus-valid: " + corpus.cases.length
  + " cases; exact rational projection and complete joint loss vectors verified\n");
