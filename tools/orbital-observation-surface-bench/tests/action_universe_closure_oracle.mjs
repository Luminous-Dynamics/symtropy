import fs from "node:fs";
import { createHash } from "node:crypto";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
function digestIds(ids) {
  return sha256(JSON.stringify([...ids].sort()));
}
function digestActionPayloads(actions) {
  const pairs = actions.map(item => [item.action_id, item.payload_sha256])
    .sort((left, right) => left[0].localeCompare(right[0]));
  return sha256(JSON.stringify(pairs));
}
function asSet(values) { return new Set(values); }
function equalSet(left, right) {
  return left.size === right.size && [...left].every(value => right.has(value));
}
function difference(left, right) {
  return [...left].filter(value => !right.has(value)).sort();
}
function intersection(left, right) {
  return new Set([...left].filter(value => right.has(value)));
}
function evaluateCase(item) {
  if (digestIds(item.expected_partition_ids) !== item.expected_partition_ids_sha256) {
    throw new Error(`${item.case_id}: planned partition digest mismatch`);
  }
  if (digestIds(item.expected_action_ids) !== item.expected_action_ids_sha256) {
    throw new Error(`${item.case_id}: planned action digest mismatch`);
  }
  if (digestIds(item.expected_exclusion_ids) !== item.expected_exclusion_ids_sha256) {
    throw new Error(`${item.case_id}: planned exclusion digest mismatch`);
  }
  if (digestActionPayloads(item.actions) !== item.expected_action_payloads_sha256) {
    throw new Error(`${item.case_id}: action payload digest mismatch`);
  }

  const expectedPartitions = asSet(item.expected_partition_ids);
  const observedPartitions = asSet(item.partitions.map(partition => partition.partition_id));
  const missingPartitionIds = difference(expectedPartitions, observedPartitions);
  const extraPartitionIds = difference(observedPartitions, expectedPartitions);

  const expectedActions = asSet(item.expected_action_ids);
  const observedActions = asSet(item.actions.map(action => action.action_id));
  const actionInventoryMatches = equalSet(expectedActions, observedActions);

  const expectedExclusions = asSet(item.expected_exclusion_ids);
  const observedExclusions = asSet(item.exclusions.map(exclusion => exclusion.exclusion_id));
  const exclusionInventoryMatches = equalSet(expectedExclusions, observedExclusions);

  const actionById = new Map(item.actions.map(action => [action.action_id, action]));
  const exclusionById = new Map(item.exclusions.map(exclusion => [exclusion.exclusion_id, exclusion]));
  let partitionSemanticsValid = true;
  const unwitnessedExclusionIds = [];

  for (const partition of item.partitions) {
    if (partition.classification === "indeterminate") partitionSemanticsValid = false;
    const listedActions = asSet(partition.action_ids);
    const witnessedActions = asSet(item.actions
      .filter(action => action.partition_id === partition.partition_id)
      .map(action => action.action_id));
    if (!equalSet(listedActions, witnessedActions)) partitionSemanticsValid = false;
    if (partition.classification === "present" && witnessedActions.size === 0) {
      partitionSemanticsValid = false;
    }
    if (partition.classification === "excluded" || partition.classification === "not_applicable") {
      if (partition.exclusion_ids.length === 0) {
        partitionSemanticsValid = false;
        for (const exclusion of item.exclusions) {
          if (exclusion.partition_id === partition.partition_id) unwitnessedExclusionIds.push(exclusion.exclusion_id);
        }
      }
      for (const exclusionId of partition.exclusion_ids) {
        const exclusion = exclusionById.get(exclusionId);
        if (!(exclusion
          && exclusion.partition_id === partition.partition_id
          && exclusion.reason_code.trim().length > 0
          && exclusion.evidence_receipt_id.trim().length > 0
          && exclusion.negative_space_witness_receipt_id.trim().length > 0)) {
          unwitnessedExclusionIds.push(exclusionId);
        }
      }
    } else if (partition.exclusion_ids.length > 0) {
      partitionSemanticsValid = false;
    }
    for (const actionId of partition.action_ids) {
      if (!actionById.has(actionId)) partitionSemanticsValid = false;
    }
  }

  const actionPartitionIds = asSet(item.actions.map(action => action.partition_id));
  if ([...actionPartitionIds].some(partitionId => !observedPartitions.has(partitionId))) {
    partitionSemanticsValid = false;
  }

  if (!exclusionInventoryMatches) {
    unwitnessedExclusionIds.push(...difference(expectedExclusions, observedExclusions));
  }
  const uniqueUnwitnessed = [...new Set(unwitnessedExclusionIds)].sort();

  const commonDependencies = new Set();
  for (let i = 0; i < item.enumerators.length; i += 1) {
    const left = new Set([...item.enumerators[i].dependency_ids,
      `registry:${item.enumerators[i].source_registry_id}`]);
    for (let j = i + 1; j < item.enumerators.length; j += 1) {
      const right = new Set([...item.enumerators[j].dependency_ids,
        `registry:${item.enumerators[j].source_registry_id}`]);
      for (const dependency of intersection(left, right)) commonDependencies.add(dependency);
    }
  }

  const enumeratorReports = [];
  const enumeratorSets = [];
  const methods = new Set();
  const algorithms = new Set();
  const registries = new Set();
  let enumeratorScopeMatches = true;
  for (const enumerator of item.enumerators) {
    const ids = asSet(enumerator.action_ids);
    const matched = equalSet(ids, expectedActions);
    enumeratorSets.push(ids);
    methods.add(enumerator.method_id);
    algorithms.add(enumerator.algorithm_sha256);
    registries.add(enumerator.source_registry_id);
    if (enumerator.scope_id !== item.scope.scope_id
      || enumerator.scope_snapshot_sha256 !== item.scope.snapshot_sha256) {
      enumeratorScopeMatches = false;
    }
    enumeratorReports.push({
      enumerator_id: enumerator.enumerator_id,
      method_id: enumerator.method_id,
      source_registry_id: enumerator.source_registry_id,
      action_count: enumerator.action_ids.length,
      matched_planned_inventory: matched
    });
  }

  const enumeratorsAreDistinct = item.enumerators.length >= 2
    && methods.size === item.enumerators.length
    && algorithms.size === item.enumerators.length
    && registries.size === item.enumerators.length
    && commonDependencies.size === 0;
  const enumeratorAgreement = enumeratorSets.length > 0
    && enumeratorSets.every(set => equalSet(set, expectedActions))
    && enumeratorSets.slice(1).every((set, index) => equalSet(set, enumeratorSets[index]));

  let status;
  if (item.scope.boundary_kind !== "finite_bounded") status = "CLOSURE_INDETERMINATE";
  else if (item.scope.snapshot_sha256 !== item.scope.current_snapshot_sha256) status = "SCOPE_DRIFT";
  else if (!enumeratorScopeMatches) status = "STALE_SNAPSHOT";
  else if (missingPartitionIds.length > 0 || extraPartitionIds.length > 0) status = "PARTITION_OMITTED";
  else if (!partitionSemanticsValid || uniqueUnwitnessed.length > 0) status = "NEGATIVE_SPACE_UNWITNESSED";
  else if (enumeratorSets.length > 0 && !enumeratorsAreDistinct) status = "COMMON_CAUSE_UNRESOLVED";
  else if (!actionInventoryMatches || !enumeratorAgreement || item.enumerators.length < 2) status = "ENUMERATOR_DISAGREEMENT";
  else status = "CLOSED_WITHIN_DECLARED_SCOPE";

  return {
    case_id: item.case_id,
    status,
    scope_id: item.scope.scope_id,
    scope_snapshot_sha256: item.scope.snapshot_sha256,
    expected_partition_count: item.expected_partition_ids.length,
    observed_partition_count: item.partitions.length,
    expected_action_count: item.expected_action_ids.length,
    observed_action_count: item.actions.length,
    expected_exclusion_count: item.expected_exclusion_ids.length,
    observed_exclusion_count: item.exclusions.length,
    common_dependency_ids: [...commonDependencies].sort(),
    enumerator_reports: enumeratorReports,
    missing_partition_ids: missingPartitionIds,
    unwitnessed_exclusion_ids: uniqueUnwitnessed,
    physical_qualification_granted: false,
    authority_granted: false
  };
}

function run(path) {
  const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
  if (corpus.schema !== "luminous.orbital-action-closure.corpus.v0.1"
    || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== "finite-source-bound-action-closure-structure-only"
    || !Array.isArray(corpus.cases) || corpus.cases.length < 1 || corpus.cases.length > 128) {
    throw new Error("unsupported action-closure corpus or claim ceiling");
  }
  const cases = corpus.cases.map(evaluateCase);
  const count = status => cases.filter(item => item.status === status).length;
  return {
    result: cases.every(item => item.status === "CLOSED_WITHIN_DECLARED_SCOPE")
      ? "pass" : "adversarial-controls-detected",
    summary: {
      case_count: cases.length,
      closed_within_declared_scope_cases: count("CLOSED_WITHIN_DECLARED_SCOPE"),
      enumerator_disagreement_cases: count("ENUMERATOR_DISAGREEMENT"),
      partition_omitted_cases: count("PARTITION_OMITTED"),
      negative_space_unwitnessed_cases: count("NEGATIVE_SPACE_UNWITNESSED"),
      common_cause_unresolved_cases: count("COMMON_CAUSE_UNRESOLVED"),
      scope_drift_cases: count("SCOPE_DRIFT"),
      stale_snapshot_cases: count("STALE_SNAPSHOT"),
      indeterminate_cases: count("CLOSURE_INDETERMINATE")
    },
    cases,
    physical_qualification_granted: false,
    authority_granted: false
  };
}

try {
  const input = process.argv[2];
  if (!input) throw new Error("usage: action_universe_closure_oracle.mjs <corpus.json>");
  process.stdout.write(JSON.stringify(run(input), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
