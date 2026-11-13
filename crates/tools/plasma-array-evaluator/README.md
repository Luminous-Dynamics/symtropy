# plasma-array-evaluator

A standalone, dependency-light Rust evaluator for the frozen synthetic scenario set in the Symtropy plasma-array scale-out benchmark.

## Run

From the Symtropy repository root:

```sh
cargo test --manifest-path crates/tools/plasma-array-evaluator/Cargo.toml
cargo run --manifest-path crates/tools/plasma-array-evaluator/Cargo.toml -- benchmarks/plasma/plasma-array-scaleout-v0.json
```

The package has its own Cargo workspace boundary and does not join the root workspace. It uses Serde only for JSON input/output.

## Scope

The evaluator checks manifest identity, synthetic-only evidence, unique cell/scenario IDs, stale configurations, per-cell service reconciliation, active-cell sample CV, linear scale-out efficiency, explicit cross-talk coverage, failure isolation, attempt denominators and complete-system energy/gas boundaries. It independently checks frozen expected scale-out/CV/service-retention metrics and compares computed terminal dispositions with each frozen expected disposition.

All values in the fixture are artificial. A successful audit means only that the benchmark contract and arithmetic are internally consistent. It is not a plasma solver, physical observation, device-performance prediction, safety case or authorization to operate hardware.

## Validation status

Rust tests are authored. Run them with a supported Rust toolchain and report exact-head CI separately. No result should be inferred from branch creation or queued/skipped workflows.


## Fail-closed interpretation

- A multi-cell scenario with `cross_talk.coverage_status = NOT_MEASURED` is BLOCKED. An empty edge list is not evidence of zero cross-talk.
- A single-cell scenario must explicitly declare `NOT_APPLICABLE`.
- A `COMPLETE` multi-cell cross-talk assessment requires a sampling-plan ID and identical required/observed neighbor-pair sets, with an edge record for each pair.
- The signed neighbor-output delta is thresholded by its absolute magnitude; both positive and negative coupling effects count.
- `expected_metrics` is parsed and checked. Numeric G_N and CV values are compared within the declared tolerance; `computed_from_cells` requires a derived CV; null CV requires an undefined result; faulted-service retention is independently compared.
- Grid energy per accepted service is emitted only when the declared full grid-to-service boundary and all required subsystem indicators, meter identities and gas accounting are complete. Blocked energy boundaries produce no scoreable energy-intensity value.
- The one-cell reference scenario must itself pass the same evaluator before it can normalize any array scenario.
- The versioned input structs reject unknown fields; update the schema deliberately when the contract changes.

The 12-case fixture currently has 1 SYNTHETIC_PASS, 4 BLOCKED and 7 REJECTED expected dispositions. The five-cell linear and 100-cell isolated-failure cases retain their synthetic service arithmetic but are BLOCKED for missing cross-talk coverage. This is intentional: array-scale output is not qualified while cross-cell coupling is unmeasured.

Twenty-one Rust tests are authored, including an independent formula oracle for N=5 and N=100. They have **not** been compiled or executed in this environment; no CI result is claimed.
