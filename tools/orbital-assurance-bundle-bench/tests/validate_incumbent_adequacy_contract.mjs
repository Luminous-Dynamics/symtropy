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
      a.action_id < b.action_id ? -1 : a.action_id > b.action_id ? 1 : 0)
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
  || !Array.isArray(corpus.cases) || corpus.cases.length !== 26) {
  throw new Error("incumbent adequacy corpus does not match its v0.1 contract");
}
const ids = new Set();
for (const item of corpus.cases) {
  if (!item.case_id || ids.has(item.case_id) || !Array.isArray(item.action_ids)
    || !Array.isArray(item.expected_world_ids) || !Array.isArray(item.worlds)
    || !item.loss_model_set || !item.expected) {
    throw new Error("invalid incumbent adequacy case structure");
  }
  ids.add(item.case_id);
  if (new Set(item.expected_world_ids).size !== item.expected_world_ids.length
    || digestIds(item.expected_world_ids) !== item.expected_world_ids_sha256
    || new Set(item.loss_model_set.expected_scenario_ids).size !== item.loss_model_set.expected_scenario_ids.length
    || digestIds(item.loss_model_set.expected_scenario_ids) !== item.loss_model_set.expected_scenario_ids_sha256) {
    throw new Error("physical-world or joint-scenario inventory digest mismatch in " + item.case_id);
  }
  if (!/^[0-9a-f]{64}$/.test(item.world_set_receipt_sha256)
    || !/^[0-9a-f]{40}$/.test(item.loss_model_set.source_commit)
    || !/^[0-9a-f]{64}$/.test(item.loss_model_set.source_receipt_sha256)
    || !/^[0-9a-f]{64}$/.test(item.loss_model_set.source_scenarios_sha256)
    || !/^[0-9a-f]{64}$/.test(item.loss_model_set.projected_scenarios_sha256)
    || item.loss_model_set.source_receipt_schema !== "luminous.orbital-joint-loss-scenario.receipt.v0.1"
    || item.loss_model_set.source_claim_ceiling !== "finite-joint-loss-scenario-regret-algebra-only") {
    throw new Error("joint loss-model source receipt pins invalid in " + item.case_id);
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
    for (const row of scenario.losses) {
      if (!Number.isSafeInteger(row.loss) || row.loss < 0 || !row.action_id) {
        throw new Error("invalid loss row in " + item.case_id + "/" + scenario.scenario_id);
      }
    }
  }
  if (scenarioIds.size !== item.loss_model_set.expected_scenario_ids.length
    || item.loss_model_set.expected_scenario_ids.some(id => !scenarioIds.has(id))) {
    throw new Error("observed joint scenarios differ from planned inventory in " + item.case_id);
  }
  if (worldIds.size !== item.expected_world_ids.length
    || item.expected_world_ids.some(id => !worldIds.has(id))) {
    throw new Error("joint scenarios do not cover exact physical-world inventory in " + item.case_id);
  }
  if (projectedDigest(item.worlds) !== item.loss_model_set.projected_scenarios_sha256) {
    throw new Error("projected scenario digest mismatch in " + item.case_id);
  }
}
const multi = corpus.cases.find(item => item.case_id === "multi_loss_models_same_world");
if (!multi || multi.worlds.length !== 2
  || new Set(multi.worlds.map(world => world.world_id)).size !== 1
  || new Set(multi.worlds.map(world => world.loss_model_id)).size !== 2) {
  throw new Error("multi-loss-model / one-physical-world positive control is missing");
}
const stale = corpus.cases.find(item => item.case_id === "loss_model_set_stale");
if (!stale || stale.world_set_state !== "CURRENT" || stale.loss_model_set.state !== "STALE") {
  throw new Error("loss-model staleness must be independent of world-set currentness");
}
process.stdout.write("incumbent-adequacy-corpus-valid: " + corpus.cases.length
  + " cases; separate physical-world and joint loss-model inventories verified\n");
