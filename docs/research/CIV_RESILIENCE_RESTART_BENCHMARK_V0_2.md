# CIV-RES-IND-001 — Essential-Service Recovery + Small-Industry Restart Benchmark v0.2

**Status:** benchmark protocol proposal; executable evidence remains synthetic-only  
**Owners:** Symtropy scenario execution/replay; Symthaea candidate policies; Mycelix provenance/coordination; Sol-Atlas spatial constraints  
**Parent:** [Issue #1708](https://github.com/Luminous-Dynamics/symtropy/issues/1708)  
**Paired-prefix harness:** [CIV-RES-IND-003 / issue #2401](https://github.com/Luminous-Dynamics/symtropy/issues/2401)  
**Date:** 2026-10-21

## 1. Research question

Given the same declared initial state, observation policy, disturbances, resource ledger, authority limits, service floors and quality constraints, does an AI-assisted planning policy restore essential services and restart conforming useful production more effectively than well-specified conventional procedures and transparent heuristics?

Every candidate must be allowed to lose. A simulator result is not a plant result. A checksum does not authenticate a publisher or physical event. A policy action is not permission to actuate equipment.

## 2. Why this is an industry-relevant gap

Industrial standards and products already supply important parts of the digital thread: ISA-95 / IEC 62264 structures enterprise-to-control integration; OPC UA provides industrial information models and communication; GS1 EPCIS supports interoperable event-based visibility; QIF, STEP and MTConnect cover parts of product definition, quality information and machine data. We should interoperate with these systems rather than try to replace them.

NIST's 2026 smart-manufacturing AI/ML roadmap identifies integration of heterogeneous sensing/control systems, industrial data management, reliability, explainability and trustworthy operation as active challenges. NIST's 2026 digital-twin workshop report likewise emphasizes interoperability, verification/validation/uncertainty quantification (VVUQ), cybersecurity and workforce readiness. Our opportunity is not “AI everywhere”; it is an independently inspectable chain from source data and uncertainty to constraints, candidate actions, replayable outcomes and measured benefit.

References:
- NIST, [2026 Roadmap on Artificial Intelligence and Machine Learning for Smart Manufacturing](https://www.nist.gov/publications/2026-roadmap-artificial-intelligence-and-machine-learning-smart-manufacturing), 2026-07-03.
- NIST, [Digital Twins Workshops Summary Report (NISTIR 8620)](https://www.nist.gov/publications/digital-twins-workshops-summary-report), 2026-07-21.
- NIST, [Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing).
- [ISA-95 / IEC 62264](https://www.isa.org/standards-and-publications/isa-standards/isa-95-standard), [OPC UA information models](https://profiles.opcfoundation.org/document/1), and [GS1 EPCIS](https://www.gs1.org/standards/epcis).

## 3. Four composable owners

| Responsibility | Owner | Boundary |
|---|---|---|
| Policy proposals, assumptions, epistemic uncertainty and explanations | Symthaea | A proposal is not truth, authorization or a physical effect. |
| Shared resource/event references, provenance, commitments and corrections | Mycelix | Existing Economic Reality Graph remains the resource/event/claim substrate; no second graph. |
| Spatial/facility/network constraints and location-specific coverage | Sol-Atlas | Spatial projections remain source-qualified and uncertainty-aware. |
| Deterministic scenario execution, action-trace receipts and replay | Symtropy | Synthetic model validity must not be conflated with real-plant validity. |

Existing service-floor, capability-closure, production-quality and authority owners remain authoritative. This benchmark must not implement a competing closure engine or scheduler.

## 4. Experimental protocol

Freeze and hash the fixture, scenario generator version, candidate policy version, baseline parameters, holdout membership, evaluator source, dependency lock/toolchain and serialization rules before scoring.

For each time step:

1. **Observe:** expose only measurements whose availability timestamps are no later than the decision time. Declare sensor lag, detection delay, conflict handling and freshness policy.
2. **Decide:** each policy receives only a typed observation-history envelope, its own version/configuration and allowed static context. Scenario IDs, future event schedules, future arrivals and raw simulator state are prohibited inputs.
3. **Validate:** send every proposal through the same feasibility, service-floor, quality, safety/authority and resource-conservation checks.
4. **Commit:** reserve finite material, energy, worker hours and repair stock in one ledger. A lot, reserve or person's time cannot be spent twice.
5. **Advance:** apply known events and model effects only at their specified times.
6. **Verify:** repair completion, calibration and product conformity become usable only after their declared verification boundary.
7. **Record:** emit normalized result vectors and receipts bound to exact inputs and policy/evaluator versions.

When a source owner specifies different semantics, this protocol must adapt through a versioned adapter; it must not silently override the owner.

## 5. Policy arms and fairness rules

- **P0 — documented conventional procedure:** deterministic representation of the current documented operating procedure. Record every tie-break and manual escalation.
- **P1 — transparent heuristic:** small rule-based policy using declared criticality, current visible service state, known blockers and resource constraints. It must fail closed on missing evidence.
- **P2 — Symthaea-assisted proposals:** candidate plans with declared assumptions, expected resource/capability deltas and uncertainty. Use the same validator as P0 and P1.
- **ORACLE — tiny-fixture diagnostic only:** exhaustive enumeration for a small set of hand-checkable scenarios. It is a privileged-information upper bound, not a deployable competitor and not part of live-policy rankings.

All live-policy arms get identical observation streams, action budgets, resource budgets, deadlines, stop rules and safety/quality constraints. Do not intentionally weaken P0/P1 or give P2 future data. Freeze parameters before holdout scoring; record model selection and all retries.

The criticality heuristic currently implemented in the Rust oracle is only an executable reference-policy input-boundary example. It is **not** a validated manufacturing heuristic, safety controller or evidence of superiority.

## 6. Scenario design

Start with six existing essential-service functions (water, sanitation, critical electricity, food preservation, communications and configured care) and the small-workshop capability graph. Extend the campaign only after single-disturbance semantics are independently reproducible.

The initial comparison target is at least **1,000 stratified synthetic scenarios**, with distinct development, validation and frozen holdout sets. Scenario counts are not probability estimates. Include compound events only after their component semantics pass independent checks.

Strata should include nominal/infeasible cases; finite reserves and duplicate-spend controls; common-mode failures across nominally redundant assets; delayed/censored/conflicting observations; late imports and unavailable repair stock; wrong material grade, expired calibration and incomplete inspection; worker-skill and maintenance bottlenecks; unequal demand by location/population group; cold restart and post-repair verification delay; missing data; and out-of-distribution parameters. Tiny instances should also be independently enumerable by a simpler evaluator.

## 7. Metrics — never hide a hard failure in one score

Report each metric separately, with units, denominator, uncertainty interval and scenario-level outcomes.

**Service and equity:** time to restore each service; time and magnitude below each service floor; unmet demand by location/group; first floor breach; per-group coverage. Total delivered volume does not replace service floors.

**Production and quality:** step of first conforming output; conforming units; first-pass yield; scrap/rework/rejects; blocked production steps; time to verified productive capacity. A running machine or produced geometry is not proof of conforming output.

**Resource and economics:** energy per conforming unit; material and repair-stock consumption; skilled labor hours; imports and delivery delays; maintenance burden; waste and cost ranges when source data justify them. Preserve raw units and assumptions.

**Policy quality:** invalid/blocked proposals; unresolved unknowns; human review/escalation count; action latency; intervention count; resource-feasibility violations; policy regret relative to the tiny-fixture oracle; P0/P1/P2 paired differences.

**Integrity/replay:** frozen-case agreement; input-order invariance; exact-head manifest verification; determinism under a fixed seed/toolchain; trace-prefix consistency; mutation-control coverage; digest and receipt reproducibility.

OEE (availability × performance × quality) can be reported where a plant has a validated OEE definition and source data, but it cannot substitute for energy per conforming unit, service-floor violations, equity, material conservation, safety or evidence quality. A single 85% OEE headline is not a universal target for every process or industry.

## 8. Prespecified acceptance ladder

### H0 — contract integrity
- All frozen schemas and expected disposition vectors validate.
- Duplicate IDs, unknown critical fields, stale digests, incomplete references and invalid state transitions fail closed.
- 100% frozen-fixture agreement is required before comparative policy scoring starts.

### H1 — paired-prefix non-anticipation contract
- For a deterministic policy version, equal canonical visible-history inputs through time t must yield equal recomputed actions through t.
- Every tested hidden/future/scenario-ID input mutation is rejected.
- Input/action trace receipts are stable under pinned serialization, source, manifest and toolchain.
- This does not prove there are no side channels in a deployed process; that requires a runtime-isolated policy harness and adversarial inspection.

### H2 — fair synthetic comparison
- At least 1,000 stratified scenarios, a frozen holdout, independent tiny-instance results and paired P0/P1/P2 comparisons.
- Publish per-metric distributions, paired deltas and confidence intervals, not merely an average score.
- No candidate is eligible for a “better” result if it violates a hard service, safety, quality, authority or conservation gate.
- Every missed hard-gate case remains visible even if mean throughput improves.

### H3 — prospective physical pilot
A target such as **at least 5 percentage points higher conforming yield** or **at least 10% lower energy per conforming unit** may be preregistered as a hypothesis, not claimed as an achievement. Compare against an agreed conventional baseline across representative shifts/lots, with independent measurements and confidence intervals. Require no safety/quality regression; otherwise reject the improvement claim. Site suitability, data access, calibration and workforce effects need their own review.

### H4 — independent replication
An external implementer should be able to consume the public fixtures, run a replaceable adapter and reproduce semantic outcomes without importing Symthaea or Mycelix runtime internals. Report semantic loss in an adapter explicitly.

## 9. Current implementation state

- The v0.1 Rust oracle verifies six synthetic scenario dispositions and binds raw fixture/evaluator/lock bytes to a manifest.
- CIV-RES-IND-003 adds paired futures with equal visible prefixes, a transparent reference heuristic that accepts only bounded observation history, recomputed actions, and per-step serialized input/action digests.
- Negative controls reject hidden future fields, mismatched policy versions, action/observation inconsistencies and favorable defaults when current service-floor evidence is missing.
- This is not yet the actual Symthaea policy runner, an independent industrial baseline study, a physical digital-twin validation, a plant safety case, or a production-performance result.

## 10. Reporting discipline

Every result should include immutable subject commit, fixture and evaluator digests, dependency lock/toolchain identity, policy/configuration versions, random seeds, exact allowed input fields, scenario stratum and holdout membership, normalized results plus all hard-gate failures, unresolved assumptions/missing data/uncertainty, and exact claim ceiling.

No aggregate “civilization resilience”, “industrial intelligence” or “efficiency” score may override a critical failure. A failed result is useful evidence and must remain reproducible.
