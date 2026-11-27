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

The external comparator, reproduction and acceptance protocol is maintained in [benchmark-matrix.md](benchmark-matrix.md). The paired South African source-qualified aggregates are frozen in [south-african-port-energy-evidence-v0.tsv](fixtures/south-african-port-energy-evidence-v0.tsv); their pre-feasibility and estimate-only claim ceilings are explained in the matrix. The distinction between synthetic counterfactual, observed operational difference and identified causal effect is defined in [causal-evaluation-protocol.md](causal-evaluation-protocol.md), composing Symtropy #3152 and #5108 rather than duplicating their estimators. Symtropy #4204 remains the owner of public ship-block dataset admission and faithful reproduction; this benchmark consumes its outputs only after those source checks pass.

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

Eighteen Rust unit tests and four integration tests are authored. The unit suite covers the five hand-calculated known answers, late and partial receiver acceptance, failed/unknown inspection, rejected/unknown receiver disposition, separate inspection and receiver revision mismatch, missing/duplicate evidence records, over-qualification/over-acceptance, receiver-day overflow, input-order invariance, unknown/duplicate dependencies and unsupported shock timing. Integration coverage includes an independent base-receipt oracle, a 30-case adversarial corpus, and a Mycelix #6774 owner-receipt composition test that independently parses capacity/feasibility outputs and checks their downstream service effect. The exact-head workflow runs formatting, locked tests, strict Clippy and the deterministic CLI receipt.

**Qualification status:** the v0.1 code-bearing baseline passed dedicated exact-head workflow [run #17](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45947986837), and the subsequent v0.1 branch head also passed the dedicated research workflow and repository CI (runs #18 and #22623). The present v0.4 extension separates inspection qualification from receiver acceptance and adds a 30-case adversarial corpus; it has not yet completed its own exact-head qualification. The authoring environment has no Cargo/rustc, so no local Rust build/test/format/Clippy result is claimed.

## What this fixture does not establish

It does not establish actual supplier independence, actual regional capacity, physical production quality, class/statutory compliance, a real grid outage distribution, cost competitiveness, emissions performance, job creation, causal economic impact, a South African port capacity fact or a real receiver delivery.

The result is conditional on the exact declared synthetic dependency topology, inspection records, receiver-acceptance records and timing rule. An incomplete dependency inventory must yield UNKNOWN/INDETERMINATE in a future cross-stack adapter, not an inferred independent path.

## Next qualification steps

1. Complete exact-head qualification of v0.4; address every failure without weakening gates.
2. Replace the frozen Mycelix receipt snapshot with an exact-digest, freshness-, rights- and authority-bound owner adapter.
3. Extend both evidence records to actual source IDs, timestamped observations, custody/dispatch/handoff provenance and participant authority from Mycelix's owner-native artifacts.
4. Add transitive dependency closure, stale/revoked source evidence, unknown dependency remainder, inspection measurement uncertainty, partial dispatch/receipt and cancellation/rework states.
5. Extend beyond the current 30 cases to at least 30 frozen cases and held-out facility/shock families.
6. Consume owner-native outputs from #6773, #5762 and #3683 by exact version/digest.
7. Reproduce the strongest matched port-energy, shipyard and service-chain baselines before making any performance claim.

A passing fixture is evidence of this model's arithmetic and declared topology only. It is not a physical or industry qualification.
