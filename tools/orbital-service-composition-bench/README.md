# Orbital Service Composition Benchmark v0.1

Independent, Rust-first reference contract for [ORBITAL-SERVICE-CORPUS-001 #3258](https://github.com/Luminous-Dynamics/symtropy/issues/3258), composed with the Earth-to-orbit delivery, capture and fluid benchmark owners.

## What it tests

The benchmark asks whether three separate synthetic evidence claims can be joined without losing identity or acceptance boundaries:

`payload delivery to target orbit -> mechanical capture handoff -> receiver-accepted fluid transfer`

It requires the same subject identity across all stages, the exact destination node and target orbit, a matching interface ID and revision between capture and transfer, the matching fluid identity, and receiver-accepted mass meeting the declared service quantity.

Launch attempt is not payload delivery. Approach is not capture. Capture is not fluid connection. Storage-only evidence is not transfer. Gross/source mass moved is not receiver-accepted mass. Customer acknowledgement is not technical acceptance.

## Run

```sh
cargo fmt --manifest-path tools/orbital-service-composition-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-service-composition-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-service-composition-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path tools/orbital-service-composition-bench/Cargo.toml -- \
  fixture tools/orbital-service-composition-bench/fixtures/v0.1/cases.json \
  --source-commit "$(git rev-parse HEAD)"
```

## Frozen corpus

The v0.1 corpus contains 20 synthetic cases: one complete contract control, ten hard-negative controls and nine missing/insufficient-evidence controls. The expected dispositions are frozen in `cases.json`. The target is 100% expected-result agreement and zero operational-authority grants.

## Current integration gap

The three source contracts are deliberately separate:

- Symtropy [#3252 Earth-to-orbit chain](https://github.com/Luminous-Dynamics/symtropy/pull/3252) now exposes per-cargo `CargoDeliveryV1` records with cargo identity, mass, target orbit, destination, terminal node and arrival epoch. The source now has a file-backed CLI receipt (`luminous.earth-orbit-chain.receipt.v1`) that binds exact campaign input bytes and source commit. Its resource/orbit evidence inputs remain caller-supplied and unauthenticated in v0.1.
- Symthaea [#12903 orbital-capture-handoff](https://github.com/Luminous-Dynamics/symthaea/pull/12903) now emits per-case client ID, interface ID/revision, frame and epoch after validating client identity against its profile. The fixture uses synthetic IDs; producer identity and physical mating remain unauthenticated/unproven.
- Symtropy [#6182 orbital-fluid-transfer](https://github.com/Luminous-Dynamics/symtropy/pull/6182) now emits receiver ID, interface ID and revision on each transfer/storage report. The fluid receiver must match the client and interface accepted by capture; field equality does not prove a physical seal, compatible materials or actual transfer.

Symtropy [#6168 reusable-launch-production](https://github.com/Luminous-Dynamics/symtropy/pull/6168) remains an important separate fleet/manufacturing/maintenance owner; it is not treated as proof of a particular payload's target-orbit delivery.

The v0.1 fixture pins those exact source commits to make version assumptions explicit and tests both client-ID and receiver-ID joins; a missing receiver identity is Indeterminate and a mismatched receiver is Rejected. Each evidence envelope declares an artifact SHA-256, producer ID, authority scope, context of use and claim ceiling. The evaluator checks required field presence and digest shape, but the `synthetic://` references and digest values are deterministic placeholders—not actual source artifact references or hashes. The Earth-to-orbit owner now exposes a Serde-serializable domain report, but not a signed/authenticated receipt envelope. The evaluator does not fetch source bytes, hash them, verify signatures or authenticate producer identity.

The nominal positive control remains a *hypothetical adapter-shape test*. It is not evidence that the three current PRs already integrate their actual artifacts. The Earth-to-orbit report is serializable but not signed/authenticated; capture/fluid now emit the identity fields needed for joining but still use synthetic evidence and do not prove physical service. The validator checks declared fields, identities, source pins and status semantics; it does not fetch, authenticate or independently recompute source receipts.

## Industry baseline and better-than-SOTA test

The v0.1 contract is not a launch-vehicle performance comparison. The current public benchmarks set separate bars:

- **Reusable launch operations:** SpaceX's June 2026 EU prospectus reports 165 Falcon 9 launches in 2025, including 157 launches using flight-proven boosters; it reports 40 Falcon launches in Q1 2026, 39 of them on flight-proven boosters, and a company-stated mission success rate above 99% as of 31 March 2026. This is a company disclosure and an operational/cadence baseline, not a matched comparison to this software contract. Source: https://content.spacex.com/cms-assets/FINAL_Documents%20and%20Updates/SpaceX%20-%20EU%20Prospectus%20%28Approved%20by%20Bafin%29%20-%20June%205%2C%202026.pdf
- **ISAM maturity:** NASA's 2026 State of Play records 621 capability-area entries, 61 facility descriptions and 169 developer profiles. NASA's entry threshold requires existing hardware as a ground prototype or flight demonstration; a concept presentation alone is insufficient. Inventory counts are not a quality leaderboard. Sources: https://www.nasa.gov/isam/ and https://www.nasa.gov/humans-in-space/commercial-space/nasas-isam-state-of-play-the-industry-standard-for-space-servicing-assembly-and-manufacturing/
- **Automated structural assembly:** on 5 November 2026 DARPA reported the Caltech NOM4D/ASTRA payload had assembled a truss in orbit using a three-axis gantry, composite members, custom joints and a reflective mesh. The hundreds-of-metres/kilometres scale is a future goal, not a demonstrated result. Source: https://www.darpa.mil/news/2026/nom4d-program-demonstrates-automated-in-space-assembly
- **Sustainable operations:** ESA's 2026 space environment report counts more than 300 launches and more than 4,000 payloads placed in orbit during 2025, and highlights debris growth, end-of-life removal and standardised servicing/removal interfaces. Sources: https://www.esa.int/articles/esa-space-environment-report-2026 and https://www.esa.int/media/payloads-and-objects-2026-space-environment-report

### What would count as superiority

Do not claim to outperform SpaceX, NASA, DARPA or a solver suite on one aggregate score. Demonstrate superiority on aligned, independently checked subproblems:

1. **Acceptance correctness:** 100% agreement with the independent oracle on bounded exact cases; zero false acceptance across all frozen critical negative controls; report false rejection and indeterminate rates separately.
2. **Identity and evidence continuity:** 100% exact payload/cargo, destination orbit, receiver, interface revision, fluid/batch and source-receipt lineage across every required handoff. Missing identity or envelope fields stays Indeterminate; malformed digests or conflicting claims are Rejected. A structurally complete envelope still does not authenticate its producer.
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


## Compose actual source receipt files

The CLI now has a second mode for joining the JSON receipts emitted by the three source tools:

```sh
cargo run --locked --manifest-path tools/orbital-service-composition-bench/Cargo.toml -- \
  compose \
  tools/orbital-service-composition-bench/fixtures/v0.1/cases.json \
  earth-orbit-chain-receipt.json \
  orbital-capture-handoff-receipt.json \
  orbital-fluid-transfer-receipt.json \
  --source-commit "$(git rev-parse HEAD)"
```

The adapter requires the exact source receipt schemas and frozen commits, and joins:
- delivery: per-cargo ID/mass, target orbit, destination, arrival and accepted status;
- capture: client ID, interface ID/revision, frame, epoch and accepted-for-handoff status with no authority grant;
- fluid: receiver ID, the same interface ID/revision, fluid class/ID, phase, accepted receiver mass, temperature, pressure and metrology uncertainty.

The returned bundle contains SHA-256 digests of the exact composition fixture and all three receipt files, pinned source commits and the joined service record. The bundle command fails closed on schema/commit mismatch, receiver or interface mismatch, rejected status or insufficient accepted mass.

The receipt-bundle unit tests currently use synthetic receipt-shaped values. The adapter is ready to consume the three file outputs, but no cross-repository CI artifact ingestion has yet been claimed. A valid bundle remains synthetic software evidence because all three current source fixtures are synthetic and their upstream resource/orbit/capture/fluid evidence inputs are not hardware-qualified or authenticated.
