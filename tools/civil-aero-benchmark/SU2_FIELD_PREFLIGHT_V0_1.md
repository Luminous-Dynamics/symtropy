# SU2 dry-run field preflight v0.1

## Why this gate exists

SU2's troubleshooting guidance says a dry run with \`SU2_CFD -d case.cfg\` prints the available volume and screen/history output fields for the exact solver and configuration. The same guidance warns that an unknown \`CONV_FIELD\` is ignored; if the requested convergence field is not recognized, the solver may disable its convergence check.

Do not infer field compatibility from a generic config template, an old SU2 version, or the existence of a similarly named CSV column. Check the exact executable, solver family, configuration and mesh before the production solve.

## Procedure

1. Build SU2 from a pinned source commit and record its build/toolchain details and executable digest.
2. Stage the exact production config and mesh into a fresh SU2 dry-run work directory.
3. Invoke the pinned executable with exactly \`-d CONFIG_PATH\` through \`external_solver_run\`, using solver ID \`SU2_CFD_DRY_RUN\`. This creates a process-only receipt; there are intentionally no expected physics outputs.
4. Run \`su2_field_preflight_audit\` against that run directory. It verifies the dry-run receipt, the exact argv, staged config/mesh digests, native mesh structure, explicit \`SOLVER\`, \`CONV_FIELD\` and non-empty \`HISTORY_OUTPUT\`, and the available screen/history field inventory.
5. Require every field needed by the history importer and residual/monitor screen. If any field is missing or \`CONV_FIELD\` is not in the exact dry-run inventory, stop and fix the configuration before executing CFD.
6. After the production run, independently import the actual history output, screen residual/monitor behavior, bind config/geometry/mesh/output lineage, and then perform grid convergence and V&V.

## Example

\`\`\`sh
SU2_SHA256="$(sha256sum ./SU2_CFD | awk '{print $1}')"

cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin external_solver_run -- \\
  --run-id dpw8-v1a-periodic-l1-a136-dry-run \\
  --solver-id SU2_CFD_DRY_RUN \\
  --version-claim "SU2 pinned source build; cite exact build record" \\
  --executable ./SU2_CFD \\
  --expected-executable-sha256 "$SU2_SHA256" \\
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136-dry-run \\
  --input ./config.cfg=config.cfg \\
  --input ./mesh.su2=mesh.su2 \\
  --timeout-seconds 300 \\
  --max-log-bytes 67108864 \\
  -- -d config.cfg

cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin su2_field_preflight_audit -- \\
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136-dry-run \\
  --config config.cfg \\
  --mesh mesh.su2 \\
  --require-field RMS_DENSITY \\
  --require-field RMS_MOMENTUM-X \\
  --require-field DRAG \\
  --require-field LIFT
\`\`\`

The field inventory is taken from the solver's dry-run output, not hard-coded from documentation. Keep the preflight run directory immutable and preserve its runner receipt.

## Evidence ceiling

The preflight proves that the exact executable/configuration dry-run advertised the required fields and that the configured convergence field was recognized in that inventory. It does not prove that a production solve wrote the fields, that the values are correctly normalized, that the iterative solution converged, that the mesh is appropriate, that the solution is grid-independent, or that the model agrees with experiment.

The production-run history importer and convergence screen remain mandatory. Grid-convergence and ASME V&V 20-style validation are separate gates.
