# Maritime Capability-to-Service Counterfactual (MCSC) v0.1

Research artifact for [Symtropy #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686).

## Purpose

This fixture isolates one question:

**Does the apparent addition of regional suppliers improve receiver-delivered service under a shared upstream disruption, or does only an independently qualified alternative path provide recoverability?**

It composes, but does not replace:

- Mycelix #6773 / #4814: regional productive capability and supplier/resource evidence.
- Mycelix #5762: declared multi-valued/transitive dependency and common-cause semantics.
- Symtropy #3683: conserved expected-service denominator and receiver acceptance.
- Symtropy #2048 / #4294: work-package, repair and lifecycle semantics.

This artifact is a synthetic counterfactual composition reference, not a second supplier graph, production scheduler, optimizer, cargo ledger or physical model. Each OPTION carries a configuration revision and must have exactly one INSPECTION and one RECEIVER_ACCEPTANCE record using that same revision. INSPECTION PASS can qualify a partial quantity; FAIL/UNKNOWN qualifies zero. RECEIVER_ACCEPTANCE ACCEPTED can accept no more than the inspection-qualified quantity; REJECTED/UNKNOWN accepts zero. Only receiver-accepted units that also meet the service deadline count as on-time accepted service.

The external comparator, reproduction and acceptance protocol is maintained in [benchmark-matrix.md](benchmark-matrix.md). The paired South African source-qualified aggregates are frozen in [south-african-port-energy-evidence-v0.tsv](fixtures/south-african-port-energy-evidence-v0.tsv); their pre-feasibility and estimate-only claim ceilings are explained in the matrix. A Rust integration test enforces MVA/MW separation, UNKNOWN handling and evidence-class/claim-ceiling consistency. The distinction between synthetic counterfactual, observed operational difference and identified causal effect is defined in [causal-evaluation-protocol.md](causal-evaluation-protocol.md), composing Symtropy #3152 and #5108 rather than duplicating their estimators. The time-bounded offer/qualification state machine and SOVQC lifecycle extension are defined in [capability-lifecycle-protocol.md](capability-lifecycle-protocol.md), composing Mycelix #6533, Symtropy #5710 and #3121. The common-cause recovery and decision-regret lane is defined in [resilience-regret-protocol.md](resilience-regret-protocol.md) and tracked in #7747. The new [KITE information-set replay protocol](information-set-regret-protocol.md), tracked in #7775, separates clairvoyant hindsight bounds from same-information decision oracles and freezes late-evidence/no-lookahead controls in `fixtures/information-set-replay-v0.tsv`. A Rust integration conformance test at `tests/information_set_replay.rs` parses the 24-field, 20-case manifest and recomputes visibility, evidence admissibility and action executability. The follow-on reusable library API in `src/kite.rs` adds append-only revision-lineage validation, deterministic as-of vintage selection, separate visibility/ingestion/validation/expiry/rights/authority gates, and action executability checks; the API has unit and external integration tests. It remains a benchmark primitive, not a production evidence store, source authenticator or authorization engine. Symtropy #4204 remains the owner of public ship-block dataset admission and faithful reproduction; this benchmark consumes its outputs only after those source checks pass.

## Frozen fixture

File: `fixtures/mcsc-counterfactual-tiny-v0.tsv`

All values are synthetic benchmark units. The service unit is an abstract cargo unit; it is not a container, TEU, tonne or real shipment. Each portfolio selects one fully qualified alternative for one critical maintenance work package. The base case has a passing inspection and a receiver-accepted 10-unit output at the declared configuration revision. The separate inspection and receiver records are deliberate: passing inspection is not proof of delivery or receiver acceptance.

The service obligation is 10 expected units with a receiver deadline at day 10. The denominator remains 10 for every portfolio, including late, partial, rejected and unknown outcomes.

### Dependency topology

- LOCAL-A and LOCAL-B are separate regional facilities.
- Both depend on MILL-01, GRID-A and NDT-01.
- LOCAL-C is a regional alternative with separate MILL-02, GRID-B and NDT-02 dependencies.
- IMPORT uses a foreign supply path and does not consume the declared regional dependencies.
- EXISTING is the no-new-investment / existing-capability baseline.

Two regional supplier names (LOCAL-A and LOCAL-B) therefore do not imply two independent production paths. The frozen disruption makes GRID-A unavailable over the half-open interval [day 0, day 6). A dependency becomes available at day 6.

### Timing rule

This micro-instance schedules every portfolio from day 0. For each required dependency, compute its effective availability as the later of its declared availability and the end of any outage interval that overlaps day 0. The ready day is the maximum effective availability across the option's dependencies, or day 0 if it has none.

`ready_day = max(0, effective_available_day(d) for d in D)`

`receiver_day = ready_day + production_days + inspection_days + commissioning_days + handoff_days`

The parser rejects shocks that do not begin at day 0: later-starting shocks, partial resource recovery, overlapping work and stochastic processing times require an event simulator and are not approximated by this arithmetic reference. Duration arithmetic is checked for overflow. Input record ordering and dependency ordering do not change the canonical receipt.

For this tiny case, each option's production/supply duration is a fixed synthetic duration. A real benchmark must use source-qualified time windows and distributions rather than treating these fixture values as estimates.

On-time accepted service is the full expected-unit quantity when `receiver_day <= deadline_day`; otherwise it is zero on time. Late delivery remains late, not deleted from the expected denominator. This all-or-nothing batch rule is deliberately simplistic and must not be mistaken for a realistic cargo or vessel model.

## Expected results

| Portfolio | No-shock receiver day | GRID-A outage receiver day | Inspection-qualified | Receiver-accepted | On-time accepted |
|---|---:|---:|---:|---:|---:|
| LOCAL-A | 8 | 14 | 10/10 | 10/10 | 0/10 |
| LOCAL-B | 9 | 15 | 10/10 | 10/10 | 0/10 |
| LOCAL-C | 10 | 10 | 10/10 | 10/10 | 10/10 |
| IMPORT | 13 | 13 | 10/10 | 10/10 | 0/10 |
| EXISTING | 23 | 23 | 10/10 | 10/10 | 0/10 |

The result is intentionally not a universal claim that regional production beats imports. It demonstrates a narrow structural point: LOCAL-A and LOCAL-B are not independent against the declared GRID-A shock; LOCAL-C preserves the service deadline in this frozen case despite its higher synthetic cost than IMPORT and lower synthetic regional value than LOCAL-A/LOCAL-B.

Cost, embodied emissions, regional value, receiver delay and resilience must remain separate dimensions. Do not collapse them into a weighted score.

## Independent hand calculation

- LOCAL-A under GRID-A outage: (6 + 5 + 1 + 1 + 1 = 14).
- LOCAL-B under GRID-A outage: (6 + 6 + 1 + 1 + 1 = 15).
- LOCAL-C: (0 + 7 + 1 + 1 + 1 = 10).
- IMPORT: (0 + 10 + 1 + 1 + 1 = 13).
- EXISTING: (0 + 20 + 1 + 1 + 1 = 23).

The denominator is 10 for all five alternatives. Only LOCAL-C reaches the receiver by day 10 in the outage case.

## Cross-owner productive-capability receipt

The fixture `fixtures/mycelix-mpcb-tiny-v0.receipt.tsv` is a frozen transcript of the deterministic receipt emitted by Mycelix #6774's dedicated Maritime Productive Capability Reference workflow (run #5, source commit `877716f9be9cbdf56f9c4b8be67349b9bca97a43`). Source extraction details and claim ceiling are recorded in `fixtures/mycelix-mpcb-tiny-v0.receipt-source.md`.

The receipt carries the owner-computed four sourcing plans, work-package selections, shared regional plate/welding/NDT capacities, Pareto classifications and exact blocking reasons. A new integration test independently parses that owner receipt, verifies its summary/resource blockers, and joins only the owner's **FEASIBLE** plans to a synthetic receiver-service case. The resource-infeasible all-local plan remains visible in the owner receipt and is excluded before any receiver completion time is calculated (4 t plate demand > 3 t capacity; 16 h welding demand > 12 h capacity).

For the three feasible plans, the test adds the same explicit synthetic downstream assumptions—inspection PASS for 10 units and receiver ACCEPTED for 10 units—then composes the owner's max lead time with one day each for inspection, commissioning and handoff. The A-local/B-import hybrid meets the synthetic day-20 deadline; the two other owner-feasible portfolios are accepted but late. **Those downstream PASS/ACCEPTED records are assumptions authored by the integration test, not facts emitted by Mycelix and not inferred from capability feasibility.**

This test composes the owner receipt; it does not re-run Mycelix's source enumeration or resource feasibility engine. It is not yet a production adapter, and the receipt is a log-extracted snapshot rather than a downloaded signed artifact. The future adapter must bind exact digests, freshness, authority and rights before consuming live owner outputs.

## Standalone Rust reference

The dependency-free research crate lives at `research/maritime-capability-to-service`, outside the Holochain production workspace.

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-capability-to-service/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-capability-to-service/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-capability-to-service/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-capability-to-service/Cargo.toml
```

The Rust reference parses the frozen service obligation, dependency inventory, day-zero shocks, alternatives, configuration revisions, inspection dispositions and receiver-acceptance dispositions. Every option must have exactly one inspection record and one receiver-acceptance record, both bound to its configuration revision. Inspection PASS can qualify a partial quantity; FAIL/UNKNOWN qualifies zero. Receiver ACCEPTED may accept no more than the inspection-qualified quantity; REJECTED/UNKNOWN accepts zero. The receipt separately reports expected, inspection-qualified, receiver-accepted and on-time accepted units. Missing/duplicate evidence, revision mismatch, over-qualification/over-acceptance, unknown/duplicate dependencies, unsupported shock timing and receiver-day overflow fail closed.

Twenty-six Rust unit tests and eighteen integration tests are authored. The unit suite covers the five hand-calculated known answers, late and partial receiver acceptance, failed/unknown inspection, rejected/unknown receiver disposition, separate inspection and receiver revision mismatch, missing/duplicate evidence records, over-qualification/over-acceptance, receiver-day overflow, input-order invariance, unknown/duplicate dependencies and unsupported shock timing. Integration coverage includes an independent base-receipt oracle, a 30-case service corpus, Mycelix #6774 capability and #6343 port-city owner-trace composition, canonical #7554 receiver-service receipt conformance, three South African source-evidence controls, and four common-cause/lifecycle-regret manifest controls plus a separate matched recovery-policy comparison, including an end-to-end fourteen-dimension regret vector against a clairvoyant bound and a comparison of disruption-management, full-rescheduling and proactive–reactive hybrid policies. The exact-head workflow runs formatting, locked tests, strict Clippy and the deterministic CLI receipt.

**Qualification status:** the v0.1 code-bearing baseline passed dedicated exact-head workflow [run #17](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45947986837), and the subsequent v0.1 branch head also passed the dedicated research workflow and repository CI (runs #18 and #22623). The current extension separates inspection qualification from receiver acceptance, adds a 30-case adversarial corpus, implements vector-valued service option comparison, and consumes frozen Mycelix/receiver-service owner receipts; this exact head has not yet completed its hosted qualification. The authoring environment has no Cargo/rustc, so no local Rust build/test/format/Clippy result is claimed.

## Recovery decision regret: executable vector comparator

The Rust reference now also exposes `compare_recovery_regret(candidate, reference)` over `RecoveryServiceOutcome`. It returns candidate-minus-reference deltas for unmet units at deadline, inspection-qualified units, receiver-accepted units, late accepted units, unresolved units, service-deficit unit-hours, time to minimum viable service, time to full service, recovery cost, recovery emissions and requalification-action count.

The API requires the same frozen instance identity/version, scenario, service, deadline, horizon and expected-unit denominator. It rejects impossible accepted/on-time/unresolved quantities and inconsistent recovery times. The reference may be a clairvoyant per-scenario upper bound, but that bound is not an implementable policy and carries no probability weight. No scalar regret or expected-loss figure is produced.

The synthetic unit tests exercise a nine-dimensional known answer, mismatched context, invalid recovery inputs and zero-regret replay. This qualifies only the comparator arithmetic/guards; the 20-case #7747 scenario manifest is not yet connected to an operational recovery optimizer.

## Service option value: vector comparison

The Rust reference now exposes `compare_service_options(baseline, candidate)`. It only compares outcomes from the same frozen instance, service obligation, receiver deadline and expected-unit denominator. It returns a vector of signed deltas—receiver day, inspection-qualified units, receiver-accepted units, on-time accepted units, cost, embodied emissions and regional value added. It does not produce a weighted score.

In the frozen MCSC-TINY-001 outage case, comparing EXISTING with LOCAL-C yields:
- receiver completion 13 days earlier;
- no change in inspection-qualified or total receiver-accepted units;
- 10 additional on-time receiver-accepted units;
- 55 additional synthetic cost units;
- 13 fewer synthetic embodied-emissions units;
- 60 additional synthetic regional-value units.

This is a modelled option-value vector, not a causal estimate or a recommendation to invest in LOCAL-C. The API rejects comparisons across different service denominators or deadlines.

## What this fixture does not establish

It does not establish actual supplier independence, actual regional capacity, physical production quality, class/statutory compliance, a real grid outage distribution, cost competitiveness, emissions performance, job creation, causal economic impact, a South African port capacity fact or a real receiver delivery.

The result is conditional on the exact declared synthetic dependency topology, inspection records, receiver-acceptance records and timing rule. An incomplete dependency inventory must yield UNKNOWN/INDETERMINATE in a future cross-stack adapter, not an inferred independent path.

## Next qualification steps

1. Complete exact-head qualification of v0.4; address every failure without weakening gates.
2. Replace the frozen Mycelix receipt snapshot with an exact-digest, freshness-, rights- and authority-bound owner adapter.
3. Extend both evidence records to actual source IDs, timestamped observations, custody/dispatch/handoff provenance and participant authority from Mycelix's owner-native artifacts.
4. Add transitive dependency closure, stale/revoked source evidence, unknown dependency remainder, inspection measurement uncertainty, partial dispatch/receipt and cancellation/rework states.
5. Extend beyond the current 30 cases to held-out facility/shock families, transitive dependencies and lifecycle recovery.
6. Consume owner-native outputs from #6773, #5762 and #3683 by exact version/digest.
7. Reproduce the strongest matched port-energy, shipyard and service-chain baselines before making any performance claim.

## Canonical port-city service-equivalence owner composition

Mycelix [#6343 / draft PR #6344](https://github.com/Luminous-Dynamics/mycelix/pull/6344) now provides the cross-owner port-city service-equivalence golden trace. MCSC freezes its exact JSON source and a deterministic TSV projection with source commit/blob identity in `fixtures/mycelix-port-city-service-equivalence-source.md`.

A new Rust integration test consumes all 48 owner dispositions (14 Positive, 21 Negative, 11 Unknown, 2 Conflict), three matched policy traces, the shared PCC input profiles and three heat partitions. It independently recomputes:
- PCC import at each of the four 15-minute intervals from fixed port + vessel + terminal + IT + cooling + RO − renewables;
- interval power cap breach and 15-minute integrated import energy;
- identical compute useful work and port/terminal service obligations across the policies;
- heat recovery and IT/seawater partition residuals.

The owner-known-answer values are baseline 7.4 MW peak, naive objective-only shift 9.2 MW peak with a 1.0 MW breach, and no-new-peak admitted policy 7.2 MW with unchanged 6.125 MWh import energy. All preserve 100 useful compute-work units and declared service-energy quantities.

**Claim ceiling:** this is a synthetic integration fixture from a draft owner PR, not a physically qualified port-city plant. RO product-water volume, feedwater-quality evidence and environmental discharge limits remain null/unsupplied. Heat arithmetic closure earns no potable-water or environmental credit. MCSC consumes the owner trace; it does not implement the PCC verifier, compute scheduler, thermal store, desalination process, marine forecast, spatial-rights layer or common-cause dependency graph. The exact source owner PR has not been independently qualified by its dedicated golden-trace workflow.

## Canonical receiver-service owner composition

Symtropy [#7554 / draft PR #7557](https://github.com/Luminous-Dynamics/symtropy/pull/7557) owns receiver-level service adjudication and protected service floors. MCSC does not create a competing receiver evaluator. The owner PR now includes a direct cross-owner integration test that:

- consumes the pinned Mycelix #6774 sourcing/capacity receipt;
- excludes the all-local resource-infeasible plan before any receiver completion time is calculated;
- sends only the three Mycelix-marked feasible alternatives to #7554's canonical `evaluate_receiver_service` function;
- labels the additional inspection/commissioning/handoff time and verified receiver-acceptance records as synthetic test assumptions, not Mycelix facts;
- exercises accepted/on-time vs accepted/late outcomes and a 6/10 observed + 4/10 censored case, which must yield UNKNOWN against an 80% protected floor.

The frozen #7554 service receipt is also consumed by `tests/canonical_receiver_service_receipt.rs` as a bounded conformance test for cohort arithmetic, conservative lower/upper bounds and PASS/FAIL/UNKNOWN/INVALID_EVIDENCE dispositions. That receipt is not a substitute for real custody: cargo identity/lineage and custody remain with Symtropy #6631/#6662 and Mycelix's participant-authorized evidence.

**Qualification status:** the base #7554 owner implementation passed its earlier exact-head workflow. The new Mycelix composition integration is in #7557's branch and its exact-head run #32 is queued; no pass is claimed for that new join until formatting, tests, strict Clippy and receipt generation complete.

A passing fixture is evidence of this model's arithmetic and declared topology only. It is not a physical or industry qualification.

## Same-information decision oracle (R1)

The one-epoch finite R1 implementation is specified in [same-information-oracle-protocol.md](same-information-oracle-protocol.md) and tracked in [MCSC-SAME-INFO-ORACLE-001 #7834](https://github.com/Luminous-Dynamics/symtropy/issues/7834). It composes KITE's as-of evidence snapshots and enforces one common action across scenarios with identical observable information. It enumerates bounded action policies and reports component-wise adverse bounds with scenario witnesses and a Pareto frontier without probabilities or scalar weights.

The first executable slice solves one decision epoch only. It is not a multistage optimizer or deployable recovery policy. R0 clairvoyant bounds, R1 same-information oracle results, and R2 compute-budget-matched operational policies remain separate.