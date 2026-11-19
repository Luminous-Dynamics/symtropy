# Orbital Service Composition Benchmark v0.1

Independent, Rust-first reference contract for [ORBITAL-SERVICE-CORPUS-001 #3258](https://github.com/Luminous-Dynamics/symtropy/issues/3258), composed with the Earth-to-orbit delivery, capture and fluid benchmark owners.

## What it tests

The benchmark asks whether three separate synthetic evidence claims can be joined without losing identity or acceptance boundaries:

`payload delivery to target orbit -> mechanical capture handoff -> receiver-accepted fluid transfer`

It requires the same subject identity across all stages, a matching interface ID and revision between capture and transfer, the target orbit required by the mission profile, a matching fluid identity, and receiver-accepted mass meeting the declared service quantity.

Launch attempt is not payload delivery. Approach is not capture. Capture is not fluid connection. Storage-only evidence is not transfer. Gross/source mass moved is not receiver-accepted mass. Customer acknowledgement is not technical acceptance.

## Run

```sh
cargo fmt --manifest-path tools/orbital-service-composition-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-service-composition-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-service-composition-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path tools/orbital-service-composition-bench/Cargo.toml -- \
  tools/orbital-service-composition-bench/fixtures/v0.1/cases.json \
  --source-commit "$(git rev-parse HEAD)"
```

## Frozen corpus

The v0.1 corpus contains 14 synthetic cases: one complete contract control, seven hard-negative controls and six missing/insufficient-evidence controls. The expected dispositions are frozen in `cases.json`. The target is 100% expected-result agreement and zero operational-authority grants.

## Current integration gap

The three source contracts are deliberately separate:

- Symtropy [#3252 Earth-to-orbit chain](https://github.com/Luminous-Dynamics/symtropy/pull/3252) now exposes per-cargo `CargoDeliveryV1` records with cargo identity, mass, target orbit, destination, terminal node and arrival epoch. Its exact branch head is pinned in this fixture. Its per-cargo domain report is now Serde-serializable, but it is not yet a signed/authenticated receipt envelope.
- Symthaea [#12903 orbital-capture-handoff](https://github.com/Luminous-Dynamics/symthaea/pull/12903) qualifies a synthetic capture-handoff envelope, but its current report does not yet expose a joined service-client identity and prepared fluid-interface lineage as an authenticated artifact.
- Symtropy [#6182 orbital-fluid-transfer](https://github.com/Luminous-Dynamics/symtropy/pull/6182) validates synthetic fluid accounting and receiver-state conditions, but its current scenario/report does not yet carry the same receiver/client and prepared interface ID/revision as the capture handoff.

Symtropy [#6168 reusable-launch-production](https://github.com/Luminous-Dynamics/symtropy/pull/6168) remains an important separate fleet/manufacturing/maintenance owner; it is not treated as proof of a particular payload's target-orbit delivery.

The v0.1 fixture pins those source commits to make version assumptions explicit. Its `synthetic://` evidence references are adapter-contract test values, not actual source receipt digests. The current chain crate exposes a domain report but not a canonical signed serialized receipt. The evaluator checks the declared event contract and source-pin consistency; it does not fetch, authenticate or independently recompute source receipts.

Consequently, the complete positive control is a *hypothetical adapter-shape test*. It is not evidence that the three current PRs already integrate. A cargo aggregate without a per-cargo qualified delivery record, a proximity-only result, a storage-only cryogenic record or a receiver transfer without the matching capture/interface lineage correctly remains Indeterminate.

## Industry baseline and better-than-SOTA test

The v0.1 contract is not a launch-vehicle performance comparison. The current public benchmarks set separate bars:

- **Reusable launch operations:** SpaceX's June 2026 EU prospectus reports 165 Falcon 9 launches in 2025, including 157 launches using flight-proven boosters; it reports 40 Falcon launches in Q1 2026, 39 of them on flight-proven boosters, and a company-stated mission success rate above 99% as of 31 March 2026. This is a company disclosure and an operational/cadence baseline, not a matched comparison to this software contract. Source: https://content.spacex.com/cms-assets/FINAL_Documents%20and%20Updates/SpaceX%20-%20EU%20Prospectus%20%28Approved%20by%20Bafin%29%20-%20June%205%2C%202026.pdf
- **ISAM maturity:** NASA's 2026 State of Play records 621 capability-area entries, 61 facility descriptions and 169 developer profiles. NASA's entry threshold requires existing hardware as a ground prototype or flight demonstration; a concept presentation alone is insufficient. Inventory counts are not a quality leaderboard. Sources: https://www.nasa.gov/isam/ and https://www.nasa.gov/humans-in-space/commercial-space/nasas-isam-state-of-play-the-industry-standard-for-space-servicing-assembly-and-manufacturing/
- **Automated structural assembly:** on 5 November 2026 DARPA reported the Caltech NOM4D/ASTRA payload had assembled a truss in orbit using a three-axis gantry, composite members, custom joints and a reflective mesh. The hundreds-of-metres/kilometres scale is a future goal, not a demonstrated result. Source: https://www.darpa.mil/news/2026/nom4d-program-demonstrates-automated-in-space-assembly
- **Sustainable operations:** ESA's 2026 space environment report counts more than 300 launches and more than 4,000 payloads placed in orbit during 2025, and highlights debris growth, end-of-life removal and standardised servicing/removal interfaces. Sources: https://www.esa.int/articles/esa-space-environment-report-2026 and https://www.esa.int/media/payloads-and-objects-2026-space-environment-report

### What would count as superiority

Do not claim to outperform SpaceX, NASA, DARPA or a solver suite on one aggregate score. Demonstrate superiority on aligned, independently checked subproblems:

1. **Acceptance correctness:** 100% agreement with the independent oracle on bounded exact cases; zero false acceptance across all frozen critical negative controls; report false rejection and indeterminate rates separately.
2. **Identity and evidence continuity:** 100% exact payload/cargo, destination orbit, receiver, interface revision, fluid/batch and source-receipt lineage across every required handoff. Missing identity or an unauthenticated receipt stays Indeterminate.
3. **Planning quality:** on small instances, match exhaustive enumeration exactly. On larger cases, compare the same input against a transparent greedy baseline and MILP/CP-SAT or time-expanded flow. Publish optimality gap, wall time, memory, hardware and constraint feasibility under equal budgets.
4. **Service economics:** compare lifecycle cost, time, energy and unserved demand per receiver-accepted service unit—not cost per launch or gross kg. Keep safety, service quality, reliability and environmental limits as hard constraints, not weighted penalties.
5. **Resilience:** publish N-1 and common-cause fault outcomes, recovery time, stranded mass and lower-tail service. Do not invent failure probabilities; use measured distributions only when their source population and censoring are declared.
6. **Physical progression:** after synthetic qualification, run a ground demonstrator with calibrated metrology, an inert-fluid transfer loop, a mechanically captured interface and a modular truss test article. Each result is limited to its tested hardware/configuration/environment; it does not inherit orbital qualification.

**Proposed research target, not an achieved result:** seek at least a 10% lifecycle-cost-per-accepted-service improvement over the strongest matched baseline on a held-out campaign, while maintaining zero critical false accepts and no degradation of hard service/reliability constraints. Do not use that target until the baseline, corpus, resource budget and acceptance criteria are frozen.

## Paradigm shifts and next qualification steps

1. **Service-first economics:** optimize cost, time, energy and risk per receiver-accepted service unit, not launch count or gross kilograms moved.
2. **Identity-carrying interfaces:** use stable mission, payload/client, interface, fluid-batch and configuration identities through every handoff.
3. **Capability composition:** require all mandatory service dependencies at the right evidence tier; never average away a missing hard prerequisite.
4. **Evidence as data plane:** each adapter should emit canonical event + exact upstream receipt digest + producer identity + authority scope + measurement/calibration context.
5. **Repairable infrastructure:** include inspectability, isolation, replacement, residual inventory and safe abort in lifecycle metrics.
6. **Independent verification:** the composition oracle remains separate from specialist models, receipt producers and any future optimizer.

## Claim ceiling

A passing v0.1 means only that synthetic event traces conform to this software contract. It does not validate upstream receipt bytes, authenticate producers, establish physical payload delivery, orbit insertion, capture, sealing, transfer, metrology, sustained service, economics, hardware performance or flight qualification. No live command or actuator path exists.
