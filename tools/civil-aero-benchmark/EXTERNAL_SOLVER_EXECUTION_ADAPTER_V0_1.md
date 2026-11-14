# External solver execution adapter v0.1

## Purpose and evidence ceiling

This adapter makes the existing Rust `external_solver` process runner usable from the command line. It is suitable for invoking a caller-built and caller-pinned SU2 executable, such as `SU2_CFD`, without shell interpolation. It is not an SU2-specific physics parser and does not establish convergence, grid independence, aerodynamic validity, or flight suitability.

Every invocation runs in a newly created directory. Inputs are copied into that directory only after their length and SHA-256 are measured and rechecked during staging. The executable is checked against a caller-supplied SHA-256 before launch and its actual digest is recorded in the receipt. The runner captures stdout/stderr under byte limits, enforces a wall timeout, records exit status, hashes expected outputs, and writes a receipt that can be re-read and independently replayed offline.

The receipt's evidence ceiling remains:

`process_receipt_only_caller_version_claim_unverified_no_convergence_or_physical_validation`

A version string is a caller claim. The executable digest identifies bytes, not the source revision, compiler, linked libraries, or build recipe. Local source paths are recorded as operator-supplied provenance labels, not authenticated source records.

## Build the solver

Build SU2 from a pinned source revision using a documented toolchain and configuration. Record the exact SU2 source commit, build options, compiler/toolchain, linked libraries, and final executable digest outside the run receipt. The caller must provide the expected executable digest to the adapter. A release label alone is insufficient.

## Run SU2

The config must refer to staged paths, e.g. `MESH_FILENAME=mesh.su2`. Stage the config and mesh explicitly; stage any additional geometry, restart, table, or boundary-condition files needed by the chosen SU2 case. Declare every output whose bytes are required for replay.

```sh
SU2_SHA256="$(sha256sum ./SU2_CFD | awk '{print $1}')"

cargo run --release --locked \
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \
  --bin external_solver_run -- \
  --run-id dpw8-v1a-periodic-l1-a136 \
  --solver-id SU2_CFD \
  --version-claim "SU2 source build; supply exact source/build record separately" \
  --executable ./SU2_CFD \
  --expected-executable-sha256 "$SU2_SHA256" \
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136 \
  --input ./config.cfg=config.cfg \
  --input ./mesh.su2=mesh.su2 \
  --output history.csv \
  --output forces_breakdown.dat \
  --output surface_flow.vtu \
  --timeout-seconds 7200 \
  --max-log-bytes 67108864 \
  -- -f config.cfg

cargo run --release --locked \
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \
  --bin external_solver_replay_audit -- \
  ./artifacts/dpw8-v1a-periodic-l1-a136
```

The `--` separator is mandatory: all following tokens are passed literally as solver arguments. There is no shell expansion. The runner clears the inherited environment. Supply only necessary environment entries via repeated `--env KEY=VALUE`; in particular, provide `PATH`, `LD_LIBRARY_PATH`, or thread/device controls explicitly when the executable needs them. Environment values are not written in plaintext into the environment manifest; only key names and value hashes are retained.

Use a new run directory for every grid, angle, solver build, boundary-condition variant, and retry. Never reuse stale output directories. The adapter does not infer SU2 output filenames from the config: explicitly list expected outputs.

## CI known-answer

`scripts/verify-external-solver-cli.sh` runs a tiny shell fixture through the exact same CLI, replays the receipt, then mutates a captured log and verifies that offline replay rejects it. The fixture is not SU2 and produces no aerodynamic evidence.

## Next integration

The next adapter increment is a parser for SU2's history/force outputs that produces typed quantities with declared sign conventions and units, followed by binding those output digests to the existing grid-convergence and uncertainty admission inputs. This CLI intentionally does not parse or promote solver results by itself.
