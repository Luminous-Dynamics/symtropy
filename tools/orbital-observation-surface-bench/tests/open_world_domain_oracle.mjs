import fs from "node:fs";
import { createHash } from "node:crypto";

const CLAIM = "independent-finite-domain-grammar-falsification-only";
const SCHEMA = "luminous.orbital-open-world-domain.corpus.v0.1";

function fail(message) { throw new Error(message); }
function sha256(value) { return createHash("sha256").update(value).digest("hex"); }
function validId(value) {
  return typeof value === "string" && value.trim().length > 0 && /^[\x00-\x7F]+$/.test(value);
}
function validSha(value) { return typeof value === "string" && /^[0-9a-f]{64}$/.test(value); }
function uniqueIds(values, label) {
  if (!Array.isArray(values) || values.length > 4096
    || values.some(value => !validId(value)) || new Set(values).size !== values.length) {
    fail(label + " contains invalid or duplicate IDs");
  }
}
function payloadDigest(actionId) {
  return sha256("orbital-domain-action:v0.1:" + actionId);
}
function overlaps(domain, component, operation, actor) {
  const start = Math.max(domain.window_start_ms, component.active_start_ms,
    operation.active_start_ms, actor.active_start_ms);
  const end = Math.min(domain.window_end_ms, component.active_end_ms,
    operation.active_end_ms, actor.active_end_ms);
  return start < end;
}

// Independent Cartesian reference enumerator; it does not read either candidate
// enumerator's action list.
function generateDomainActions(domain) {
  const generated = [];
  for (const component of domain.components) {
    for (const operation of domain.operations) {
      if (!component.capability_ids.includes(operation.required_capability_id)
        || component.resource_capacity_milli < operation.resource_required_milli) continue;
      for (const actor of domain.actors) {
        if (!actor.role_ids.includes(operation.required_role_id)
          || !overlaps(domain, component, operation, actor)) continue;
        const actionId = "component:" + component.component_id
          + "|operation:" + operation.operation_id + "|actor:" + actor.actor_id;
        generated.push({ action_id: actionId, payload_sha256: payloadDigest(actionId) });
      }
    }
  }
  generated.sort((a, b) => a.action_id.localeCompare(b.action_id));
  return generated;
}

function validateDomain(domain) {
  if (!validId(domain.domain_id) || !validId(domain.grammar_version)
    || !validSha(domain.snapshot_sha256) || !validSha(domain.current_snapshot_sha256)
    || !Number.isSafeInteger(domain.window_start_ms) || !Number.isSafeInteger(domain.window_end_ms)
    || domain.window_start_ms >= domain.window_end_ms
    || !Array.isArray(domain.components) || !domain.components.length || domain.components.length > 512
    || !Array.isArray(domain.operations) || !domain.operations.length || domain.operations.length > 512
    || !Array.isArray(domain.actors) || !domain.actors.length || domain.actors.length > 512) {
    fail("invalid bounded domain manifest");
  }
  uniqueIds(domain.components.map(x => x.component_id), "component IDs");
  for (const component of domain.components) {
    uniqueIds(component.capability_ids, "component capabilities");
    if (!validId(component.component_id) || !Number.isSafeInteger(component.resource_capacity_milli)
      || !Number.isSafeInteger(component.active_start_ms) || !Number.isSafeInteger(component.active_end_ms)
      || component.active_start_ms >= component.active_end_ms) fail("invalid component row");
  }
  uniqueIds(domain.operations.map(x => x.operation_id), "operation IDs");
  for (const operation of domain.operations) {
    if (!validId(operation.operation_id) || !validId(operation.required_capability_id)
      || !validId(operation.required_role_id) || !Number.isSafeInteger(operation.resource_required_milli)
      || !Number.isSafeInteger(operation.active_start_ms) || !Number.isSafeInteger(operation.active_end_ms)
      || operation.active_start_ms >= operation.active_end_ms) fail("invalid operation row");
  }
  uniqueIds(domain.actors.map(x => x.actor_id), "actor IDs");
  for (const actor of domain.actors) {
    uniqueIds(actor.role_ids, "actor roles");
    if (!validId(actor.actor_id) || !Number.isSafeInteger(actor.active_start_ms)
      || !Number.isSafeInteger(actor.active_end_ms) || actor.active_start_ms >= actor.active_end_ms) {
      fail("invalid actor row");
    }
  }
}

function validateEnumerator(item) {
  if (!validId(item.enumerator_id) || !validId(item.method_id) || !validId(item.source_id)
    || !validSha(item.snapshot_sha256) || !validId(item.execution_receipt_id)) {
    fail("enumerator receipt metadata is invalid");
  }
  uniqueIds(item.action_ids, "enumerator action inventory");
}
function difference(left, right) {
  return [...left].filter(id => !right.has(id)).sort();
}

function evaluateCase(item) {
  if (!validId(item.case_id)) fail("invalid domain case ID");
  validateDomain(item.domain);
  validateEnumerator(item.registry_enumerator);
  validateEnumerator(item.model_enumerator);
  if (!Array.isArray(item.external_challenges) || item.external_challenges.length > 4096) {
    fail("external challenge inventory exceeds bound");
  }

  const generated = generateDomainActions(item.domain);
  const generatedIds = new Set(generated.map(x => x.action_id));
  const registryIds = new Set(item.registry_enumerator.action_ids);
  const modelIds = new Set(item.model_enumerator.action_ids);
  const registryMissing = difference(generatedIds, registryIds);
  const modelMissing = difference(generatedIds, modelIds);
  const registryExtra = difference(registryIds, generatedIds);
  const modelExtra = difference(modelIds, generatedIds);
  const jointlyOmitted = registryMissing.filter(id => modelMissing.includes(id));

  const challengeIds = new Set();
  for (const challenge of item.external_challenges) {
    if (!validId(challenge.action_id) || !validSha(challenge.payload_sha256)
      || !validId(challenge.in_scope_evidence_receipt_id)
      || challenge.payload_sha256 !== payloadDigest(challenge.action_id)
      || challengeIds.has(challenge.action_id)) {
      fail("external challenge witness is invalid or duplicated");
    }
    challengeIds.add(challenge.action_id);
  }
  const externalChallengeActionIds = difference(challengeIds, generatedIds);

  const enumeratorSnapshotMatches = item.registry_enumerator.snapshot_sha256 === item.domain.snapshot_sha256
    && item.model_enumerator.snapshot_sha256 === item.domain.snapshot_sha256;
  const enumeratorReports = [item.registry_enumerator, item.model_enumerator].map(enumerator => ({
    enumerator_id: enumerator.enumerator_id,
    method_id: enumerator.method_id,
    source_id: enumerator.source_id,
    action_count: new Set(enumerator.action_ids).size,
    matched_generated_domain: new Set(enumerator.action_ids).size === generatedIds.size
      && [...generatedIds].every(id => enumerator.action_ids.includes(id))
  }));

  let status;
  if (item.domain.snapshot_sha256 !== item.domain.current_snapshot_sha256) status = "DOMAIN_SNAPSHOT_DRIFT";
  else if (!enumeratorSnapshotMatches) status = "STALE_ENUMERATION";
  else if (externalChallengeActionIds.length) status = "DOMAIN_MODEL_INADEQUATE";
  else if (registryExtra.length || modelExtra.length) status = "ENUMERATOR_OUTSIDE_DOMAIN";
  else if (jointlyOmitted.length) status = "COMMON_MODE_ACTION_OMISSION";
  else if (registryMissing.length || modelMissing.length) status = "ENUMERATOR_DISAGREEMENT";
  else status = "CLOSED_WITHIN_GENERATED_DOMAIN";

  return {
    case_id: item.case_id,
    status,
    domain_id: item.domain.domain_id,
    snapshot_sha256: item.domain.snapshot_sha256,
    generated_action_count: generated.length,
    registry_missing_action_ids: registryMissing,
    model_missing_action_ids: modelMissing,
    registry_extra_action_ids: registryExtra,
    model_extra_action_ids: modelExtra,
    jointly_omitted_action_ids: jointlyOmitted,
    external_challenge_action_ids: externalChallengeActionIds,
    enumerator_reports: enumeratorReports,
    physical_qualification_granted: false,
    authority_granted: false
  };
}

function evaluate(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM || !Array.isArray(corpus.cases)
    || !corpus.cases.length || corpus.cases.length > 128) {
    fail("unsupported open-world domain corpus or claim ceiling");
  }
  const ids = new Set();
  const cases = corpus.cases.map(item => {
    if (ids.has(item.case_id)) fail("duplicate open-world domain case ID");
    ids.add(item.case_id);
    return evaluateCase(item);
  }).sort((a, b) => a.case_id.localeCompare(b.case_id));
  const count = status => cases.filter(item => item.status === status).length;
  const allClosed = cases.every(item => item.status === "CLOSED_WITHIN_GENERATED_DOMAIN");
  return {
    result: allClosed ? "pass" : "adversarial-controls-detected",
    evidence_class: "synthetic",
    claim_ceiling: CLAIM,
    summary: {
      case_count: cases.length,
      closed_within_generated_domain_cases: count("CLOSED_WITHIN_GENERATED_DOMAIN"),
      common_mode_omission_cases: count("COMMON_MODE_ACTION_OMISSION"),
      enumerator_disagreement_cases: count("ENUMERATOR_DISAGREEMENT"),
      domain_model_inadequate_cases: count("DOMAIN_MODEL_INADEQUATE"),
      stale_snapshot_cases: count("DOMAIN_SNAPSHOT_DRIFT") + count("STALE_ENUMERATION"),
      enumerator_outside_domain_cases: count("ENUMERATOR_OUTSIDE_DOMAIN")
    },
    cases,
    physical_qualification_granted: false,
    authority_granted: false
  };
}

try {
  const inputPath = process.argv[2];
  if (!inputPath) fail("usage: open_world_domain_oracle.mjs <corpus.json>");
  const corpus = JSON.parse(fs.readFileSync(inputPath, "utf8"));
  process.stdout.write(JSON.stringify(evaluate(corpus), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exitCode = 1;
}
