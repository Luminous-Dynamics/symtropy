import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";

const SCHEMA = "luminous.orbital-incumbent-world-set-bridge.receipt.v0.1";
const CLAIM_CEILING = "source-receipt-structure-and-inventory-composition-only";
const SHA = /^[0-9a-f]{64}$/;
const COMMIT = /^[0-9a-f]{40}$/;

function sha256(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function validId(value) {
  return typeof value === "string" && value.trim().length > 0
    && [...value].every(ch => {
      const code = ch.codePointAt(0);
      return (code >= 33 && code <= 126) || code === 32;
    });
}

function sortedUniqueIds(value) {
  return Array.isArray(value)
    && value.every(validId)
    && new Set(value).size === value.length
    && value.every((item, index) => index === 0 || value[index - 1] < item);
}

function digestMatches(ids, digest) {
  return SHA.test(digest) && sha256(JSON.stringify(ids)) === digest;
}

function hasExactKeys(value, expectedKeys) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function validateSchemaValue(value, schema, path = "$", errors = []) {
  const type = schema.type;
  if (type === "object" && (!value || typeof value !== "object" || Array.isArray(value))) {
    errors.push(path + ": expected object");
    return errors;
  }
  if (type === "array" && !Array.isArray(value)) {
    errors.push(path + ": expected array");
    return errors;
  }
  if (type === "string" && typeof value !== "string") {
    errors.push(path + ": expected string");
    return errors;
  }
  if (type === "integer" && (!Number.isSafeInteger(value))) {
    errors.push(path + ": expected safe integer");
    return errors;
  }
  if (type === "boolean" && typeof value !== "boolean") {
    errors.push(path + ": expected boolean");
    return errors;
  }
  if (Object.hasOwn(schema, "const") && value !== schema.const) errors.push(path + ": const mismatch");
  if (schema.enum && !schema.enum.includes(value)) errors.push(path + ": enum mismatch");
  if (type === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) errors.push(path + ": minLength");
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) errors.push(path + ": pattern");
  }
  if (type === "integer") {
    if (schema.minimum !== undefined && value < schema.minimum) errors.push(path + ": minimum");
    if (schema.maximum !== undefined && value > schema.maximum) errors.push(path + ": maximum");
  }
  if (type === "array") {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(path + ": minItems");
    if (schema.uniqueItems && new Set(value.map(item => JSON.stringify(item))).size !== value.length) {
      errors.push(path + ": uniqueItems");
    }
    if (schema.items) value.forEach((item, index) => validateSchemaValue(item, schema.items, path + "[" + index + "]", errors));
  }
  if (type === "object") {
    for (const key of schema.required ?? []) {
      if (!Object.hasOwn(value, key)) errors.push(path + ": missing " + key);
    }
    const properties = schema.properties ?? {};
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!Object.hasOwn(properties, key)) errors.push(path + ": unknown property " + key);
      }
    }
    for (const [key, propertySchema] of Object.entries(properties)) {
      if (Object.hasOwn(value, key)) validateSchemaValue(value[key], propertySchema, path + "." + key, errors);
    }
  }
  return errors;
}

function validFraction(value) {
  return value && Number.isSafeInteger(value.numerator)
    && Number.isSafeInteger(value.denominator)
    && value.numerator > 0 && value.denominator > 0;
}

function alphaWithinBudget(total, allocated) {
  if (!hasExactKeys(total, ["numerator", "denominator"])
    || !hasExactKeys(allocated, ["numerator", "denominator"])
    || !validFraction(total) || !validFraction(allocated)) return false;
  const totalN = BigInt(total.numerator);
  const totalD = BigInt(total.denominator);
  const allocatedN = BigInt(allocated.numerator);
  const allocatedD = BigInt(allocated.denominator);
  return totalN <= totalD && allocatedN * totalD <= totalN * allocatedD;
}

function invalid(reason) { return { status: "INVALID_INPUT", reason }; }
function abstain(reason) { return { status: "ABSTAIN", reason }; }

function validateReceipt(receipt) {
  if (!hasExactKeys(receipt, ["schema", "evidence_class", "claim_ceiling", "source", "contract", "cutoff",
    "world_inventory", "qualification", "adapter_disposition", "authority_granted"])
    || receipt.schema !== SCHEMA || receipt.evidence_class !== "synthetic"
    || receipt.claim_ceiling !== CLAIM_CEILING || receipt.authority_granted !== false
    || !["CONSUMABLE", "ABSTAIN"].includes(receipt.adapter_disposition)) {
    return invalid("envelope identity, claim ceiling, disposition or authority field is invalid");
  }

  const source = receipt.source;
  if (!hasExactKeys(source, ["owner_issue", "receipt_schema", "claim_ceiling", "source_commit",
    "source_case_id", "receipt_id", "receipt_sha256"]) || source.owner_issue !== "#6887"
    || typeof source.receipt_schema !== "string" || !source.receipt_schema.trim()
    || typeof source.claim_ceiling !== "string" || !source.claim_ceiling.trim()
    || !COMMIT.test(source.source_commit || "")
    || !validId(source.source_case_id) || !validId(source.receipt_id)
    || !SHA.test(source.receipt_sha256 || "")) {
    return invalid("upstream source identity or receipt pin is malformed");
  }

  const contract = receipt.contract;
  if (!hasExactKeys(contract, ["guarantee_type", "guarantee_disposition", "optional_stopping_covered",
    "predictable_selection", "adaptive_selection_covered", "process_id", "look_index", "stopping_state",
    "alpha_total", "alpha_allocated", "alpha_allocation_id", "alpha_allocation_sha256", "filtration_sha256",
    "selection_policy_sha256", "conditional_kernel_sha256"]) || !validId(contract.process_id) || !Number.isSafeInteger(contract.look_index)
    || contract.look_index < 1 || !["ACTIVE", "STOPPED"].includes(contract.stopping_state)
    || !["QUALIFIED", "UNQUALIFIED"].includes(contract.guarantee_disposition)
    || typeof contract.optional_stopping_covered !== "boolean"
    || typeof contract.predictable_selection !== "boolean"
    || typeof contract.adaptive_selection_covered !== "boolean"
    || !validId(contract.alpha_allocation_id)
    || !SHA.test(contract.alpha_allocation_sha256 || "")
    || !SHA.test(contract.filtration_sha256 || "")
    || !SHA.test(contract.selection_policy_sha256 || "")
    || !SHA.test(contract.conditional_kernel_sha256 || "")) {
    return invalid("sequential process, filtration, policy or kernel commitment is malformed");
  }
  if (!alphaWithinBudget(contract.alpha_total, contract.alpha_allocated)) {
    return invalid("allocated alpha is invalid or exceeds the declared campaign budget");
  }

  const cutoff = receipt.cutoff;
  if (!hasExactKeys(cutoff, ["cutoff_id", "event_watermark_sequence", "observed_max_sequence",
    "history_sha256", "knowledge_time_utc"]) || !validId(cutoff.cutoff_id)
    || !Number.isSafeInteger(cutoff.event_watermark_sequence)
    || cutoff.event_watermark_sequence < 0
    || !Number.isSafeInteger(cutoff.observed_max_sequence)
    || cutoff.observed_max_sequence < 0
    || cutoff.observed_max_sequence > cutoff.event_watermark_sequence
    || !SHA.test(cutoff.history_sha256 || "")
    || typeof cutoff.knowledge_time_utc !== "string"
    || !cutoff.knowledge_time_utc.endsWith("Z")
    || !Number.isFinite(Date.parse(cutoff.knowledge_time_utc))) {
    return invalid("decision cutoff or evidence-history boundary is malformed");
  }

  const inventory = receipt.world_inventory;
  if (!hasExactKeys(inventory, ["candidate_world_ids", "candidate_world_ids_sha256",
    "surviving_world_ids", "surviving_world_ids_sha256", "excluded_world_ids", "excluded_world_ids_sha256"])
    || !sortedUniqueIds(inventory.candidate_world_ids)
    || inventory.candidate_world_ids.length === 0
    || !sortedUniqueIds(inventory.surviving_world_ids)
    || !sortedUniqueIds(inventory.excluded_world_ids)
    || !digestMatches(inventory.candidate_world_ids, inventory.candidate_world_ids_sha256)
    || !digestMatches(inventory.surviving_world_ids, inventory.surviving_world_ids_sha256)
    || !digestMatches(inventory.excluded_world_ids, inventory.excluded_world_ids_sha256)) {
    return invalid("world inventory is not canonical, unique and digest-consistent");
  }

  const candidate = new Set(inventory.candidate_world_ids);
  const surviving = new Set(inventory.surviving_world_ids);
  const excluded = new Set(inventory.excluded_world_ids);
  if ([...surviving].some(id => excluded.has(id))
    || surviving.size + excluded.size !== candidate.size
    || [...surviving, ...excluded].some(id => !candidate.has(id))) {
    return invalid("surviving and excluded inventories do not partition the candidate set");
  }

  const qualification = receipt.qualification;
  if (!hasExactKeys(qualification, ["source_state", "model_family_state", "channel_dependence_state",
    "action_realization_state"])
    || !["CURRENT", "STALE", "REVOKED", "CONTRADICTED", "INDETERMINATE"].includes(qualification.source_state)
    || !["ADEQUATE", "INADEQUATE", "INDETERMINATE"].includes(qualification.model_family_state)
    || !["ADEQUATE", "UNRESOLVED", "INADEQUATE"].includes(qualification.channel_dependence_state)
    || !["VERIFIED", "NOT_REQUIRED_BY_PROFILE", "UNKNOWN", "CONTRADICTED"].includes(qualification.action_realization_state)) {
    return invalid("qualification object has unknown fields or malformed state enums");
  }

  let computed;
  if (contract.guarantee_type !== "ANYTIME_VALID_WORLD_EXCLUSION") {
    computed = abstain("UPSTREAM_NOT_ANYTIME_VALID");
  } else if (contract.guarantee_disposition !== "QUALIFIED") {
    computed = abstain("UPSTREAM_GUARANTEE_NOT_QUALIFIED");
  } else if (contract.optional_stopping_covered !== true) {
    computed = abstain("OPTIONAL_STOPPING_NOT_COVERED");
  } else if (contract.predictable_selection !== true || contract.adaptive_selection_covered !== true) {
    computed = abstain("ADAPTIVE_SELECTION_NOT_COVERED");
  } else if (receipt.qualification?.source_state !== "CURRENT") {
    computed = abstain("SOURCE_NOT_CURRENT");
  } else if (receipt.qualification?.model_family_state !== "ADEQUATE") {
    computed = abstain("MODEL_FAMILY_NOT_ADEQUATE");
  } else if (receipt.qualification?.channel_dependence_state !== "ADEQUATE") {
    computed = abstain("CHANNEL_DEPENDENCE_NOT_ADEQUATE");
  } else if (!["VERIFIED", "NOT_REQUIRED_BY_PROFILE"].includes(
    receipt.qualification?.action_realization_state)) {
    computed = abstain("ACTION_REALIZATION_NOT_QUALIFIED");
  } else if (inventory.surviving_world_ids.length === 0) {
    computed = abstain("EMPTY_SURVIVING_WORLD_SET");
  } else {
    computed = { status: "CONSUMABLE", reason: "SOURCE_BOUND_WORLD_SET_READY_FOR_DOWNSTREAM_GATE" };
  }

  if (receipt.adapter_disposition !== computed.status) {
    return invalid("declared adapter disposition does not match independently derived disposition");
  }
  return computed;
}

function clone(value) { return JSON.parse(JSON.stringify(value)); }
function setDigest(ids) { return sha256(JSON.stringify(ids)); }

const [fixturePath, schemaPath] = process.argv.slice(2);
if (!fixturePath || !schemaPath) {
  throw new Error("usage: validate_anytime_world_set_bridge.mjs <synthetic-receipt.json> <receipt-schema.json>");
}
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const schemaErrors = validateSchemaValue(fixture, schema);
assert.deepEqual(schemaErrors, [], "frozen source receipt must validate against the strict JSON Schema");
const tests = [
  ["unknown top-level property", r => {
    r.unreviewed_field = "must reject";
    return r;
  }, "INVALID_INPUT", "envelope identity, claim ceiling, disposition or authority field is invalid"],
  ["missing source receipt digest", r => {
    delete r.source.receipt_sha256;
    return r;
  }, "INVALID_INPUT", "upstream source identity or receipt pin is malformed"],
  ["unknown nested contract property", r => {
    r.contract.unreviewed_field = true;
    return r;
  }, "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"],
  ["malformed stopping state enum", r => {
    r.contract.stopping_state = "MAYBE";
    return r;
  }, "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"],
  ["malformed guarantee disposition enum", r => {
    r.contract.guarantee_disposition = "MOSTLY_QUALIFIED";
    return r;
  }, "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"],
  ["malformed model qualification enum", r => {
    r.qualification.model_family_state = "PROBABLY_ADEQUATE";
    return r;
  }, "INVALID_INPUT", "qualification object has unknown fields or malformed state enums"],
  ["non-string adapter disposition", r => {
    r.adapter_disposition = [];
    return r;
  }, "INVALID_INPUT", "envelope identity, claim ceiling, disposition or authority field is invalid"],
  ["non-string guarantee disposition", r => {
    r.contract.guarantee_disposition = [];
    return r;
  }, "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"],
  ["non-string qualification state", r => {
    r.qualification.source_state = [];
    return r;
  }, "INVALID_INPUT", "qualification object has unknown fields or malformed state enums"],
  ["qualified adaptive source", r => r, "CONSUMABLE", "SOURCE_BOUND_WORLD_SET_READY_FOR_DOWNSTREAM_GATE"],
  ["empty survivor set abstains", r => {
    r.world_inventory.surviving_world_ids = [];
    r.world_inventory.surviving_world_ids_sha256 = setDigest([]);
    r.world_inventory.excluded_world_ids = [...r.world_inventory.candidate_world_ids];
    r.world_inventory.excluded_world_ids_sha256 = setDigest(r.world_inventory.excluded_world_ids);
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "EMPTY_SURVIVING_WORLD_SET"],
  ["fixed-look test is not anytime-valid", r => {
    r.contract.guarantee_type = "FIXED_LOOK_ONLY";
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "UPSTREAM_NOT_ANYTIME_VALID"],
  ["optional stopping is not covered", r => {
    r.contract.optional_stopping_covered = false;
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "OPTIONAL_STOPPING_NOT_COVERED"],
  ["adaptive selection is not covered", r => {
    r.contract.adaptive_selection_covered = false;
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "ADAPTIVE_SELECTION_NOT_COVERED"],
  ["alpha allocation exceeds campaign budget", r => {
    r.contract.alpha_allocated = { numerator: 1, denominator: 10 };
    return r;
  }, "INVALID_INPUT", "allocated alpha is invalid or exceeds the declared campaign budget"],
  ["survivor digest tampering", r => {
    r.world_inventory.surviving_world_ids_sha256 = "0".repeat(64);
    return r;
  }, "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"],
  ["duplicate survivor identity", r => {
    r.world_inventory.surviving_world_ids = ["world-A", "world-A"];
    return r;
  }, "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"],
  ["survivor/excluded overlap", r => {
    r.world_inventory.excluded_world_ids = ["world-B"];
    r.world_inventory.excluded_world_ids_sha256 = setDigest(["world-B"]);
    return r;
  }, "INVALID_INPUT", "surviving and excluded inventories do not partition the candidate set"],
  ["future evidence after cutoff", r => {
    r.cutoff.observed_max_sequence = r.cutoff.event_watermark_sequence + 1;
    return r;
  }, "INVALID_INPUT", "decision cutoff or evidence-history boundary is malformed"],
  ["stale source receipt", r => {
    r.qualification.source_state = "STALE";
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "SOURCE_NOT_CURRENT"],
  ["inadequate model family", r => {
    r.qualification.model_family_state = "INADEQUATE";
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "MODEL_FAMILY_NOT_ADEQUATE"],
  ["unresolved common-cause channel", r => {
    r.qualification.channel_dependence_state = "UNRESOLVED";
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "CHANNEL_DEPENDENCE_NOT_ADEQUATE"],
  ["unknown action realization", r => {
    r.qualification.action_realization_state = "UNKNOWN";
    r.adapter_disposition = "ABSTAIN";
    return r;
  }, "ABSTAIN", "ACTION_REALIZATION_NOT_QUALIFIED"],
  ["missing selection-policy commitment", r => {
    r.contract.selection_policy_sha256 = null;
    return r;
  }, "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"],
  ["malformed source receipt digest", r => {
    r.source.receipt_sha256 = "not-a-digest";
    return r;
  }, "INVALID_INPUT", "upstream source identity or receipt pin is malformed"],
  ["unsorted world inventory", r => {
    r.world_inventory.candidate_world_ids = ["world-B", "world-A"];
    return r;
  }, "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"]
];

const results = [];
for (const [name, mutate, expectedStatus, expectedReason] of tests) {
  const candidate = mutate(clone(fixture));
  const actual = validateReceipt(candidate);
  const passed = actual.status === expectedStatus && actual.reason === expectedReason;
  results.push({
    case: name,
    status: actual.status,
    reason: actual.reason,
    expected_status: expectedStatus,
    expected_reason: expectedReason,
    passed
  });
  assert.equal(actual.status, expectedStatus, name + ": status");
  assert.equal(actual.reason, expectedReason, name + ": reason");
}
const report = {
  oracle: "node-bigint-independent-reference",
  case_count: results.length,
  expected_result_mismatches: results.filter(item => !item.passed).length,
  authority_grants: 0,
  result: results.every(item => item.passed) ? "pass" : "fail",
  cases: results
};
process.stdout.write(JSON.stringify(report, null, 2) + "\\n");
process.stderr.write("node-anytime-world-set-oracle: " + tests.length
  + " cases passed; BigInt exactness; no authority granted\\n");
