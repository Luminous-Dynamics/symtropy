import assert from "node:assert/strict";
import fs from "node:fs";

const path = process.argv[2];
if (!path) throw new Error("usage: node reference_oracle.mjs <campaigns.json>");
const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
assert.equal(corpus.schema, "luminous.orbital-interface-test-matrix.corpus.v0.1");
assert.equal(corpus.evidence_class, "synthetic");
assert.equal(corpus.claim_ceiling, "synthetic-test-matrix-coverage-semantics-only");

function eligible(run) {
  return ["passed", "failed"].includes(run.outcome)
    && /^[0-9a-f]{64}$/.test(run.raw_artifact_sha256)
    && run.calibration_valid === true
    && run.instrument_in_range === true
    && run.uncertainty_complete === true
    && run.clock_aligned === true
    && typeof run.independence_group === "string"
    && run.independence_group.length > 0
    && Array.isArray(run.evidence_keys)
    && run.evidence_keys.length > 0;
}
function scoped(run, obligation) {
  return run.configuration_id === obligation.configuration_id
    && run.procedure_revision === obligation.procedure_revision
    && run.service_kind === obligation.service_kind
    && run.lifecycle_phase === obligation.lifecycle_phase
    && run.evidence_keys.includes(obligation.evidence_key);
}
function exact(run, obligation) {
  return Object.entries(obligation.conditions).every(([factor, level]) => run.conditions[factor] === level);
}
function baselineOfat(obligation, runs) {
  const cells = Object.entries(obligation.conditions);
  return cells.every(([factor, level]) =>
    runs.some(run => eligible(run) && scoped(run, obligation) && run.conditions[factor] === level));
}
function baselinePairwise(obligation, runs) {
  const cells = Object.entries(obligation.conditions);
  if (cells.length < 3) return baselineOfat(obligation, runs);
  for (let i = 0; i < cells.length; i += 1) {
    for (let j = i + 1; j < cells.length; j += 1) {
      const [f1, v1] = cells[i];
      const [f2, v2] = cells[j];
      const found = runs.some(run => eligible(run) && scoped(run, obligation)
        && run.conditions[f1] === v1 && run.conditions[f2] === v2);
      if (!found) return false;
    }
  }
  return true;
}
function strictGroups(obligation, runs) {
  return new Set(runs.filter(run => eligible(run) && scoped(run, obligation) && exact(run, obligation))
    .map(run => run.independence_group));
}
function inspect(caseInput) {
  if (!caseInput.frozen_before_execution) {
    return {
      disposition: "rejected", covered: 0, missing: caseInput.obligations.filter(o => o.criticality !== "informational").map(o => o.id),
      eligible: 0, failed: 0, aborted: 0, invalid: 0, planned: 0, duplicates: 0,
      budget: false, cycle: false, ofat: 0, pairwise: 0
    };
  }
  const byId = new Map();
  const unique = [];
  let duplicates = 0;
  for (const run of caseInput.runs) {
    if (byId.has(run.run_id)) {
      if (JSON.stringify(byId.get(run.run_id)) === JSON.stringify(run)) {
        duplicates += 1;
        continue;
      }
      return {
        disposition: "disputed", covered: 0,
        missing: caseInput.obligations.filter(o => o.criticality !== "informational").map(o => o.id),
        eligible: 0, failed: 0, aborted: 0, invalid: 0, planned: 0,
        duplicates, budget: false, cycle: false, ofat: 0, pairwise: 0
      };
    }
    byId.set(run.run_id, run);
    unique.push(run);
  }
  const badNa = caseInput.obligations.some(o =>
    (o.not_applicable_reason !== null && (!o.not_applicable_approved || !o.not_applicable_reason))
    || (o.not_applicable_approved && o.not_applicable_reason === null));
  if (badNa) {
    return {
      disposition: "rejected", covered: 0,
      missing: caseInput.obligations.filter(o => o.criticality !== "informational").map(o => o.id),
      eligible: 0, failed: 0, aborted: 0, invalid: 0, planned: 0,
      duplicates, budget: false, cycle: false, ofat: 0, pairwise: 0
    };
  }
  const eligibleRuns = unique.filter(eligible);
  const failed = eligibleRuns.filter(run => run.outcome === "failed").length;
  const aborted = unique.filter(run => run.outcome === "aborted").length;
  const invalid = unique.filter(run => run.outcome === "invalid"
    || (["passed", "failed"].includes(run.outcome) && !eligible(run))).length;
  const planned = unique.filter(run => run.outcome === "planned").length;
  const cost = unique.filter(run => run.outcome !== "planned").reduce((sum, run) => sum + run.cost_units, 0);
  const budget = cost > caseInput.cost_budget;
  const cycle = unique.filter(run => run.outcome === "passed").length >= caseInput.cycle_count_threshold;
  let covered = 0;
  let failedCoverage = false;
  let ofatFalse = 0;
  let pairwiseFalse = 0;
  const missing = [];
  for (const obligation of caseInput.obligations) {
    if (obligation.not_applicable_approved && obligation.not_applicable_reason) {
      covered += 1;
      continue;
    }
    const groups = strictGroups(obligation, unique);
    const strict = groups.size >= obligation.minimum_independent_groups;
    const required = obligation.criticality !== "informational";
    if (strict) {
      covered += 1;
      failedCoverage ||= unique.some(run => run.outcome === "failed" && eligible(run)
        && scoped(run, obligation) && exact(run, obligation));
    } else if (required) {
      missing.push(obligation.id);
    }
    if (required && !strict && baselineOfat(obligation, unique)) ofatFalse += 1;
    if (required && !strict && baselinePairwise(obligation, unique)) pairwiseFalse += 1;
  }
  const disposition = missing.length
    ? (eligibleRuns.length === 0 ? "indeterminate" : "coverage_gap")
    : (failedCoverage ? "coverage_complete_with_failures" : "coverage_complete");
  return {
    disposition, covered, missing, eligible: eligibleRuns.length, failed, aborted, invalid, planned,
    duplicates, budget, cycle, ofat: ofatFalse, pairwise: pairwiseFalse
  };
}

assert.equal(corpus.cases.length, 35);
const seen = new Set();
for (const item of corpus.cases) {
  assert.ok(!seen.has(item.case_id), "duplicate campaign case " + item.case_id);
  seen.add(item.case_id);
  const actual = inspect(item);
  const expected = item.expected;
  assert.deepEqual(actual, {
    disposition: expected.disposition,
    covered: expected.covered_obligations,
    missing: expected.missing_obligation_ids,
    eligible: expected.eligible_runs,
    failed: expected.failed_runs,
    aborted: expected.aborted_runs,
    invalid: expected.invalid_runs,
    planned: expected.planned_runs,
    duplicates: expected.duplicate_runs,
    budget: expected.budget_exceeded,
    cycle: expected.cycle_count_baseline_pass,
    ofat: expected.ofat_false_coverage_count,
    pairwise: expected.pairwise_false_coverage_count
  }, item.case_id);

  const permuted = {
    ...item,
    obligations: [...item.obligations].reverse(),
    runs: [...item.runs].reverse()
  };
  const permutedResult = inspect(permuted);
  assert.deepEqual(
    { ...permutedResult, missing: [...permutedResult.missing].sort() },
    { ...actual, missing: [...actual.missing].sort() },
    item.case_id + " permutation invariance"
  );
}
const cycle = corpus.cases.find(c => c.case_id === "case-02-many-nominal-cycles-no-low-energy");
assert.equal(cycle.expected.cycle_count_baseline_pass, true);
assert.equal(cycle.expected.disposition, "coverage_gap");
const ofatTrap = corpus.cases.find(c => c.case_id === "case-04-ofat-does-not-cover-required-pair");
assert.equal(ofatTrap.expected.ofat_false_coverage_count, 1);
const pairwiseTrap = corpus.cases.find(c => c.case_id === "case-05-pairwise-does-not-cover-critical-triple");
assert.equal(pairwiseTrap.expected.pairwise_false_coverage_count, 1);
process.stdout.write("independent oracle: 35/35 cases and permutation checks passed\n");
