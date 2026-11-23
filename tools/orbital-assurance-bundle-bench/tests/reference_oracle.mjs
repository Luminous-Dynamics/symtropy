#!/usr/bin/env node
import fs from "node:fs";

const allowedTypes = new Set([
  "orbital_review_bundle", "lifecycle_qualification", "test_matrix_adequacy",
  "event_history_integrity", "mission_success_probability", "constant_failure_intensity", "power_law_recurrent_intensity",
  "weibull_first_failure_life", "physical_test_observation", "measurement_calibration", "model_credibility",
  "evidence_custody", "external_authority_decision", "defeater",
]);
const priorities = [
  ["GRAPH_INVALID", "invalid_graph"],
  ["CLAIM_TYPE_MISMATCH", "blocked_by_claim_type_mismatch"],
  ["SCOPE_MISMATCH", "blocked_by_scope_mismatch"],
  ["MISSING_AUTHORITY", "blocked_by_missing_authority"],
  ["CONTRADICTED", "contradicted"],
  ["UNSUPPORTED_OR_STALE", "blocked_by_unsupported_or_stale_evidence"],
  ["INDETERMINATE", "indeterminate"],
  ["OPEN_DEFEATER", "blocked_by_open_defeater"],
  ["INCOMPLETE", "incomplete_argument"],
];

function equal(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}
function matchesHex(value, count) {
  return typeof value === "string" && new RegExp("^[0-9a-f]{" + count + "}$").test(value);
}
function evaluate(scenario) {
  const blockers = [];
  const add = (code, claimId, detail) => blockers.push({ code, claim_id: claimId ?? null, detail });

  if (!scenario.case_id || !scenario.root_claim_id) add("GRAPH_INVALID", null, "case and root IDs must be non-empty");
  const scope = scenario.scope;
  if (!scope.subject_id || !scope.configuration_id || !scope.operating_mode_id ||
      !scope.mission_profile_id || !scope.environment_profile_id ||
      scope.valid_from > scope.valid_until || scenario.effective_cutoff < scope.valid_from ||
      scenario.effective_cutoff > scope.valid_until) {
    add("SCOPE_MISMATCH", null, "invalid or out-of-window case scope");
  }
  if (scenario.knowledge_cutoff < scenario.effective_cutoff) {
    add("INDETERMINATE", null, "knowledge cutoff precedes effective cutoff");
  }

  const claims = new Map();
  const ids = new Set();
  for (const claim of scenario.claims) {
    if (!claim.claim_id || ids.has(claim.claim_id)) {
      add("GRAPH_INVALID", claim.claim_id, "duplicate or empty claim ID");
      continue;
    }
    ids.add(claim.claim_id);
    claims.set(claim.claim_id, claim);
    if (!allowedTypes.has(claim.claim_type)) add("CLAIM_TYPE_MISMATCH", claim.claim_id, "unknown claim type");
  }
  const root = claims.get(scenario.root_claim_id);
  if (!root) {
    add("INCOMPLETE", null, "root claim is missing");
    return { blockers, disposition: "incomplete_argument" };
  }
  if (root.claim_type !== "orbital_review_bundle" || root.evidence !== null) {
    add("CLAIM_TYPE_MISMATCH", root.claim_id, "root must be an unevidenced orbital_review_bundle claim");
  }

  const pins = new Map();
  for (const pin of scenario.receipt_pins) {
    if (!pin.receipt_id || !pin.source_repo || !pin.source_path || !pin.receipt_schema ||
        !matchesHex(pin.source_commit, 40) || !matchesHex(pin.receipt_sha256, 64) ||
        pins.has(pin.receipt_id)) {
      add("GRAPH_INVALID", null, "duplicate or malformed receipt pin");
    } else pins.set(pin.receipt_id, pin);
  }

  const counts = new Map();
  let missingAuthority = false;
  for (const claim of scenario.claims) {
    counts.set(claim.claim_type, (counts.get(claim.claim_type) || 0) + 1);
    if (claim.claim_id === scenario.root_claim_id) continue;

    if (!equal(claim.scope, scenario.scope)) {
      add("SCOPE_MISMATCH", claim.claim_id, "claim scope differs from the case's exact subject/configuration/profile/time scope");
    }

    let typed = true;
    if (claim.claim_type === "mission_success_probability") {
      typed = claim.estimand === "mission_success_probability" &&
        claim.unit === "probability" && claim.model_profile === "bernoulli_single_mission";
    }
    if (claim.claim_type === "constant_failure_intensity") {
      typed = claim.estimand === "constant_failure_intensity" &&
        claim.unit === "events_per_million_operating_hours" &&
        claim.model_profile === "homogeneous_poisson_constant_intensity";
    }
    if (claim.claim_type === "power_law_recurrent_intensity") {
      typed = claim.estimand === "power_law_recurrent_failure_intensity" &&
        claim.unit === "events_per_operating_second" &&
        claim.model_profile === "power_law_nhpp_minimal_repair_v0_1";
    }
    if (claim.claim_type === "weibull_first_failure_life") {
      typed = claim.estimand === "first_failure_life_distribution" &&
        claim.unit === "operating_seconds" &&
        claim.model_profile === "weibull_first_failure_life_v0_1";
    }
    if (!typed) {
      add("CLAIM_TYPE_MISMATCH", claim.claim_id, "estimand, unit and model profile do not match the claim type");
    }
    if (claim.claim_type === "model_credibility" &&
        (!claim.validation_domain || !claim.validation_domain.trim())) {
      add("INCOMPLETE", claim.claim_id, "model credibility claim lacks a declared validation domain");
    }

    if (claim.state !== "supported") {
      if (claim.state === "contradicted") add("CONTRADICTED", claim.claim_id, "claim is contradicted");
      else if (claim.state === "indeterminate") add("INDETERMINATE", claim.claim_id, "claim state is indeterminate");
      else if (claim.state === "open_defeater") add("OPEN_DEFEATER", claim.claim_id, "mandatory defeater remains open");
      else add("UNSUPPORTED_OR_STALE", claim.claim_id, "claim is stale, withdrawn or unsupported");
    }
    if (claim.claim_type === "external_authority_decision" && claim.mandatory) {
      missingAuthority ||= claim.state !== "supported";
    }

    const evidence = claim.evidence;
    if (evidence === null) {
      add("INCOMPLETE", claim.claim_id, "claim has no exact receipt reference");
      continue;
    }
    let classMatches = true;
    if (claim.claim_type === "physical_test_observation" || claim.claim_type === "measurement_calibration") {
      classMatches = evidence.evidence_class === "physical" || evidence.evidence_class === "bench";
    }
    if (claim.claim_type === "evidence_custody") {
      classMatches = evidence.evidence_class === "software" || evidence.evidence_class === "authority";
    }
    if (claim.claim_type === "external_authority_decision") {
      classMatches = evidence.evidence_class === "authority";
    }
    if (!classMatches) {
      add("CLAIM_TYPE_MISMATCH", claim.claim_id, "evidence class is incompatible with this claim type");
    }

    if (!evidence.receipt_id || !evidence.source_repo || !evidence.source_path ||
        !evidence.receipt_schema || !matchesHex(evidence.source_commit, 40) ||
        !matchesHex(evidence.receipt_sha256, 64)) {
      add("INCOMPLETE", claim.claim_id, "receipt reference is malformed");
    } else if (evidence.effective_from > scenario.effective_cutoff ||
        evidence.effective_until < scenario.effective_cutoff ||
        evidence.recorded_at > scenario.knowledge_cutoff) {
      add("UNSUPPORTED_OR_STALE", claim.claim_id, "receipt is not current at the declared effective/knowledge cutoffs");
    } else if (pins.has(evidence.receipt_id)) {
      if (!equal(evidence, {
        ...evidence,
        receipt_id: pins.get(evidence.receipt_id).receipt_id,
        source_repo: pins.get(evidence.receipt_id).source_repo,
        source_path: pins.get(evidence.receipt_id).source_path,
        source_commit: pins.get(evidence.receipt_id).source_commit,
        receipt_schema: pins.get(evidence.receipt_id).receipt_schema,
        receipt_sha256: pins.get(evidence.receipt_id).receipt_sha256,
      })) {
        add("UNSUPPORTED_OR_STALE", claim.claim_id, "claim receipt does not match the exact case-level pin");
      }
    } else {
      add("INCOMPLETE", claim.claim_id, "receipt is not included in the case-level pin set");
    }
  }

  const seenRequired = new Set();
  for (const type of scenario.required_claim_types) {
    if (seenRequired.has(type)) add("GRAPH_INVALID", null, "required claim type is listed more than once");
    seenRequired.add(type);
    if (type === "orbital_review_bundle") {
      add("CLAIM_TYPE_MISMATCH", scenario.root_claim_id, "root claim type cannot be declared as a required evidence dependency");
    } else if (!counts.has(type)) {
      add(type === "external_authority_decision" ? "MISSING_AUTHORITY" : "INCOMPLETE",
        null, "required claim type " + type + " is missing");
      missingAuthority ||= type === "external_authority_decision";
    } else if (counts.get(type) > 1) {
      add("GRAPH_INVALID", null, "required claim type " + type + " appears more than once");
    }
  }

  const edgeSet = new Set();
  for (const edge of scenario.edges) {
    const key = edge.from_claim_id + "|" + edge.to_claim_id + "|" + edge.relation;
    if (!claims.has(edge.from_claim_id) || !claims.has(edge.to_claim_id) ||
        edge.from_claim_id === edge.to_claim_id || edgeSet.has(key)) {
      add("GRAPH_INVALID", null, "edge references missing claim, self-loop or duplicate edge");
      continue;
    }
    edgeSet.add(key);
    const from = claims.get(edge.from_claim_id);
    const to = claims.get(edge.to_claim_id);
    if (edge.relation === "contradicts") {
      add("CONTRADICTED", to.claim_id, "claim is contradicted by " + from.claim_id);
    }
    if (edge.relation === "supports") {
      if (from.claim_type === "orbital_review_bundle") {
        add("GRAPH_INVALID", from.claim_id, "support edges point from evidence claim to the claim it supports");
      } else if (to.claim_type !== "orbital_review_bundle") {
        add("GRAPH_INVALID", from.claim_id, "v0.1 permits direct support edges only into the review root");
      }
    }
    const statisticalProfiles = new Set([
      "mission_success_probability",
      "constant_failure_intensity",
      "power_law_recurrent_intensity",
      "weibull_first_failure_life",
    ]);
    if (edge.relation === "derived_from" &&
        from.claim_type !== to.claim_type &&
        statisticalProfiles.has(from.claim_type) &&
        statisticalProfiles.has(to.claim_type)) {
      add("CLAIM_TYPE_MISMATCH", from.claim_id,
        "Bernoulli, constant-intensity, power-law recurrent-event and first-failure Weibull estimands are non-interchangeable");
    }
  }

  for (const claim of scenario.claims) {
    if (claim.claim_id !== scenario.root_claim_id &&
        !scenario.edges.some(edge => edge.from_claim_id === claim.claim_id &&
          edge.to_claim_id === scenario.root_claim_id && edge.relation === "supports")) {
      add("INCOMPLETE", claim.claim_id, "claim is not connected to the review root");
    }
  }
  for (const type of scenario.required_claim_types) {
    for (const claim of scenario.claims.filter(item => item.claim_type === type)) {
      if (!claim.mandatory) add("INCOMPLETE", claim.claim_id, "required claim is not marked mandatory");
    }
  }

  const adjacency = new Map();
  for (const edge of scenario.edges) {
    if (edge.relation === "contradicts") continue;
    if (!adjacency.has(edge.from_claim_id)) adjacency.set(edge.from_claim_id, []);
    adjacency.get(edge.from_claim_id).push(edge.to_claim_id);
  }
  const marks = new Map();
  let cycle = false;
  function visit(node) {
    if (marks.get(node) === 1) { cycle = true; return; }
    if (marks.get(node) === 2) return;
    marks.set(node, 1);
    for (const child of adjacency.get(node) || []) visit(child);
    marks.set(node, 2);
  }
  for (const id of ids) visit(id);
  if (cycle) add("GRAPH_INVALID", null, "claim dependency graph contains a cycle");

  if (scenario.claims.some(claim => claim.claim_type === "external_authority_decision" &&
      claim.mandatory && claim.state !== "supported") || missingAuthority) {
    if (!blockers.some(blocker => blocker.code === "MISSING_AUTHORITY")) {
      add("MISSING_AUTHORITY", null, "required external authority decision is absent or not current");
    }
  }

  const unique = new Map();
  for (const blocker of blockers) {
    unique.set(blocker.code + "|" + blocker.claim_id + "|" + blocker.detail, blocker);
  }
  const normalized = [...unique.values()].sort((a, b) =>
    a.code.localeCompare(b.code) ||
    (a.claim_id || "").localeCompare(b.claim_id || "") ||
    a.detail.localeCompare(b.detail));
  let disposition = "review_bundle_complete";
  for (const [code, name] of priorities) {
    if (normalized.some(blocker => blocker.code === code)) { disposition = name; break; }
  }
  return { blockers: normalized, disposition };
}

const fixturePath = process.argv[2];
if (!fixturePath) {
  process.stderr.write("usage: node reference_oracle.mjs <assurance-cases.json>\n");
  process.exit(2);
}
const corpus = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (corpus.schema !== "luminous.orbital-assurance-bundle.corpus.v0.1" ||
    corpus.evidence_class !== "synthetic" ||
    corpus.claim_ceiling !== "synthetic-assurance-graph-structure-only") {
  throw new Error("unsupported corpus contract");
}
if (corpus.cases.length !== 60) throw new Error("expected 60 frozen cases");
const ids = new Set();
let mismatches = 0;
let complete = 0;
for (const scenario of corpus.cases) {
  if (ids.has(scenario.case_id)) throw new Error("duplicate case ID: " + scenario.case_id);
  ids.add(scenario.case_id);
  const result = evaluate(scenario);
  if (result.disposition !== scenario.expected.disposition ||
      result.blockers.length !== scenario.expected.blocker_count ||
      scenario.expected.authority_granted !== false) {
    mismatches += 1;
    process.stderr.write(
      scenario.case_id + ": expected " + scenario.expected.disposition +
      "/" + scenario.expected.blocker_count + ", got " +
      result.disposition + "/" + result.blockers.length + "\n",
    );
  }
  if (result.disposition === "review_bundle_complete") complete += 1;
}
if (complete < 1) throw new Error("reject-all behavior is not accepted: no positive control");
if (mismatches !== 0) {
  process.stderr.write("FAIL: " + mismatches + " expected-result mismatches\n");
  process.exit(1);
}
process.stdout.write("PASS: 60 cases; independent graph oracle; " + complete + " positive controls; zero mismatches; zero authority grants\n");
