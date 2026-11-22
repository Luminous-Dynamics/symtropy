// Independent exact-rational oracle for the finite decision-identifiability corpus.
// This intentionally does not import or execute the Rust implementation.
import fs from "node:fs";

const MAX_SAFE = 9_007_199_254_740_991n;

function fail(message) { throw new Error(message); }

function gcd(a, b) {
  while (b !== 0n) {
    const r = a % b;
    a = b;
    b = r;
  }
  return a > 0n ? a : 1n;
}

function rational(value, path) {
  if (!Number.isSafeInteger(value?.numerator) ||
      !Number.isSafeInteger(value?.denominator) ||
      value.numerator < 0 ||
      value.denominator <= 0 ||
      BigInt(value.numerator) > MAX_SAFE ||
      BigInt(value.denominator) > MAX_SAFE ||
      value.numerator > value.denominator) {
    fail(path + ": invalid exact probability fraction");
  }
  return { n: BigInt(value.numerator), d: BigInt(value.denominator) };
}

function add(left, right) {
  const common = gcd(left.d, right.d);
  const leftScale = right.d / common;
  const rightScale = left.d / common;
  const n = left.n * leftScale + right.n * rightScale;
  const d = left.d * leftScale;
  const divisor = gcd(n, d);
  const reduced = { n: n / divisor, d: d / divisor };
  if (reduced.n > MAX_SAFE || reduced.d > MAX_SAFE) {
    fail("exact probability sum exceeds the portable integer envelope");
  }
  return reduced;
}

function law(value, path) {
  if (!Array.isArray(value?.outcomes) || value.outcomes.length < 1 || value.outcomes.length > 64) {
    fail(path + ": invalid finite probability law");
  }
  const outcomes = new Map();
  let total = { n: 0n, d: 1n };
  for (const [index, mass] of value.outcomes.entries()) {
    if (typeof mass?.outcome !== "string" || mass.outcome.trim().length === 0 || outcomes.has(mass.outcome)) {
      fail(path + ".outcomes[" + index + "]: outcome names must be nonempty and unique");
    }
    const p = rational(mass.probability, path + ".outcomes[" + index + "].probability");
    outcomes.set(mass.outcome, p);
    total = add(total, p);
  }
  if (total.n !== total.d) fail(path + ": probability masses do not sum exactly to one");
  return outcomes;
}

function equalRational(a, b) {
  return a.n * b.d === b.n * a.d;
}

function equalLaw(left, right) {
  const names = new Set([...left.keys(), ...right.keys()]);
  for (const name of names) {
    const a = left.get(name) ?? { n: 0n, d: 1n };
    const b = right.get(name) ?? { n: 0n, d: 1n };
    if (!equalRational(a, b)) return false;
  }
  return true;
}

function canonical(value) {
  if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
  if (value !== null && typeof value === "object") {
    return "{" + Object.keys(value).sort().map(k => JSON.stringify(k) + ":" + canonical(value[k])).join(",") + "}";
  }
  return JSON.stringify(value);
}

function validateCase(item, index) {
  const path = "cases[" + index + "]";
  if (typeof item.case_id !== "string" || !item.case_id.trim()) fail(path + ": missing case ID");
  if (typeof item.evidence?.scope_id !== "string" || !item.evidence.scope_id.trim()) fail(path + ": missing evidence scope");
  if (typeof item.evidence?.configuration_id !== "string" || !item.evidence.configuration_id.trim()) fail(path + ": missing configuration");
  if (!["admissible", "indeterminate"].includes(item.evidence.admissibility_status)) fail(path + ": invalid evidence admission status");
  if (item.evidence.admissibility_status === "admissible" &&
      !(typeof item.evidence.admissibility_receipt_id === "string" && item.evidence.admissibility_receipt_id.trim())) {
    fail(path + ": admissible evidence requires an upstream receipt identity");
  }
  law(item.evidence.observation_law, path + ".evidence.observation_law");
  if (!Array.isArray(item.worlds) || item.worlds.length < 1 || item.worlds.length > 64) fail(path + ": invalid world count");
  const worldIds = new Set();
  for (const [worldIndex, world] of item.worlds.entries()) {
    if (typeof world.world_id !== "string" || !world.world_id.trim() || worldIds.has(world.world_id)) fail(path + ": duplicate/empty world ID");
    worldIds.add(world.world_id);
    if (typeof world.decision_class !== "string" || !world.decision_class.trim()) fail(path + ": missing decision class");
    if (!world.parameter_signature || Array.isArray(world.parameter_signature) || typeof world.parameter_signature !== "object" ||
        Object.keys(world.parameter_signature).length === 0) fail(path + ": missing parameter signature");
    law(world.observation_law, path + ".worlds[" + worldIndex + "].observation_law");
  }
  if (!Array.isArray(item.probes) || item.probes.length > 64) fail(path + ": invalid probe count");
  const probeIds = new Set();
  for (const [probeIndex, probe] of item.probes.entries()) {
    if (typeof probe.probe_id !== "string" || !probe.probe_id.trim() || probeIds.has(probe.probe_id)) fail(path + ": duplicate/empty probe ID");
    probeIds.add(probe.probe_id);
    if (!["admissible", "inadmissible", "indeterminate"].includes(probe.admissibility_status)) fail(path + ": invalid probe admission status");
    if (probe.admissibility_status === "admissible" &&
        !(typeof probe.admissibility_receipt_id === "string" && probe.admissibility_receipt_id.trim())) {
      fail(path + ": admissible probe requires an upstream receipt identity");
    }
    for (const key of ["reference_trials", "exposure_micro_units", "elapsed_millis"]) {
      if (!Number.isSafeInteger(probe.cost?.[key]) || probe.cost[key] < 0) fail(path + ": invalid probe cost " + key);
    }
    const keys = Object.keys(probe.world_outcome_laws ?? {}).sort();
    if (canonical(keys) !== canonical([...worldIds].sort())) fail(path + ": probe laws must cover every candidate world");
    for (const [worldId, probeLaw] of Object.entries(probe.world_outcome_laws)) {
      law(probeLaw, path + ".probes[" + probeIndex + "].world_outcome_laws." + worldId);
    }
  }
}

function rankProbe(probe, compatibleWorlds) {
  const outcomeDecisions = new Map();
  const outcomeWorldCounts = new Map();
  for (const world of compatibleWorlds) {
    const outcomeLaw = law(probe.world_outcome_laws[world.world_id], "probe " + probe.probe_id + "/" + world.world_id);
    for (const [outcome, probability] of outcomeLaw.entries()) {
      if (probability.n === 0n) continue;
      if (!outcomeDecisions.has(outcome)) outcomeDecisions.set(outcome, new Set());
      outcomeDecisions.get(outcome).add(world.decision_class);
      outcomeWorldCounts.set(outcome, (outcomeWorldCounts.get(outcome) ?? 0) + 1);
    }
  }
  const maxDecisions = Math.max(...[...outcomeDecisions.values()].map(set => set.size));
  const maxWorlds = Math.max(...outcomeWorldCounts.values());
  return {
    rank: 0,
    probe_id: probe.probe_id,
    worst_case_decision_class_count: maxDecisions,
    worst_case_compatible_world_count: maxWorlds,
    cost: {
      reference_trials: probe.cost.reference_trials,
      exposure_micro_units: probe.cost.exposure_micro_units,
      elapsed_millis: probe.cost.elapsed_millis
    }
  };
}

function compareProbe(a, b) {
  return a.worst_case_decision_class_count - b.worst_case_decision_class_count ||
    a.cost.reference_trials - b.cost.reference_trials ||
    a.cost.exposure_micro_units - b.cost.exposure_micro_units ||
    a.cost.elapsed_millis - b.cost.elapsed_millis ||
    a.probe_id.localeCompare(b.probe_id);
}

function emptyProbeSelection(status) {
  return {
    status,
    selected_probe_id: null,
    selected_worst_case_decision_class_count: null,
    admissible_probe_count: 0,
    excluded_probe_ids: [],
    ranked_admissible_probes: []
  };
}

function selectProbe(item, compatibleWorlds) {
  const excluded = [];
  const ranked = [];
  for (const probe of item.probes) {
    if (probe.admissibility_status !== "admissible" ||
        typeof probe.admissibility_receipt_id !== "string" ||
        !probe.admissibility_receipt_id.trim()) {
      excluded.push(probe.probe_id);
      continue;
    }
    ranked.push(rankProbe(probe, compatibleWorlds));
  }
  ranked.sort(compareProbe);
  ranked.forEach((entry, index) => { entry.rank = index + 1; });
  if (ranked.length === 0) {
    return {
      status:"no_admissible_probe",
      selected_probe_id:null,
      selected_worst_case_decision_class_count:null,
      admissible_probe_count:0,
      excluded_probe_ids:excluded,
      ranked_admissible_probes:[]
    };
  }
  const best = ranked[0];
  return {
    status:best.worst_case_decision_class_count === 1
      ? "selected_resolving_probe"
      : "selected_best_available_non_resolving_probe",
    selected_probe_id:best.probe_id,
    selected_worst_case_decision_class_count:best.worst_case_decision_class_count,
    admissible_probe_count:ranked.length,
    excluded_probe_ids:excluded,
    ranked_admissible_probes:ranked
  };
}

function evaluateCase(item) {
  if (item.evidence.admissibility_status !== "admissible" ||
      typeof item.evidence.admissibility_receipt_id !== "string" ||
      !item.evidence.admissibility_receipt_id.trim()) {
    return {
      case_id:item.case_id,
      disposition:"EVIDENCE_INDETERMINATE",
      evidence_scope_id:item.evidence.scope_id,
      compatible_world_ids:[],
      compatible_world_count:0,
      decision_classes:[],
      parameter_identified:null,
      probe_selection:emptyProbeSelection("not_evaluated")
    };
  }

  const evidenceLaw = law(item.evidence.observation_law, "evidence/" + item.case_id);
  const compatible = item.worlds.filter(world => equalLaw(law(world.observation_law, "world/" + world.world_id), evidenceLaw));
  const worldIds = compatible.map(world => world.world_id).sort();
  if (compatible.length === 0) {
    return {
      case_id:item.case_id,
      disposition:"IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE",
      evidence_scope_id:item.evidence.scope_id,
      compatible_world_ids:[],
      compatible_world_count:0,
      decision_classes:[],
      parameter_identified:null,
      probe_selection:emptyProbeSelection("not_evaluated")
    };
  }

  const decisionClasses = [...new Set(compatible.map(world => world.decision_class))].sort();
  const signatures = new Set(compatible.map(world => canonical(world.parameter_signature)));
  const parameterIdentified = signatures.size === 1;
  let disposition;
  let probeSelection;
  if (decisionClasses.length > 1) {
    disposition = "DECISION_NOT_IDENTIFIED";
    probeSelection = selectProbe(item, compatible);
  } else if (!parameterIdentified) {
    disposition = "PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED";
    probeSelection = emptyProbeSelection("not_required_decision_already_identified");
  } else {
    disposition = "DECISION_IDENTIFIED";
    probeSelection = emptyProbeSelection("not_required_decision_already_identified");
  }

  return {
    case_id:item.case_id,
    disposition,
    evidence_scope_id:item.evidence.scope_id,
    compatible_world_ids:worldIds,
    compatible_world_count:worldIds.length,
    decision_classes:decisionClasses,
    parameter_identified:parameterIdentified,
    probe_selection:probeSelection
  };
}

try {
  const [corpusPath] = process.argv.slice(2);
  if (!corpusPath) fail("usage: decision_reference_oracle.mjs <corpus.json>");
  const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
  if (corpus.schema !== "luminous.orbital-decision-identifiability.corpus.v0.1" ||
      corpus.evidence_class !== "synthetic" ||
      corpus.claim_ceiling !== "exact-finite-world-decision-set-algebra-only" ||
      !Array.isArray(corpus.cases) || corpus.cases.length === 0 || corpus.cases.length > 256) {
    fail("unsupported decision-identifiability corpus or claim ceiling");
  }
  const ids = new Set();
  corpus.cases.forEach((item, index) => {
    validateCase(item, index);
    if (ids.has(item.case_id)) fail("duplicate case ID " + item.case_id);
    ids.add(item.case_id);
  });
  const cases = corpus.cases.map(evaluateCase);
  const summary = {
    case_count:cases.length,
    decision_identified_cases:0,
    parameter_unidentified_decision_identified_cases:0,
    decision_not_identified_cases:0,
    model_incompatible_cases:0,
    evidence_indeterminate_cases:0,
    resolving_probe_selected_cases:0,
    no_admissible_probe_cases:0
  };
  for (const item of cases) {
    switch (item.disposition) {
      case "DECISION_IDENTIFIED": summary.decision_identified_cases++; break;
      case "PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED": summary.parameter_unidentified_decision_identified_cases++; break;
      case "DECISION_NOT_IDENTIFIED": summary.decision_not_identified_cases++; break;
      case "IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE": summary.model_incompatible_cases++; break;
      case "EVIDENCE_INDETERMINATE": summary.evidence_indeterminate_cases++; break;
      default: fail("unknown disposition " + item.disposition);
    }
    if (item.probe_selection.status === "selected_resolving_probe") summary.resolving_probe_selected_cases++;
    if (item.probe_selection.status === "no_admissible_probe") summary.no_admissible_probe_cases++;
  }
  const output = {
    result:"pass",
    evidence_class:"synthetic",
    claim_ceiling:"exact-finite-world-decision-set-algebra-only",
    summary,
    cases,
    physical_qualification_granted:false,
    authority_granted:false
  };
  process.stdout.write(JSON.stringify(output,null,2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
