# MFG Route Robustness Benchmark v0.1

**Status:** research protocol draft  
**Owner:** Symtropy, issue [#7019](https://github.com/Luminous-Dynamics/symtropy/issues/7019)  
**Branch:** `research/mfg-route-robust-benchmark-v0-1`  
**Evidence class at this stage:** synthetic protocol only  
**Claim ceiling:** no provider, resource, supplier, process, product, physical route or production outcome is qualified.

## 1. Purpose

The machine-capability compiler in [#6786 / PR #6790](https://github.com/Luminous-Dynamics/symtropy/pull/6790) answers whether a candidate production route is feasible, infeasible, unknown or unsupported and exposes a nominal cost/lead-time frontier when the economics are complete.

That is necessary but not sufficient for a distributed production network. A route that is cheapest today may fail under a supplier disruption, machine outage, stale calibration, unavailable inspector, material-lot change or demand surge. Conversely, the route with the best nominal score may be needlessly rejected when a qualified alternate exists.

This benchmark evaluates **route-choice policies**, not another route-feasibility engine or scheduler:

> Given one frozen demand and the evidence-qualified route alternatives, how well does each policy choose and adapt across matched operating regimes, without selecting an inadmissible route or using information that was unavailable at decision time?

The benchmark is intended to reveal when a policy is better, worse, indistinguishable or not evaluable. It must be possible for the proposed Luminous policy to lose to a simple baseline.

## 2. Architectural boundary

The benchmark is an adapter and independent evaluator over existing owners. It must not create a second source of truth.

| Concern | Owner to reuse | Benchmark treatment |
|---|---|---|
| Product/part intent, revision and rights | Product owner; Symtropy #6735 and source product contract | Pin exact identity and revision; do not infer missing geometry or rights |
| Process/capability semantics | Symthaea #5686 and public contract substrate | Consume typed owner references; no parallel process ontology |
| Route feasibility and transitive machine closure | Symtropy #6786 / #6790 | Consume its disposition and blockers; do not recalculate route feasibility |
| Provider/resource offer | Mycelix #6532/#6541 | Treat as a provider declaration until independently resolved and qualified |
| Common-cause dependency analysis | Mycelix #5762 and Symthaea #5774 | Consume dependency identities/dispositions; a declared source count is not independence |
| Production scheduling | Symthaea #7677 and product-specific scheduler owners | Consume plans/results; do not make this benchmark a second scheduler |
| Scenario-family robustness method | Symtropy #2439 | Reuse matched family/seed discipline and per-family regret reporting |
| Execution reconciliation | Symtropy #5758 and Mycelix execution owners | Preserve intended, recorded and observed states |
| Accepted-unit qualification | Mycelix #3084 | Only an owner-native accepted-unit record counts as accepted output |
| Matched production-value outcome | Symthaea #7608 | Reuse primary outcome, guardrail and uncertainty rules |
| Machine interaction / authority | Symthaea #7706 | Outside scope; recommendations never actuate or reserve resources |

An adapter may weaken a claim to Unknown, Unavailable or NotEvaluable. It must never strengthen the owner's raw disposition. Unresolved references are not evidence.

A resource-offer pointer is not a qualified resource, a capacity window is not a reservation, and a schedule is not executed production. A completed operation is not an accepted unit.

## 3. Definitions

- **Route candidate:** an immutable route identity/revision plus the owner-native facts and evidence references needed by the route compiler.
- **Route policy:** a deterministic or explicitly stochastic rule that selects among admissible candidates using only information available at its decision cut-off.
- **Scenario family:** a declared operating regime, such as correlated supplier disruption or calibration downtime. Families represent model-form variation and must not be pooled as IID seeds.
- **Decision cut-off:** the timestamp/version boundary after which observations are unavailable to that decision.
- **Recourse:** a subsequent recommendation after a new observation or disruption. Recourse may propose a new route but cannot imply reserved capacity.
- **Accepted output:** a unique product/unit identity with the current required inspection and release evidence from its authoritative owner. A job-complete flag does not count.
- **Unknown:** evidence is missing, stale, unresolved, unsupported or insufficient to establish a positive disposition. Unknown is neither feasible nor infeasible.

## 4. Research comparator

Kim and Jung, *On-demand supply chain configuration and planning for shared additive manufacturing*, International Journal of Production Research, published online 2 September 2026, models provider selection, part-to-job grouping, job-to-machine assignment and daily scheduling for heterogeneous shared-AM providers. It compares a mixed-integer linear programming (MILP) formulation with a genetic algorithm (GA) and reports GA solutions within 1% of MILP on medium-scale instances.

- DOI: https://doi.org/10.1080/00207543.2026.2728013
- The paper's reported gap is a result for its own formulation, instances, constraints and compute setup. It is not a Luminous result and is not directly transferable to our different evidence contracts.

Reproduce a bounded subset only when the formulation and assumptions can be matched faithfully. Record the exact objective, constraints, instance generation, solver/version, stopping criteria, hardware and compute budget. Where a faithful reproduction is not possible, report the difference and do not compare headline gaps.

The intended extension is not to replace MILP or GA. It is to compare ordinary allocation/planning policies with policies that preserve evidence-state distinctions, common-cause dependencies, uncertainty and non-anticipative recovery.

## 5. Experimental design

### 5.1 Matched scenario families

Stage 0 uses synthetic data only. Freeze six families, each with at least 20 paired seeds:

1. **Nominal:** stable demand, suppliers, machines, materials, staffing and utilities.
2. **Supplier variability:** changing quoted cost, quote expiry, long-tail lead time and declared capacity.
3. **Common-cause disruption:** two nominal suppliers share an upstream component, port, utility, owner or specialist workforce.
4. **Machine and maintenance disruption:** downtime, tool wear, calibration expiry and contention for repair resources.
5. **Process and inspection constraints:** material-lot changes, process-envelope mismatch, missing inspection capacity and requalification burden.
6. **Demand and service stress:** demand surge, tighter deadlines, geographic separation and limited delivery windows.

All policies receive the same base workload and scenario realizations. Hold out complete families for confirmation. A random seed is not an independent scenario family, and multiple seeds of one model do not establish robustness to model-form changes.

Each benchmark instance should reference, not redefine:

- exact product/part ID, revision and rights state;
- output quantity, delivery deadline and acceptance contract;
- route IDs/revisions and machine/process/material profile identities;
- typed provider/resource offer IDs and revisions where available;
- declared capacity/calendar facts and their decision-time visibility;
- transitive dependency identities and common-cause memberships;
- cost-scope identity, currency, quantity, price date and evidence class;
- calibration, safety, skill, inspection and qualification evidence references;
- scenario family, seed and observation cut-off;
- exact policy, evaluator, source and fixture digests.

Unknown dimensions, tolerances, volumes, prices, capacities, source identities and failure probabilities remain null/unknown. The benchmark must not invent physical facts to make a route appear comparable.

### 5.2 Information and leakage discipline

A policy may use only evidence available at its decision cut-off. The evaluator must retain hidden future scenario state separately from policy-visible observations.

Required negative controls:

- future supplier failure is revealed only when its observation becomes available;
- a policy must not select a future route based on a failure that has not yet occurred or been observed;
- stale or post-cut-off evidence must not refresh a resource or calibration state;
- the evaluator must detect a route decision whose evidence timestamp exceeds its cut-off;
- reordering input observations must not change a deterministic decision unless the contract defines order as semantically relevant.

For stochastic policies, freeze policy seeds separately from environment seeds. Report multiple policy seeds under the same environment realizations.

### 5.3 Baseline policies

All baselines use the same admissibility gate and identical decision-time evidence:

1. Cheapest admissible route subject to a declared deadline.
2. Fastest admissible route subject to a declared cost ceiling.
3. Fixed-priority or nearest-admissible-route heuristic.
4. The #6790 nominal cost/lead-time frontier with no additional re-ranking.
5. Exact exhaustive search on tractable, deterministic single-objective instances.
6. Matched MILP and GA reproductions for the shared-AM formulation where a faithful match is possible.
7. A scenario-aware Luminous candidate that retains non-dominated alternatives and makes explicit, evidence-bounded recourse recommendations.

For fairness, report both (a) route feasibility/admissibility and (b) route-choice quality. A policy must not receive credit for a good nominal objective if it selected a route that should have been blocked.

Do not aggregate cost, time, resilience, locality, safety and quality into one weighted score. Use distinct decision contracts, for example:

- minimize fully scoped cost subject to a deadline and fixed product acceptance requirements;
- minimize request-to-accepted-output time subject to a declared budget;
- minimize a declared resource burden subject to the same quality and delivery constraints.

Any risk measure (worst-case, quantile or CVaR) must be declared before scoring, and its confidence/coverage assumptions must be stated. Report the raw objective vector and the constraints alongside any derived ranking.

### 5.4 Oracle discipline

Use two distinct references:

- **Information-matched oracle:** an exact solution for small instances that has the same information available to the policy at each decision cut-off. This is the primary decision-regret reference for uncertain scenarios.
- **Clairvoyant lower bound:** an optional exact solution that sees the entire future realization. Label it as unattainable information advantage; never use it as the primary pass/fail comparator.

For deterministic, fully specified cases, an exhaustive oracle can provide exact known answers. For larger cases, report a bounded oracle or best available solver result and the resulting gap. Never call a heuristic's own output an oracle.

## 6. Metrics

Report these separately, with full denominators and per-family results.

| Metric | Definition / interpretation |
|---|---|
| False-feasible selections | Number of chosen routes with a hard blocker or owner disposition other than admissible; target 0 |
| Unknown promotion | Number of cases where unknown, unresolved or stale evidence is treated as positive; target 0 |
| Feasible-route coverage | Share of known-admissible opportunities retained; prevents an always-abstain policy from appearing perfect |
| Abstention | Share of decisions withheld because the evidence or objective is insufficient |
| Constrained regret | Relative objective loss versus the information-matched oracle under the same declared constraints |
| Pareto quality | Frontier coverage and dominated-choice rate; report normalization and reference frontier |
| Service level | Accepted outputs delivered by deadline divided by all admitted demand; failures and unserved demand remain in denominator |
| Tail service time | P50/P90 request-to-accepted-output time, including queue, transport, inspection and rework where evidenced |
| Capacity integrity | Overbooking, overlapping commitments and double-counted capacity; target 0 |
| Disruption loss | Unserved admitted demand and accepted-output shortfall by family |
| Recovery | Time from observed disruption to a new admissible recommendation and, separately, to restored accepted service |
| Route churn | Unnecessary switching under small non-semantic perturbations; legitimate switches after new blockers are not penalized |
| Evidence completeness | Fraction of recommendations with complete, current, owner-resolved evidence for the relevant claim |
| Runtime/scale | Wall time, memory, instance size, policy and hardware; no unbounded compute for one baseline |

For a minimization objective with positive oracle value, the constrained relative regret may be reported as:

\[
R = \frac{J_{policy}-J_{oracle}}{\max(|J_{oracle}|,\epsilon)}
\]

The evaluator must define the zero/negative-objective handling in the schema rather than silently applying this expression. Report absolute objective values as well as normalized regret.

A scenario family with no admissible route should produce an explicit no-route/abstain outcome, not a fabricated finite cost. A zero accepted-output denominator makes cost per accepted output undefined, not zero.

## 7. Hard gates and adversarial corpus

Every policy is evaluated behind the independent hard-gate verifier. At minimum, frozen cases must cover:

- revoked/missing rights and wrong product revision;
- incompatible machine, process or material envelope;
- stale calibration, safety evidence, skill or inspection qualification;
- missing/unknown critical dependency, including a child marked noncritical under a critical root;
- offer expired, withdrawn, disputed, superseded or unresolved lifecycle event;
- overlapping capacity windows, double booking and capacity reused across policies;
- provider aliases sharing a single upstream dependency;
- unknown mandatory cost treated as zero;
- quote expired or different currency/price dates without an admitted normalization source;
- future observation leakage;
- quality prediction, simulation output or job completion promoted to physical acceptance;
- failed, censored, late and unserved work dropped from denominators;
- input permutation, duplicate identifiers and altered source digests.

No economic, scheduling or resilience improvement can compensate for a hard blocker.

## 8. Proposed targets

These are project hypotheses, not observed results or industry norms:

- 100% rejection of frozen hard-blocker and unknown-promotion controls.
- 100% deterministic replay for identical manifests, policies, source digests and seeds.
- 100% exact agreement with the independent oracle on small, deterministic, fully specified single-objective instances.
- On larger matched instances, target <=2% median and <=5% P95 constrained objective gap where an exact or defensible bounded oracle exists.
- Six scenario families, >=20 paired seeds per family, and whole-family holdout.
- Zero capacity overbooking and zero hidden hard-gate violations.
- No guardrail regression in product acceptance, safety, rights or traceability.
- All failed, delayed, censored, reworked and unserved tasks retained in denominators.
- A claimed improvement must survive per-family analysis and uncertainty intervals; overlapping no-effect results are inconclusive.

The gap targets apply only when the oracle, objective and feasible set are comparable. A 1% GA-vs-MILP gap in a published study is not evidence that a different Luminous route-policy benchmark has achieved that result.

## 9. Implementation sequence

1. **Protocol and owner audit:** complete this specification and confirm the exact boundaries of #2439, #6786, #5758, #5762, #7677 and #7608.
2. **Frozen fixtures:** create a small, versioned synthetic corpus with independently authored expected outcomes and adversarial mutations.
3. **Independent verifier:** implement in Rust, offline and deterministic. Consume pinned #6790-style route-evaluator outputs; do not clone the route-feasibility logic or scheduler.
4. **Baselines and oracle:** implement transparent deterministic heuristics and an exhaustive small-instance oracle. Add MILP/GA only with a faithful formulation and declared compute budget.
5. **Family evaluator:** preserve family-level uncertainty, matched seeds, holdout boundaries and per-family regret.
6. **Reproducible report:** bind source, fixture, policy and evaluator digests; include raw outcomes, exclusions, failed cases and denominators.
7. **Recorded-data replay:** only after lawful, source-backed product-owner requirements and qualified owner-native resources are available.
8. **Physical pilot:** only after #12533/#4689/#3084 and relevant safety/quality owners authorize the exact cell, part and process. Benchmarking must not control a machine.

## 10. Standards and external references

These references guide protocol design; citing them does not establish conformity.

- ISO/ASTM 52902:2023, *Additive manufacturing — Test artefacts — Geometric capability assessment of additive manufacturing systems*: https://www.iso.org/standard/79683.html
- ISO/ASTM 52924:2023, *Additive manufacturing of polymers — Qualification principles — Classification of part properties* (includes MEX): https://www.iso.org/standard/76909.html
- ISO/ASTM 52966:2026, *Additive manufacturing — Qualification principles — Framework for categorizing resources and process capabilities*: https://www.iso.org/standard/89300.html
- NIST IR 8536, *Supply Chain Traceability: A Manufacturing Meta-Framework*, final September 2026: https://csrc.nist.gov/pubs/ir/8536/final
- Kim and Jung (2026), shared additive-manufacturing supply-chain configuration and planning: https://doi.org/10.1080/00207543.2026.2728013
- NIST AM-Bench: https://www.nist.gov/ambench — use its controlled benchmark discipline, while recognizing its 2025 polymer challenge concerned vat photopolymerization, not a desktop FFF/MEX leaderboard.

## 11. Claim ceiling

A passing synthetic harness validates only the stated software/evaluator behavior on the frozen synthetic corpus. It does not qualify any supplier, provider, offer, site, machine, operator, material, process, part, product, delivery promise or physical production route. It does not establish cost savings, improved service levels, disruption resilience, local self-sufficiency, standards conformity or superiority to commercial systems.

The first physical success criterion is not a benchmark score. It is one released low-risk part, made through a controlled route, independently measured and accepted, with all failed attempts and resource costs retained in the record.


## 12. Literature refinement: limitations of the 2026 shared-AM baseline

Kim and Jung's 2026 shared-AM configuration/planning model is a valuable matched baseline, but its stated assumptions are narrower than a real service network:

- provider operating hours, schedules and machine specifications are deterministic;
- machine capacities are fixed and breakdowns do not occur during the planning horizon;
- transportation time and cost are deterministic and known;
- quality differences across providers/machines are not explicitly modeled as a cross-provider consistency constraint;
- the paper identifies historical execution data (delays, machine reliability) as a future direction rather than an input to its present formulation.

These are not criticisms of the paper's contribution; they define a fair extension boundary for this programme. We should reproduce the published problem on its own assumptions, then introduce one extension at a time: time-varying availability, machine/inspection outages, common-cause supply disruption, material/quality compatibility and execution-history calibration. Report the ablation for each added factor.

The route-policy synthetic corpus is deliberately a stress-test abstraction. Its probability inputs are declared synthetic parameters, not fitted supplier or machine failure probabilities. Its hidden realized outages are evaluator-only challenge outcomes, not observable policy features. In v0.1, policy selection is a single primary-route recommendation; it does not model reservations, fallback recourse execution, or multi-part job grouping. Those are later extensions requiring explicit time-expanded resource and non-anticipativity semantics.

## 13. Executable synthetic tranche

Draft implementation in this PR adds:

- `tools/mfg-route-policy-benchmark/`: standalone offline Rust evaluator using strict Serde deserialization.
- `schemas/manufacturing/mfg_route_policy_case_v0_1.schema.json`: strict Draft 2020-12 case schema.
- `fixtures/manufacturing/mfg_route_policy_cases_v0_1.json`: six synthetic scenario families, 20 paired seeds per family (120 cases total).
- `fixtures/manufacturing/mfg_route_policy_known_answers_v0_1.json`: separately generated cross-language known-answer fixture for each baseline/policy choice.
- `fixtures/manufacturing/mfg_route_policy_manifest_v0_1.json`: corpus identity and claim ceiling.
- `.github/workflows/mfg-route-policy-benchmark.yml`: formatting, exact known-answer tests, output checks, negative controls, Clippy and artifact hashing.

The five policies are cheapest admissible, fastest admissible, reliability-first, expected-cost and fixed priority. The exact small-instance oracle minimizes the declared expected-cost objective using only policy-visible inputs. The clairvoyant lower bound is separately reported from evaluator-only outcomes and is **not** used for policy regret or pass/fail.

The evaluator rejects future evidence exposed at the decision cut-off, invalid probability bounds, contradictory probability declarations for one common-cause group, inconsistent family/seed manifests, malformed owner dispositions and unresolved hidden-outcome references. It makes an explicit abstention when no route passes the declared order constraints. Unknown/infeasible/unsupported owner dispositions are never selected.

### Current verification boundary

The Rust code and fixtures have been committed to the branch and are wired into an exact-branch/pull-request workflow. This environment does not have `cargo` or `rustc`, so no local Rust compile, test or Clippy pass is claimed. The fixture/known-answer generation is deterministic and was authored separately from the Rust evaluator, but the expected outputs have not yet received independent engineering review. A hosted exact-head pass, source/artifact digest review, and independent review of the objective and failure-model assumptions remain required.

### Important model limitations

- Failure probabilities and cost units are synthetic and not empirical or calibrated.
- The simple combined probability formula assumes independent route-specific residual failure and common-cause-domain failure; it does not model correlations between distinct domains.
- The common-cause group identity is supplied by the fixture; no real supplier identity resolution or cut-set proof occurs here.
- The single-route decision omits fallback reservations, multi-part job grouping, transport routing, dynamic capacity contention and time-expanded recourse.
- A synthetic completion is not a conforming part, physical delivery, accepted product or qualified manufacturing service.
