# Maritime Resilience Falsification Protocol v0.1

**Status:** test and evaluation protocol; not a validated operational model or accreditation.  
**Purpose:** make it harder for the evaluator to produce a convincing but unjustified “feasible”, “recovered” or “resilient” result.

## 1. Paradigm: falsification before optimization

A simulator can be fast, reproducible and internally consistent while being wrong about a real port. More positive examples alone do not solve that problem. The next step is to systematically search for inputs that should break the claim.

NASA-STD-7009B makes model-and-simulation credibility, acceptance criteria, and evidence over the model lifecycle explicit. NASA's implementation handbook describes planning, development/verification/validation and use as distinct life-cycle concerns. NIST research by Raunak and Olsen shows that metamorphic testing can help with verification and validation when exact expected outputs are hard to obtain. UNCTAD's port guidance additionally calls for resilience to be grounded in specific hazards, vulnerabilities, response measures and review of implemented measures.

Sources:
- [NASA-STD-7009B — Standard for Models and Simulations](https://standards.nasa.gov/standard/nasa/nasa-std-7009)
- [NASA-HDBK-7009B — implementation guide, February 2026](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009)
- [Raunak & Olsen, Metamorphic Testing on the Continuum of Verification and Validation of Simulation Models, NIST/IEEE-ACM MET 2021](https://www.nist.gov/publications/metamorphic-testing-continuum-verification-and-validation-simulation-models)
- [UNCTAD Port Resilience Guidebook](https://unctad.org/publication/building-capacity-manage-risks-and-enhance-resilience-guidebook-ports)

The intended loop is:

1. Declare the claim and domain of applicability.
2. Write its invariants and plausible falsifiers **before** looking at the run result.
3. Mutate one factor at a time, then test carefully selected interactions.
4. Compare independent computation, contract predictions and known evidence.
5. Preserve every counterexample as a regression fixture.
6. Separate failure to verify code from failure to validate a model against the real system.

A green test is evidence for the exact property tested, not a universal credibility certificate.

## 2. Immediate slice now added around the trajectory reducer

The Rust tests in `adaptive_resilience.rs` exercise:
- independent-oracle parity for the four frozen trajectory examples;
- deterministic report serialization for identical bytes and source binding;
- duplicate JSON keys, unknown fields and authority escalation;
- expected-metric drift and zero-demand rejection;
- undeclared propagation-model rejection and forbidding probability weights;
- exact service-floor recovery boundary;
- an interrupted recovery streak;
- one sample just below the floor, which must reset the recovery streak;
- a trace referencing an undeclared scenario;
- dependency edges referencing undeclared nodes;
- perturbations extending beyond the declared horizon;
- duplicate trace identifiers;
- a fixed-seed, 256-case integer-trajectory corpus compared against a test oracle that does not call production metric helpers;
- unknown/indeterminate evidence cases that cannot acquire a quantitative projection.

This is a **starter corpus**, not full falsification coverage. Rust execution still requires the exact-head workflow; no pass is implied by source inspection.

## 3. Mutate the problem along orthogonal dimensions

Freeze a corpus across the following dimensions before tuning metrics or model behaviour.

| Dimension | Mutations to exercise | Required invariant |
|---|---|---|
| Temporal boundaries | Exact expiry, just before/after expiry, interval starts/ends, gaps, overlapping intervals, future issue time | One pinned and documented inclusive/exclusive time convention; no accidental recovery or applicability at an adjacent boundary |
| Evidence quality | Missing, stale, withdrawn, conflicting, duplicated or wrong-scope evidence | Missing or weaker evidence cannot improve a dependent claim |
| Dependency graph | Missing node, unknown edge endpoint, duplicate identity, cycle, disconnected node, uncertain edge, shared dependency | Invalid references are rejected; undeclared propagation never creates numeric downstream effects |
| Disturbance shape | Single shock, duration extension, repeated shock, staggered recovery, simultaneous common cause | The declared case schedule is represented faithfully; additional adverse required cases cannot improve the robust feasible set |
| Service trajectories | One interval below floor, exact-floor sample, interrupted recovery, missing interval, zero demand, unusually high rate, overflow boundary | Exact service metric semantics; missing observations never masquerade as zero or normal service |
| Policy semantics | Evidence requested vs received, alternate capacity unconfirmed, unsupported action, external side effect sentinel | Only bounded simulation-only outputs; no implied reservation, dispatch or authority |
| Reproducibility | Same bytes/configuration; reordered JSON keys; changed source byte; changed engine/policy version | Same pinned input gives same canonical semantic output; changed bound bytes or versions create a new run binding |
| Model scope | Unsupported current, draft, energy, berth, route or vessel calculation | `UNSUPPORTED` or `INDETERMINATE`, never a plausible invented numeric result |
| Claim boundaries | True authority flag, altered claim ceiling, stale model-admission record | Run fails closed regardless of otherwise-correct metrics |
| Distribution shift | Unseen port type, hazard severity, topology, operational regime or demand profile | Report out-of-domain; do not transfer a validation claim without evidence |

Do not enumerate the Cartesian product of every variable blindly. Start with single-factor tests and pairwise interactions, then prioritize 3-way-and-higher cases where shared dependencies, temporal overlap, uncertain evidence or policy branching could produce a high-consequence false green. The scenario generator itself needs an independent oracle and a replayable seed.

## 4. Metamorphic relations

Metamorphic tests assert relations between results when exact outputs are unavailable. They must state their preconditions; “more evidence should always make the outcome better” is **not** a valid universal rule.

Required relations include:

1. **Hard-constraint non-compensation:** reducing delay or resource use cannot turn a violated hard constraint into a feasible claim.
2. **Evidence degradation:** replacing valid required evidence with missing/stale/conflicting evidence cannot improve the affected disposition; unrelated outcomes may remain unchanged.
3. **Case-set expansion:** adding a required adverse case can shrink or preserve the robustly-feasible candidate set, never expand it.
4. **Duration extension:** extending an otherwise identical below-service-floor interval cannot reduce integrated unmet service if demand and units stay fixed.
5. **Recovery interruption:** one below-floor interval within a recovery streak resets the streak.
6. **Time translation:** shifting every event and interval by the same amount preserves duration-based metrics only when validity windows, policy clocks and time-dependent inputs shift consistently too.
7. **Identity/ordering invariance:** reordering independent JSON object keys or non-semantic presentation order does not change semantic metric results; digest and receipt identity remain tied to exact input bytes.
8. **Duplicate evidence:** duplicating a correlated record does not increase independent corroboration.
9. **Unsupported propagation:** marking an edge model as undeclared cannot introduce a numeric propagated loss.
10. **Observability:** dropping a required observed interval cannot produce a recovered-within-horizon claim from the remaining shorter sequence.
11. **Authority invariance:** metric improvements cannot alter `NotAdmitted` or any authority flag.
12. **Schema/version drift:** changing a bound model, policy, or schema revision invalidates the old run identity; it cannot overwrite the old receipt.

Whenever an invariant does not hold under a legitimate condition, encode the boundary as a separate case instead of weakening the broad test to permit both outcomes silently.

## 5. Benchmark targets: code versus model

### A. Contract and safety-critical invariants

Proposed release gates for the pure reducer:
- 100% of frozen authority-escalation controls rejected.
- 100% of hard-constraint false-green controls rejected.
- 100% of the frozen unknown/missing/stale/conflict controls preserve uncertainty.
- 100% parity with the independent oracle over frozen integer metric cases.
- Zero off-by-one mismatches in the temporal and recovery boundary corpus.
- At least 95% mutation score overall, with 100% for critical authority, invalid-time, unknown-as-success, and unsupported-propagation mutations. Mutation score means killed non-equivalent mutations / total non-equivalent mutations; document exclusions and surviving mutants.

These targets apply to declared test controls, not the probability of failure in reality.

### B. Scenario coverage and robustness

Report a coverage matrix instead of only raw scenario count:
- 100% of each required single-factor dimension represented;
- at least 90% of the predeclared pairwise interaction set for lower-risk dimensions;
- 100% of the hand-reviewed high-consequence multi-factor interactions;
- separate counts for valid, invalid, unknown, stale, conflicting, unsupported, boundary and authority-escalation cases;
- every discovered counterexample assigned a stable ID, minimized where possible, pinned to a corpus digest and retained as a regression test.

Do not claim “all possible disruptions covered”. Describe the risk-based universe and uncovered combinations.

### C. Predictive validity

After obtaining a qualified reference corpus, split by event and time before tuning. The same incident, revised port report or downstream consequence must not leak into both training/calibration and test sets. Include:
- temporal holdout and leave-one-port-out evaluation when the corpus can support it;
- a persistence baseline and transparent capacity-loss baseline;
- per-service trajectory error, integrated loss error, onset/duration error, and time-to-recovery error;
- right-censored cases for which recovery is not observed;
- uncertainty interval calibration only if the model genuinely makes probabilistic claims.

Use the proposed goals in [MARITIME_RESILIENCE_BENCHMARKS_V0_1.md](./MARITIME_RESILIENCE_BENCHMARKS_V0_1.md) as hypotheses to test, not acceptance criteria for a real port until domain stakeholders approve a use-specific tolerance. An initial ten-event pilot is useful for discovering data problems, but it is insufficient by itself to substantiate broad reliability claims.

## 6. Failure taxonomy and report format

Every failed case should report:
- claim under test and stable property ID;
- exact scenario/input/corpus/configuration digest;
- changed factor(s) and a minimal counterexample where practical;
- expected relation, observed relation and independent-oracle result;
- failure class: contract, arithmetic, temporal, evidence, model-scope, uncertainty, operational-boundary or reproducibility;
- whether any conclusion was incorrectly improved (a false green);
- affected code/model version and regression test added;
- the scope of conclusions that must now be withdrawn.

Keep hard-constraint false greens as their own headline metric. Never bury one in an overall average or treat a high aggregate score as compensation.

## 7. Immediate next implementation after CI

1. Run the exact-head workflow for PR #1665 and fix—not suppress—any failures.
2. Add deterministic seeded generation for bounded integer-rate traces and valid/invalid interval schedules; compare every generated trace to the independent oracle.
3. Add the temporal/evidence and unknown-preserving metamorphic suite, with a machine-readable mutation inventory.
4. Measure the reducer at 1k/10k/100k/1M intervals on a pinned host, reporting p50/p95, input size, RSS and output digest. Do not use queued CI jobs as performance measurements.
5. Inspect Symtropy's actual deterministic engine capability profile, then connect only the supported trace fields. If it cannot produce a needed field, retain `UNSUPPORTED`.
6. Build a historically grounded test corpus with a separate holdout before discussing predictive performance or a real-world pilot.
7. Require a domain owner and independent reviewer to approve the intended use, uncertainty tolerance and validation evidence.

**Completion rule:** a test result is a “pass” only when its command, exact commit, corpus digest, verifier version and executed result are recorded. Static source inspection is not a Rust test pass; a contract test is not physical validation; a physical validation result for one event is not proof of general resilience.
