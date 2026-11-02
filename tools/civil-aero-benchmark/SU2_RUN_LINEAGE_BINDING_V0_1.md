# SU2 run-lineage binding v0.1

## Purpose

The run-lineage binding closes one important gap between the external process runner and CFD artifact handling. It proves, by deterministic digest linkage, that the same successful SU2 process receipt is referenced by:

- one typed SU2 history import, whose raw history bytes are bound to the runner receipt;
- one steady-history screen whose criteria, units, residual encoding, metrics and canonical digest are verified;
- three separately staged inputs: physics configuration, geometry and mesh;
- the exact expected history, force and field output bytes declared by the runner.

The output is a row-per-artifact CSV with a canonical SHA-256 digest. The Python CI oracle independently parses the process invocation manifest, re-hashes staged inputs and outputs, checks cross-document run/receipt/history/screen identities, and reconstructs the canonical digest.

## Evidence ceiling

The binding is **unsigned** and does not attest to the host, compiler, linked libraries, upstream source revision, executable build recipe, or trusted execution environment. The solver version remains a caller claim; the executable digest identifies bytes only.

It does not measure CPU core-hours, peak resident memory, GPU time or energy. The current process runner records elapsed wall time but does not provide those resource measurements. Missing resource data must stay unknown; do not fill it with zero or infer it from elapsed time.

It does not establish:
- all SU2 equations or all force/moment monitors were selected;
- conservation or physical plausibility;
- spatial grid convergence, temporal convergence or asymptotic range;
- agreement with experimental data or uncertainty coverage;
- design feasibility, safety, airworthiness or flight readiness.

The convergence screen is only as complete as the explicit caller-declared criteria. The steady classifier must not be applied to periodic/unsteady solutions. Those need an unsteady assessment based on time-step convergence and statistical confidence of time-averaged observables.

## Command example

After a successful run using the pinned external-solver runner, re-import and screen the history, then export the lineage receipt:

\`\`\`sh
cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin su2_convergence_screen -- \\
  --run-dir ./artifacts/naca0012-case-01 \\
  --history history.csv \\
  --iteration-column Iteration \\
  --header-column Iteration \\
  --header-column CL \\
  --header-column CD \\
  --header-column RMS_DENSITY \\
  --header-column RMS_MOMENTUM-X \\
  --field CL=dimensionless \\
  --field CD=dimensionless \\
  --field 'RMS_DENSITY=log10(residual)' \\
  --field 'RMS_MOMENTUM-X=log10(residual)' \\
  --residual-log10 'RMS_DENSITY|log10(residual)|6|-8|6|2' \\
  --residual-log10 'RMS_MOMENTUM-X|log10(residual)|6|-8|6|2' \\
  --monitor 'CL|dimensionless|6|0.0004|0.0003|0.02' \\
  --monitor 'CD|dimensionless|6|0.0004|0.0003|0.02' \\
  --config-input config.cfg \\
  --geometry-input geometry.dat \\
  --mesh-input mesh.su2 \\
  --additional-output forces_breakdown.dat \\
  --additional-output surface_flow.csv \\
  --lineage-output ./artifacts/naca0012-case-01/run-lineage.csv
\`\`\`

All values in the example are illustrative caller-selected criteria, not universal SU2 acceptance standards. The expected header and output paths must be adapted to the exact SU2 solver/version/configuration. Multi-zone cases and multiple mesh/configuration roles need a wider binding schema before being accepted.

## Next integration

The next composition layer is documented in [`SU2_VERIFIED_RUN_BUNDLE_V0_1.md`](./SU2_VERIFIED_RUN_BUNDLE_V0_1.md). It replays the field preflight and boundary audit from the same receipt-bound `SU2_CFD -d` run, then requires their executable/configuration/mesh identities to match the production receipt and staged inputs. The grid-family evaluator now requires this composed bundle per member.

The remaining numerical-admission bridge must consume raw history observations directly. It must preserve log10 residual encoding and must not infer a positive-linear value through implicit exponentiation, absolute values, or hidden transforms. Any explicit transform must preserve the original series, formula, units and provenance.

Resource telemetry is deliberately partial: elapsed wall time is derived from the process receipt; CPU time, peak memory, GPU time and energy remain unknown until bound to real measurements. A wall-time comparison must not be described as a complete compute-cost or energy-efficiency comparison. GCI/Richardson and experimental validation remain separate gates after the run bundle.
