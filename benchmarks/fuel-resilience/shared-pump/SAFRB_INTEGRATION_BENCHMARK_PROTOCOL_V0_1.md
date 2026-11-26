# SAFRB integrated assurance benchmark protocol v0.1

**Status:** proposed benchmark design; experiments not yet run  
**Parent:** SAFRB deliverability #7483; global interoperability profile #7507  
**Known-answer:** `SAFRB-SHARED-PUMP-2ROUTE-001`  
**Scenario SHA-256:** `0a1c507d3bff9b3062e24ca8718e102244473d839c2a31c5ead31812ee249be2`

## 1. Research question

Does composing physical feasibility, federated claim/evidence qualification, decision-aware evidence acquisition and read-only spatial projection produce **more defensible essential-service decisions** than the strongest matched single-function or human baselines, under the same evidence, time, cost, permission and safety budgets?

This is an end-to-end composition benchmark, not a replacement for each repository's owner-native component suite.

## 2. External systems and research baselines

The benchmark must compare against existing capabilities honestly. The named products are functional reference points; this protocol does not claim to have run them or reproduce their internal algorithms.

| System / programme | Publicly documented capability | SAFRB relationship and fair comparison |
|---|---|---|
| DTN TIMS | Automated terminal inventory gathering, real-time inventory and receipt/loading events, product/location/time views and alerts | Ingest its admitted inventory/event outputs as candidate evidence. SAFRB tests whether stock is admitted, accessible, transferable and sufficient for a scoped service obligation. Do not claim TIMS lacks inventory visibility. |
| AVEVA PI System | Collects, stores, normalizes, contextualizes, visualizes and delivers operational data across industrial assets | Treat as a source/historian/context layer. Test lineage, freshness, source dependence and measurement admission at the assurance boundary; do not replace historian functionality. |
| Schneider Electric EcoStruxure Microgrid Advisor | DER monitoring, forecasts, APIs and model-predictive microgrid optimization | Consume its declared forecasts/plans as candidate inputs. Independently qualify external fuel/resource paths and beneficiary-bound service evidence. Do not rank against a different EMS objective or boundary. |
| Siemens Gridscale X | Integrates grid planning, operations, asset management and metering through a shared digital-twin platform | Treat as a grid operations/digital-twin comparator. Test cross-owner, cross-sector service claim semantics and independent evidence preservation, not generic dashboard breadth. |
| NIST SI for Manufacturing / TIDAL | Research architecture for trustworthy industrial intelligence, human-machine teaming, contextualized data, digital-twin VVUQ, integration test bench and benchmark studies | Align with its measurement science, context ablations, human teaming and test-bench discipline. SAFRB's narrower hypothesis is infrastructure-owner federation and proof-qualified resource-to-beneficiary services. |
| NIST IR 8536 | Manufacturing traceability meta-framework using linked provenance, cryptographic links and selective disclosure without a centralized repository | Reuse these traceability and selective-disclosure principles through Mycelix. SAFRB adds authority recognition, physical deliverability and service qualification; do not reinvent a generic provenance chain. |

Sources:
- DTN TIMS: https://www.dtn.com/refined-fuels/terminal-operator/tims/
- AVEVA PI System: https://www.aveva.com/en/products/aveva-pi-system/
- Schneider Electric EcoStruxure Microgrid Advisor: https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/
- Siemens Gridscale X: https://www.siemens.com/en-us/products/gridscale-x/
- NIST SI for Manufacturing / TIDAL: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- NIST IR 8536: https://csrc.nist.gov/pubs/ir/8536/final

## 3. Matched experimental arms

Every arm receives the same frozen decision-time observation, service requirement, resource graph, candidate actions, cost/time budgets and held-out case IDs. The product rows above are not silently simulated as if they were vendor runs.

| Arm | System under test | What it isolates |
|---|---|---|
| A | Nominal dashboard / inventory-only decision | Common failure mode: nominal stock and declared capacity treated as available service |
| B | Fixed diagnostic checklist | Non-adaptive human-authored evidence sequence |
| C | Maximum / greedy information gain | Information reduction without service-decision utility |
| D | Decision-theoretic EVSI / VoI | Decision value with matched costs, delays and supported model family |
| E | Symtropy physical certificate only | Physical feasibility under admitted capacity inputs |
| F | Symtropy + Mycelix | Added identity, authority, freshness, provenance and common-cause qualification |
| G | F + Symthaea | Added proof-obligation-directed, model-robust investigation policy |
| H | G + Sol-Atlas projection | Added spatial/temporal observability; no change to canonical qualification |
| I | Human/operator investigation | Expert baseline under the same information, permission, time and cost budget |

Evaluate the full stack as H plus the Power Fabric service-claim adapter. The adapter must preserve claim scope and disposition, not recalculate the physical model.

## 4. Required scenario families

1. **Single-commodity physical flow:** shared pump, shared berth, shared pipeline segment, shared power feeder, capacity lower/upper bounds, outages and time-window cut sets. Use max-flow/min-cut only for network forms that satisfy its assumptions.
2. **Process-coupled and multi-commodity:** fuel grades, product compatibility, nonlinear conversion, storage, integer schedules and multi-resource constraints. Use an appropriate independently checkable LP/MILP/constraint proof; otherwise return unsupported, unproven or indeterminate.
3. **Evidence ambiguity:** stale, missing, revoked, partial, conflicting, future, replayed and out-of-scope observations; invalid calibration; correlated dashboards; alias substitution; source common causes.
4. **Authority and federation:** key authentication versus purpose recognition, title/custody/contract/access versus operational permission, revoked and conflicting credentials, selective disclosure, offline reconciliation and cross-jurisdiction profile changes.
5. **Active investigation:** actions with different ability to resolve a proof obligation, cost, delay, downtime, safety prerequisite, source dependence, refusal and inconclusive outcomes.
6. **Geospatial and temporal projection:** coordinate uncertainty, different CRS/datum, non-whole-hour offsets, daylight-saving folds/gaps, stale positions, offline updates, duplicated assets and cross-layer confusion.
7. **Beneficiary service and equity:** critical service floors, priority classes, rural/remote cohorts, affordability, recovery time and unserved demand. Keep service minima non-compensatory.

## 5. Non-compensatory gates

A run fails qualification if any of these occur:

- false guaranteed service on any hard-negative control;
- unsupported model class or invalid certificate accepted as proof;
- shared physical resource counted more than once;
- stale, future, revoked, conflicting or out-of-scope evidence promoted into a current guarantee;
- issuer key authentication treated as purpose recognition, title or dispatch authority;
- authority, certainty, currentness, specificity or evidence maturity strengthened by an adapter;
- unauthorized disclosure, inspection request, OT write, inventory mutation or equipment actuation;
- communication reachability treated as physical delivery, or a spatial visualization treated as physical feasibility;
- unknown dependency represented as independent;
- failure of an explicitly required beneficiary service floor.

A run that rejects everything is not useful: supported positive controls and unsupported/ambiguous controls must both be present and measured.

## 6. Metric vector (do not collapse to a weighted score)

**Physical:** exact flow/witness/cut agreement, conservation, false guarantees, false rejection, shared-resource double-counts, unsupported model refusals, capacity-bound sensitivity, p50/p95 verifier latency.

**Evidence and federation:** source-to-output traceability, semantic preservation, stale/conflict/revocation detection, alias/common-cause fidelity, authority/certainty/currentness/specificity promotion count, disclosure excess, offline conflict preservation.

**Investigation:** decision regret, false-safe rate, justified-decision change rate, proof-obligation coverage, redundant tests, acquisition cost, elapsed time, downtime, refusal and human-escalation rates, model-adequacy failures.

**Spatial:** topology and identity fidelity, route/resource alias correctness, CRS/time transformation errors, stale-position promotion, source maturity and claim-state preservation.

**Beneficiary outcomes:** critical service-hours delivered, worst-cohort shortfall, unserved demand, affordability, recovery time, environmental impact and local operator participation.

**System overhead:** end-to-end assurance latency as a fraction of the actual decision budget; compute, data transfer, human review and integration cost.

## 7. Prospective targets (not achieved results)

| Gate / target | Initial research target | Qualification note |
|---|---:|---|
| False guaranteed service | 0 / 3,000 stratified hard-negative cases | Report exact denominators and one-sided confidence interval; cases must span independent topology/failure families, not merely edge permutations |
| Positive-control false rejection | <=1% on >=300 supported positive cases | Unsupported cases are reported separately, not counted as positives |
| Flow/cut and supported certificate verification | 100% on all known-answer panels | Independent verifier implementation and frozen inputs |
| Semantic or authority promotion | 0 | Across all adapters, profile upgrades and locale transformations |
| Undeclared disclosure / unauthorized operational effect | 0 | Adversarial privacy and permission tests |
| Decision regret | >=20% lower than the strongest matched checklist / EIG / EVSI baseline | No increase in false-safe rate; include human/operator arm |
| Assurance overhead | P95 <=10% of available decision time budget | Include evidence acquisition delay separately from compute latency |
| Conformance breadth | 10 jurisdiction overlays, 5 languages/scripts, 4 connectivity modes | Synthetic conformance first; no claim of actual jurisdictional approval |
| Outcome distribution | No breach of declared critical floor in any protected cohort | No average score may compensate for a cohort floor breach |

For rare-event gates, publish exact binomial bounds and dependence assumptions. Zero errors in 3,000 independent negative cases supports an approximate one-sided 95% upper bound near 0.1%; correlated test variants do not count as independent trials.

## 8. Experimental protocol

- Freeze input cases, model-family definitions, source identities, resource dependency graphs, legal/profile assumptions, allowed actions, budget and artifact digests before a run.
- Separate planner-visible observations from oracle-only truth. No future receipt, assay, outage, title outcome or service result may leak into the planner.
- Use topology-family and jurisdiction-family holdouts, not random row splits. Include unseen combinations of route, common cause, evidence provider, locale and connectivity.
- Match compute budgets and information access. Report compute used, operator time, acquisition delay and refusal.
- Run ablations by removing exactly one component at a time. Attribute improvements only to the component whose contribution survives the ablation.
- Require independent implementations for certificate verification and the small-instance optimal-policy oracle. Do not share the candidate algorithm's core helper with its verifier.
- Publish all failures, abstentions, counterexamples, denominators, confidence intervals, commit SHAs and source digests.
- Keep synthetic, replay, SIL, HIL, commissioning and field evidence as separate claim classes.

## 9. SOTA claim rule

A superiority claim is permitted only when the full integrated system beats the strongest matched open, commercial-accessible or human baseline on a preregistered held-out set, under identical data, budgets and service boundaries, with no hard-gate regression. Vendor feature descriptions and synthetic known-answer passes are not benchmark results.

A credible first win is not "better forecasting" or a higher aggregate score. It is **fewer false service guarantees, lower decision regret and lower evidence-acquisition burden while preserving authority, provenance, privacy and critical-cohort service floors**.

## 10. Deployment/test ladder

1. Synthetic known-answer and adversarial fixtures.
2. Frozen historical replay with independently checked source and event lineage.
3. Shadow integration against read-only vendor exports and owner-authorized records.
4. Lab/HIL for applicable microgrid control/energy-management systems, aligned to IEC TS 62898-3-5:2026 where in scope.
5. Commissioning and periodic requalification under local authority and operator-approved test plans.
6. Field evaluation with beneficiary service measurement and independent audit.

No step is implied by passing the previous step. This protocol does not authorize OT control or physical tests.
