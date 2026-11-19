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

- Symtropy [#3252 Earth-to-orbit chain](https://github.com/Luminous-Dynamics/symtropy/pull/3252) now exposes per-cargo `CargoDeliveryV1` records with cargo identity, mass, target orbit, destination, terminal node and arrival epoch. Its exact branch head is pinned in this fixture. It remains a draft domain contract without a canonical serialized/signed receipt envelope.
- Symthaea [#12903 orbital-capture-handoff](https://github.com/Luminous-Dynamics/symthaea/pull/12903) qualifies a synthetic capture-handoff envelope, but its current report does not yet expose a joined service-client identity and prepared fluid-interface lineage as an authenticated artifact.
- Symtropy [#6182 orbital-fluid-transfer](https://github.com/Luminous-Dynamics/symtropy/pull/6182) validates synthetic fluid accounting and receiver-state conditions, but its current scenario/report does not yet carry the same receiver/client and prepared interface ID/revision as the capture handoff.

Symtropy [#6168 reusable-launch-production](https://github.com/Luminous-Dynamics/symtropy/pull/6168) remains an important separate fleet/manufacturing/maintenance owner; it is not treated as proof of a particular payload's target-orbit delivery.

The v0.1 fixture pins those source commits to make version assumptions explicit. Its `synthetic://` evidence references are adapter-contract test values, not actual source receipt digests. The current chain crate exposes a domain report but not a canonical signed serialized receipt. The evaluator checks the declared event contract and source-pin consistency; it does not fetch, authenticate or independently recompute source receipts.

Consequently, the complete positive control is a *hypothetical adapter-shape test*. It is not evidence that the three current PRs already integrate. A cargo aggregate without a per-cargo qualified delivery record, a proximity-only result, a storage-only cryogenic record or a receiver transfer without the matching capture/interface lineage correctly remains Indeterminate.

## Paradigm shifts and next qualification steps

1. **Service-first economics:** optimize cost, time, energy and risk per receiver-accepted service unit, not launch count or gross kilograms moved.
2. **Identity-carrying interfaces:** use stable mission, payload/client, interface, fluid-batch and configuration identities through every handoff.
3. **Capability composition:** require all mandatory service dependencies at the right evidence tier; never average away a missing hard prerequisite.
4. **Evidence as data plane:** each adapter should emit canonical event + exact upstream receipt digest + producer identity + authority scope + measurement/calibration context.
5. **Repairable infrastructure:** include inspectability, isolation, replacement, residual inventory and safe abort in lifecycle metrics.
6. **Independent verification:** the composition oracle remains separate from specialist models, receipt producers and any future optimizer.

## Claim ceiling

A passing v0.1 means only that synthetic event traces conform to this software contract. It does not validate upstream receipt bytes, authenticate producers, establish physical payload delivery, orbit insertion, capture, sealing, transfer, metrology, sustained service, economics, hardware performance or flight qualification. No live command or actuator path exists.
