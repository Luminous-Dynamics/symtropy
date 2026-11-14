---
title: Peace Infrastructure and Resilience Benchmark
version: 0.1
status: research-proposal
scope: synthetic conformance, intervention robustness, evidence quality, privacy, and field-evaluation contracts for peace-support systems
owner: research/evaluation/civic-systems
authority: MeasurementOnly
related:
  - PIRB_FIELD_EVALUATION_PROTOCOL_TEMPLATE_V0_1.md
  - STRATEGIC_CONFLICT_CAMPAIGN_AND_OCCUPATION_SIMULATION_V0_1.md
  - INTERSETTLEMENT_TREATY_STANDARDS_AND_MUTUAL_AID_RUNTIME_V0_1.md
  - SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
---

# Peace Infrastructure and Resilience Benchmark (PIRB) v0.1

## Status and claim ceiling

This is a research and benchmark proposal. The accompanying fixture corpus is synthetic. It is not evidence of field effectiveness, a conflict forecast, a deployment authorization, or proof that Symtropy, Symthaea, Mycelix, or Sol-Atlas currently reduces violence.

PIRB is intended to make peace-support software falsifiable and safely comparable. It complements existing peacebuilding impact frameworks; it does not replace local peacebuilders, affected communities, established evaluation standards, humanitarian organizations, or public authority.

## 1. Paradigm

The unit of improvement is not a global peace score. It is a bounded, evidence-bearing intervention hypothesis:

- a specified context and affected population;
- a stated causal mechanism that could plausibly be changed;
- an intervention that a legitimate authority could choose to attempt;
- resources, dependencies, hard constraints, and plausible failure modes;
- measurable outcomes and a baseline or defensible comparison;
- uncertainty, data limitations, and an independent review path.

Keep outcomes separate. A candidate must first satisfy legal, human-rights, civilian-safety, consent, privacy, and minimum-essential-service constraints. Only then may it be compared on distinct service, distributional, robustness, cost, time, and environmental dimensions. Do not collapse those dimensions into a compensating scalar. A higher economic total cannot make an ineligible intervention eligible.

## 2. Non-goals and safety boundaries

PIRB must not become:

- a system for ranking people, ethnicities, religions, political groups, or communities by predicted propensity for violence;
- a targeting, surveillance, coercive population-control, sanctions, or aid-denial engine;
- an autonomous negotiator, judge of political legitimacy, or authority to activate a ceasefire or settlement;
- a substitute for consent, mediation, diplomacy, rights review, or local decision-making;
- a mechanism that publishes sensitive locations, private testimony, mediator identities, beneficiary identities, or exploitable infrastructure vulnerabilities;
- a claim that a synthetic simulator has forecast real conflict or demonstrated that lives were saved.

The default mode is MeasurementOnly. Any consequential recommendation requires a named, legitimate human decision owner, an inspectable basis, explicit uncertainty, and a way to challenge the recommendation. Missing mandatory evidence or unclear authority must return UNKNOWN/BLOCKED, never an implicit pass.

## 3. Proposed cross-project responsibilities

These are interface intentions, not claims that integration is implemented.

| Project | Potential responsibility | Required boundary |
|---|---|---|
| Symthaea | Maintain competing causal hypotheses; expose evidence lineage, uncertainty, contradictory evidence, and abstention reasons | A plausible causal story is not a proven cause; model output does not grant authority |
| Mycelix | Represent consent, parties, commitments, dispute/appeal processes, resource receipts, and auditable changes | A ledger receipt proves only the event within its trust and verification boundary; it does not prove legitimacy or physical completion |
| Symtropy | Run reproducible, bounded scenario comparisons and verify lifecycle/state-machine invariants | Simulation outcomes are counterfactuals, not field observations; do not model a single universal war/peace score |
| Sol-Atlas | Supply appropriately aggregated spatial evidence about services, resources, dependencies, and recovery | Spatial correlation is not causation; sensitive layers must be access-controlled or suppressed |

Adapters must declare source, time, spatial and population scope, units, freshness, access policy, uncertainty, transformation history, and responsible authority. No project should silently inherit another project's claims or permissions.

## 3.1 Existing integration surfaces (reviewed 2026-10-18)

Prefer composition with existing work; do not create parallel evidence, civic-case, or service-capacity authorities.

- [Sol-Atlas PR #158: evidence and model-run contracts for civic intelligence](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/158) proposes versioned observations, run manifests, and evidence/reproducibility/calibration/causal-validity gates.
- [Sol-Atlas PR #116: evidence-aware spatial-economic resilience analysis](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/116) proposes service floors, bounded capacity, network flow, explicit scenarios, and distributional analysis.
- [Sol-Atlas PR #109: civic solutions portfolio and case ledger](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/109) proposes evidence-bearing multi-domain cases and independent resolution evidence.
- [Sol-Atlas PR #103: Johannesburg essential-service reliability pilot](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/103) scopes a service-reliability pilot with explicit claim boundaries.
- [Mycelix issue #5199: evidence and consent for community service agreements](https://github.com/Luminous-Dynamics/mycelix/issues/5199) tracks the downstream civic/mediation contract.

These items were open draft work or an open issue at review time; they are not represented here as merged, integrated, deployed, or validated. This specification supplies a cross-project benchmark and claim boundary rather than replacing their implementation ownership.

## 4. Benchmark tracks

### PIRB-0 — Evidence and provenance

Test source identity, versioning, timestamps, units, scope, transformations, missingness, conflicting reports, and freshness. An unknown value must remain unknown through aggregation unless a documented transformation justifies a narrower claim.

**Reference target:** 100% of seeded missing-required-field and corrupted-digest cases are rejected or reported as explicitly unknown; no test may silently convert missing or conflicting evidence into a positive fact.

### PIRB-1 — Agreement and authority semantics

Test proposed agreements, required parties, representation, separate acceptance records, effective windows, amendments, withdrawal/exit, appeals, and obligation execution. Declaration, agreement, implementation, and observed outcome are distinct states.

**Reference target:** 100% agreement with the finite reference state machine for tiny fixtures; zero transitions that activate an agreement without every required acceptance and applicable authority/effective-time checks. A simulator cannot establish whether a real party is legitimate or whether a signature was freely given.

### PIRB-2 — Service resilience and equitable allocation

Start with fictional shared-water, power-for-clinics, food distribution, and access-to-services cases. Model typed dependencies, limited budgets, outages, uncertainty, and minimum-service floors. Report both total output and the distribution of outcomes.

**Reference target:** 100% agreement with exhaustive enumeration on declared tiny allocation cases; zero eligible candidates may violate a mandatory floor. Report worst-served group, service gaps, feasibility, cost, time, and Pareto trade-offs separately. If no feasible plan exists, report infeasibility and the authority required to review constraints; never relax a rights/service floor silently.

### PIRB-3 — Robust decisions under uncertainty

Compare static plans with contingent plans that use only evidence available at the corresponding decision time. Include shocks, delayed observations, unavailable suppliers, stale capacity, conflicting reports, and distribution shift.

**Reference target:** zero look-ahead leakage in tests; deterministic results under declared input permutations; exact agreement with a separate exhaustive oracle on bounded cases. Report worst-case outcome, per-scenario outcome, regret, resource use, and any infeasibility. Do not interpret an unweighted synthetic scenario count as a real-world probability.

### PIRB-4 — Privacy and abuse resistance

Threat-model both intended use and foreseeable misuse. Include unauthorized data joins, re-identification attempts, retention violations, leaked secrets in logs, coerced consent, manipulated reports, and unsafe map granularity.

**Reference target:** zero forbidden fields in the shared fixture contract; all specified authorization and retention violations are caught in the test suite; every high-risk failure has a documented response owner. Passing synthetic tests is not proof of field privacy or security.

### PIRB-5 — Field impact and durability

Only after local governance, ethics, safety, and data-protection review, compare an intervention to a credible baseline. Select methods appropriate to the setting: randomized designs where ethical and feasible; otherwise matched, quasi-experimental, interrupted time-series, contribution-analysis, or longitudinal approaches.

Preregister the primary outcomes, comparison, observation period, subgroup reporting, missing-data policy, spillover analysis, stop conditions, and smallest practically meaningful effect with local stakeholders.

Measure at least:
- civilian safety and recurrence of the specific harm the intervention targets;
- access, continuity, and time to restore essential services;
- the distribution of benefits and burdens, including the worst-served groups;
- participation, contestability, perceived procedural fairness, and agreement durability;
- costs, implementation failures, displacement of harms, data incidents, and adverse effects.

Report point estimates and uncertainty. Do not attribute a fall in violence to software without a design that supports that inference. Where reliable comparison is impossible, report descriptive evidence and its limits rather than manufacturing a causal claim.

## 5. Measurement contract

Every benchmark result must include:

- exact code/configuration/version and input-corpus digest;
- benchmark track, test population, geography/time scope, units, seed policy, and exclusions;
- reference-oracle identity and the independent checks used;
- all required and missing evidence, failed gates, and abstentions;
- per-case results, aggregate summaries, subgroup/distributional outcomes, and uncertainty;
- baseline, negative controls, sensitivity checks, and known limitations;
- whether the result is synthetic, replayed historical, observational, or causally evaluated in the field.

Use separate labels: NOT_RUN, PASS, FAIL, UNKNOWN, BLOCKED, and NOT_APPLICABLE. Queued, skipped, stale, or absent execution is not PASS. Synthetic correctness, forecast skill, operational reliability, institutional legitimacy, and real-world impact are different evidence classes; never promote one into another.

For warning-to-response evaluations, report the declared response-time reserve separately from the response-latency estimate. A response is operationally feasible only when lead time is strictly greater than the latency bound plus that reserve, in addition to explicit authorized ownership and resource availability. A response may be marked verified only when its ordered lifecycle trace (issued, acknowledged, dispatched, arrived, action completed, outcome independently verified) has monotonic non-negative event offsets, unique event IDs, evidence references, and an explicit independent attestation for the final outcome. Reserve values are scenario inputs for synthetic conformance, not universal safety policy or empirically calibrated constants.

## 6. First reference corpus

The shared fixture contract is specified in `../schemas/peace-infrastructure-resilience-v0.1.schema.json`. The initial synthetic corpus is intentionally small and adversarial. It covers:
1. a resource allocation that maximizes total output while breaching a declared minimum-service floor;
2. a proposed agreement missing a required party's acceptance;
3. a safety-relevant capacity datum that is stale or missing;
4. two conflicting reports that must remain contested until appropriately resolved;
5. a sensitive person-level field that must not enter the shared evidence record;
6. an adaptive recovery plan that may branch only after an observation becomes available;
7. a community declining participation, which must not be silently treated as consent;
8. multiple service-recovery options evaluated over baseline, drought, and supplier-disruption scenarios, with hard gates and a separate cost/worst-case-gap Pareto frontier rather than invented scenario probabilities;
9. a before/after case in which one area improves while a neighboring area worsens, so aggregate service gap does not improve and before/after-only data cannot establish causality;
10. an essential-service warning with different combinations of authorized ownership, resources, response lead time, dispatch state, and independently verified response evidence, so an alert is not confused with an executed response or proven prevention. It now includes a hard boundary where nominal latency slack exactly equals the declared reserve, and a complete synthetic event trail with negative controls for event ordering, provenance, and independent outcome attestation;
11. a four-community network case in which aggregate service gap improves from 36 to 34 while two communities worsen. The evaluator must publish the exact improved/worsened group sets and require a qualified-with-displacement claim rather than an unqualified system-wide success label.
12. an adaptive stop-rule state machine with mandatory block conditions, review triggers for unassessed spillover/post-hoc metric changes/suppressed appeals, and fail-closed behavior for unknown evidence or authority.

These fixtures test semantics and failure handling only. Numeric quantities are fictional and are not policy thresholds. The network-spillover case is a falsification fixture: it does not estimate actual community effects or causal displacement. Later versions may add multilingual materials, realistic public infrastructure topologies, historical replay sets with documented provenance, and independently governed field-evaluation protocols.

## 7. Gate sequence

**Gate A — Reproducible contracts:** validate fixture syntax, schema, exact state transitions, invariants, and deterministic replay.

**Gate B — Independent challenge:** test against a separately implemented small-instance oracle, seeded mutations, missing-data cases, input permutations, and adversarial fixtures.

**Gate C — Privacy and governance review:** verify data-flow boundaries, access policies, threat models, appeal/exit processes, and decision authority.

**Gate D — External evaluation design:** agree on outcomes and stop conditions with local partners; preregister a defensible comparison before intervention outcomes are reviewed. Use the [PIRB field-evaluation protocol template](PIRB_FIELD_EVALUATION_PROTOCOL_TEMPLATE_V0_1.md) as a checklist, not as approval or evidence that a study has been authorized.

**Gate E — Field evidence:** publish limitations and adverse results as well as positive results. A deployment can be technically correct and still be ineffective, illegitimate, inequitable, or unsafe.

No later gate is implied by passing an earlier one.

### Quantitative acceptance targets

For the **synthetic v0.1 reference suite**, acceptance is binary and invariant-specific—not an averaged peace score:

- **Bounded decision oracle:** 100% exact agreement with the independent exhaustive oracle on every enumerated small-instance candidate and Pareto-frontier membership; unknown/failing hard gates must never be eligible.
- **Authority and consent:** zero active agreements when a required acceptance or authority proof is missing; refusal and non-participation remain non-consent.
- **Evidence integrity:** 100% detection of required negative controls for stale mandatory evidence, contradictory uncorroborated claims, missing provenance, duplicate event IDs, and out-of-order lifecycle events.
- **Warning-to-response feasibility:** zero alerts classified actionable when owner authorization or resources are missing, the reserve is missing/invalid, or lead time is less than or equal to response-latency bound plus declared reserve.
- **Response verification:** zero responses marked verified without the full ordered lifecycle, non-empty evidence references, monotonic event offsets, and an independently attested terminal outcome.
- **Privacy and non-maleficence:** zero prohibited sensitive-field mutations accepted; zero mandatory minimum-service-floor violations accepted; zero cases in which a net aggregate reduction hides worsened groups without an explicit distributional-harm qualification. Group-level impact sets must match the observed inputs exactly.
- **Schema and reproducibility:** the checked-in schema must validate against the Draft 2020-12 meta-schema; the reference corpus must validate against that schema before semantic checks run; all schema-invalid mutations fail closed. Receipts bind the exact source commit, corpus digest, schema digest, dependency lock digest, and toolchain metadata; tested scenario/alert reordering must not change semantic outcome.
- **Impact claims:** zero causal prevention claims accepted from synthetic runs or before/after-only observations. Field impact requires a separately governed evaluation design and empirical evidence.
- **Adaptive stops:** zero `CONTINUE` transitions after a mandatory floor breach, mandatory evidence staleness/conflict, prohibited sensitive field, missing authorized owner, unknown evidence state, or unregistered trigger; material unassessed spillover, post-hoc metric changes, and suppressed appeals must enter `REVIEW_REQUIRED`.
- **Stop-rule state space:** exact agreement with a separately structured test oracle over all 128 combinations of eight registered trigger classes, four evidence states, authorized-owner present/absent, and assessed/unassessed scope.

These are software-conformance targets only. The 15-minute reserve in the current fictional fixture is not a policy recommendation, a universal safety standard, or an empirically calibrated response-time model. Future operational thresholds must be set and reviewed by legitimate local actors with context-specific evidence. Passing this suite does not demonstrate conflict reduction, causal prevention, legal legitimacy, or deployment safety.

## 8. Research anchors

- United Nations and World Bank, *Pathways for Peace: Inclusive Approaches to Preventing Violent Conflict* (2018): https://www.worldbank.org/en/topic/fragilityconflictviolence/publication/pathways-for-peace-inclusive-approaches-to-preventing-violent-conflict
- Alliance for Peacebuilding, *Guiding Standards for Peacebuilding Design, Monitoring, and Evaluation* (March 2025): https://www.allianceforpeacebuilding.org/afp-publications/guiding-standards-for-peacebuilding
- United Nations Peacebuilding Impact Hub and repository of impact-measurement tools: https://www.un.org/peacebuilding/en/impact-hub and https://www.un.org/peacebuilding/en/content/repository-peacebuilding-impact-measurement-tools
- UNDP, *Evidence for Prevention* (March 2026): https://www.undp.org/publications/evidence-prevention
- ODI, *Early warning and early response to violence for civilian safety and security* (October 2025): https://odi.org/en/publications/early-warning-and-early-response-to-violence-for-civilian-safety-and-security/ — notes that warning systems often lack strong response links and that community-led approaches can be more sustainable.
- UNDRR Regional Office for Africa, *Ensuring early warnings and early action reach fragile, and conflict-affected people* (18 December 2025): https://www.undrr.org/news/ensuring-early-warnings-and-early-action-reach-fragile-and-conflict-affected-people — describes early-warning systems in conflict-affected settings as often fragmented or poorly linked to action.
- Alliance for Peacebuilding, *Localized Peace, Global Application Gaps, Challenges, and the Path Forward for Peacebuilding* (8 May 2026): https://www.allianceforpeacebuilding.org/afp-publications/au-practicum2025 — emphasizes barriers between commitments to locally led peacebuilding and implementation.
- UNDP Digital Standards, *Do no harm*: https://www.undp.org/digital/standards/5-do-no-harm
- ICRC, *Handbook on data protection in humanitarian action* (2024): https://www.icrc.org/en/data-protection-humanitarian-action-handbook
- OECD, *Evaluating Donor Engagement in Situations of Conflict and Fragility* (2012): https://www.oecd.org/en/publications/evaluating-donor-engagement-in-situations-of-conflict-and-fragility_9789264106802-en.html

These references motivate design constraints; they do not endorse PIRB or validate its implementation.
