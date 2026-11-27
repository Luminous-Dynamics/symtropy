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

This artifact is a synthetic counterfactual composition reference, not a second supplier graph, production scheduler, optimizer, cargo ledger or physical model.

## Frozen fixture

File: `fixtures/mcsc-counterfactual-tiny-v0.tsv`

All values are synthetic benchmark units. The service unit is an abstract cargo unit; it is not a container, TEU, tonne or real shipment. Each portfolio selects one fully qualified alternative for one critical maintenance work package. The selected output passes its declared one-day inspection, one-day commissioning and one-day receiver handoff unless the corresponding explicit negative-control case is introduced in a future corpus version.

The service obligation is 10 expected units with a receiver deadline at day 10. The denominator remains 10 for every portfolio, including late and unknown outcomes.

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

| Portfolio | No-shock receiver day | GRID-A outage receiver day | On-time units under GRID-A outage |
|---|---:|---:|---:|
| LOCAL-A | 8 | 14 | 0 / 10 |
| LOCAL-B | 9 | 15 | 0 / 10 |
| LOCAL-C | 10 | 10 | 10 / 10 |
| IMPORT | 13 | 13 | 0 / 10 |
| EXISTING | 23 | 23 | 0 / 10 |

The result is intentionally not a universal claim that regional production beats imports. It demonstrates a narrow structural point: LOCAL-A and LOCAL-B are not independent against the declared GRID-A shock; LOCAL-C preserves the service deadline in this frozen case despite its higher synthetic cost than IMPORT and lower synthetic regional value than LOCAL-A/LOCAL-B.

Cost, embodied emissions, regional value, receiver delay and resilience must remain separate dimensions. Do not collapse them into a weighted score.

## Independent hand calculation

- LOCAL-A under GRID-A outage: (6 + 5 + 1 + 1 + 1 = 14).
- LOCAL-B under GRID-A outage: (6 + 6 + 1 + 1 + 1 = 15).
- LOCAL-C: (0 + 7 + 1 + 1 + 1 = 10).
- IMPORT: (0 + 10 + 1 + 1 + 1 = 13).
- EXISTING: (0 + 20 + 1 + 1 + 1 = 23).

The denominator is 10 for all five alternatives. Only LOCAL-C reaches the receiver by day 10 in the outage case.

## Standalone Rust reference

The dependency-free research crate lives at `research/maritime-capability-to-service`, outside the Holochain production workspace.

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-capability-to-service/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-capability-to-service/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-capability-to-service/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-capability-to-service/Cargo.toml
```

The current Rust reference parses the frozen service obligation, dependency inventory, day-zero shock and alternatives; rejects unknown/duplicate dependencies and unsupported shock timing; computes no-shock and shocked receiver days; preserves the expected-unit denominator; and emits a deterministic tab-delimited receipt containing source inputs and outcomes.

Six Rust unit tests and one separately implemented Rust integration-test oracle are authored. The unit suite covers the five hand-calculated known answers, full denominator retention for a late option, input-record-order invariance, unknown-dependency rejection, duplicate-dependency rejection and rejection of later-starting shocks. The integration test independently parses the fixture and reconstructs every receipt outcome without calling the library parser or evaluator. The exact-head GitHub Actions workflow also runs formatting, locked tests, strict Clippy and the CLI receipt.

**Qualification status:** this authored crate and workflow have not yet been observed passing at the current PR head. The authoring environment has no Cargo/rustc, so no local Rust build/test/format/Clippy result is claimed. The hand calculation is an independent arithmetic check, not a separately implemented oracle.

## What this fixture does not establish

It does not establish actual supplier independence, actual regional capacity, physical production quality, class/statutory compliance, a real grid outage distribution, cost competitiveness, emissions performance, job creation, causal economic impact, a South African port capacity fact or a real receiver delivery.

The result is conditional on the exact declared synthetic dependency topology and timing rule. An incomplete dependency inventory must yield UNKNOWN/INDETERMINATE in a future cross-stack adapter, not an inferred independent path.

## Next qualification steps

1. Observe exact-head formatting, test, strict-Clippy and deterministic receipt results; address every failure without weakening gates.
2. Add mutation controls for shared transitive dependencies, stale/revoked evidence, inspection failure, revision mismatch, unknown dependency remainder, partial delivery and denominator deletion.
3. Extend to at least 15 tiny cases and 30 total cases, then hold out complete disruption families.
4. Consume owner-native outputs from #6773, #5762 and #3683 by exact version/digest.
5. Reproduce the strongest matched port-energy, shipyard and service-chain baselines before making any performance claim.

A passing fixture is evidence of this model's arithmetic and declared topology only. It is not a physical or industry qualification.
