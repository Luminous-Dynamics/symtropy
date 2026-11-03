# SU2 verified run bundle (schema v0.3; draft protocol)

## Purpose

Compose the already implemented SU2 dry-run preflight, boundary-marker audit, production process receipt, typed history import, declared convergence screen, staged-input/output lineage and resource observations into one deterministic per-run evidence object.

The bundle is deliberately a composition layer. It does not replace SU2, infer missing boundary physics, declare a universal convergence threshold, measure resources the runner does not capture, or promote a numerical run into experimental validation.

## Required source objects

Each production run must have:

1. One successful `SU2_CFD_DRY_RUN` receipt created by invoking the same executable bytes with exactly `-d CONFIG_PATH`.
2. A field-preflight report replayed from that dry-run directory. The v0.2 preflight records every `CONV_FIELD`, the explicit `WINDOW_CAUCHY_CRIT` mode, and (when enabled) all `CONV_WINDOW_FIELD` values and their window settings. See [`SU2_CONVERGENCE_PROTOCOL_V0_2.md`](./SU2_CONVERGENCE_PROTOCOL_V0_2.md).
3. A boundary-marker audit replayed from that same dry-run directory.
4. One successful `SU2_CFD` production receipt.
5. One raw-history import bound to the production receipt and exact history output bytes.
6. One declared residual/monitor screen bound to the same run, receipt and history digest.
7. One native SU2 termination classification replayed from the exact full production stdout/stderr and matched to the preflight's declared steady/time-window protocol.
8. For unsteady time-window runs, one per-time-step inner residual screen bound to the same preflight and production history. This draft supports residual-based `CONV_FIELD` values and requires contiguous `TIME_ITER`, `INNER_ITER`, explicit `CONV_RESIDUAL_MINVAL`, and complete configured window coverage. See [`SU2_UNSTEADY_INNER_SCREEN_V0_1.md`](./SU2_UNSTEADY_INNER_SCREEN_V0_1.md).
9. One production lineage binding for configuration, geometry, mesh, history and declared additional outputs.

**Unsteady endpoint mode is deliberately not admitted by run-bundle v0.3.** If `TIME_DOMAIN=YES` and `WINDOW_CAUCHY_CRIT=NO`, preflight can record the configuration, but this bundle refuses to promote it. A separate endpoint-specific evidence adapter is required; the per-time-step windowed inner screen is not a substitute.

The field preflight and boundary audit intentionally share one dry-run receipt. Running `SU2_CFD -d` twice for the same unchanged inputs would add cost without adding independent identity evidence. If a workflow uses separate dry runs, this v0.1 API does not silently merge them: a later schema can support multiple dry-run receipts explicitly.

## Mandatory identity invariants

The bundle builder replays the reports from the original run directories. It rejects unless:

- The dry-run receipt identifies `SU2_CFD_DRY_RUN` and the production receipt identifies `SU2_CFD`.
- Both process receipts are successful and independently byte-replayable.
- Preflight and boundary reports identify the same dry-run run ID and receipt digest.
- Dry-run and production executable SHA-256 values are identical.
- Dry-run configuration and mesh paths match the production input-binding paths.
- Dry-run configuration and mesh hashes match both the boundary audit and production lineage artifacts.
- Every history column consumed by the importer was both required and discovered by the preflight.
- The production history, convergence screen, native termination report and lineage all reference the exact production receipt and history/log bytes.
- Native termination is consistent with the preflight's time-domain/window mode and reports the relevant solver convergence completion. Scheduler signal/interruption evidence rejects admission even if a success banner is also present.
- Native steady convergence is represented by separately parsed per-field table rows. Native time-window convergence is represented by the single SU2 aggregate success banner scoped to the complete configured `CONV_WINDOW_FIELD` list; the report does not fabricate per-field native output rows from that banner. Independent post-run history screening remains per-field.
- For an unsteady time-window run, every saved time step in the required window coverage passes the explicit inner residual threshold; gaps and unsupported residual fields reject.
- Every declared convergence criterion passes. An empty, failed or insufficient criterion set cannot be promoted.
- Every active `CONV_FIELD` and `CONV_WINDOW_FIELD` is both imported from production history and covered by a passing independent post-run screen.
- Every time-window field has a terminal monitor-stability screen; a residual-only pass cannot stand in for a `TAVG_*` field.
- Every contiguous saved unsteady time step meets the explicit inner residual threshold; sparse `TIME_ITER` histories are rejected.
- The lineage binding identifies a valid production geometry digest.

The geometry is production-bound because the dry-run field and boundary checks need the configuration and mesh, but do not themselves authenticate the geometry file. Its identity is taken from the production run's staged-input receipt.

## Native termination and co-terminal time limits

SU2 v8.6.0's single-zone time driver evaluates `TimeConvergence`, `FinalTimeReached` and `MaxIterationsReached` independently. It can therefore print the window-success banner and, in the same `Solver Exit` section, a final-time or maximum-time-iterations message. Those messages are compatible: the configured window criteria were met, and a run limit was also reached. The classifier records this as `time_window_native_criteria_satisfied_with_coincident_limit` rather than incorrectly rejecting it as contradictory.

When no window-success banner is present, a final-time and/or time-iteration limit is a non-converged termination. Both limits may be reported together and are classified as `time_limits_reached_before_window_convergence`. Steady-mode success and iteration-limit messages remain mutually exclusive in the pinned driver and their coexistence is still rejected.

The signal guard remains stricter. SU2 v8.6.0's SIGTERM handler writes `Interrupt signal (N) received, saving files and exiting.` before the driver's `Solver Exit` summary. In unsteady mode, the driver can print the normal time-window message without exposing the separate interruption flag in that summary. Therefore the termination auditor inspects the entire receipt-bound stdout/stderr byte stream, not only the text after `Solver Exit`. Any well-formed signal-handler line produces an `interrupted` classification and prevents admission, even when a success banner is also present. A malformed line beginning with the signal-handler prefix is rejected as unclassified output.

Pinned SU2 v8.6.0 source references:
- [COutput.cpp: signal handler and time-window convergence monitor](https://github.com/su2code/SU2/blob/v8.6.0/SU2_CFD/src/output/COutput.cpp#L54-L60)
- [COutput.cpp: time-window start, Cauchy accumulation and max-delay rule](https://github.com/su2code/SU2/blob/v8.6.0/SU2_CFD/src/output/COutput.cpp#L1060-L1128)
- [CSinglezoneDriver.cpp: independent time-window and stop-limit messages](https://github.com/su2code/SU2/blob/v8.6.0/SU2_CFD/src/drivers/CSinglezoneDriver.cpp#L292-L309)
- [CSinglezoneDriver.cpp: steady exit and native per-field summary](https://github.com/su2code/SU2/blob/v8.6.0/SU2_CFD/src/drivers/CSinglezoneDriver.cpp#L278-L290)
- Source blob SHA-256/SHA-1 identifiers returned by the GitHub content API: `COutput.cpp 1101d0a903cba6c1a35b367ba81c2ccd547f519a`, `CSinglezoneDriver.cpp b1283979a72822cdc80dd2dcb5cffd9d02cefa8e`. These are blob identities, not a source build attestation.

## Resource observations

The current external-solver receipt records elapsed wall time in milliseconds. The bundle derives wall-clock seconds from that exact receipt and binds the observation to the receipt digest.

CPU core-hours, peak resident memory, GPU hours and energy joules are represented as typed `Unknown { reason }` observations. They are not encoded as zero, inferred from wall time, or copied from caller-entered scalars.

A measured resource value must eventually carry its value, unit, measurement method, source SHA-256 and (when characterized) standard uncertainty. Missing measurement uncertainty remains unknown; it is never treated as zero.

Consequently, this bundle can support a wall-time comparison, but cannot yet support a complete compute-cost, energy-efficiency or environmental-impact comparison.

## Canonical receipt and replay

The report includes:

- Dry-run and production run IDs and receipt SHA-256 values.
- Solver executable, configuration, geometry and mesh digests.
- Field-preflight and boundary-audit digests.
- Raw history, parsed history, independent convergence-screen, native termination, unsteady inner-screen and production-lineage digests.
- Typed resource observations and their canonical digest.
- A deterministic bundle SHA-256 over the full report fields and typed resource observations.

`verify_su2_verified_run_bundle` re-reads both run directories, replays all component auditors and recomputes the composite report. The report and receipts are unsigned integrity objects: they do not attest the host, compiler, linked libraries, build provenance, trusted execution environment or source authorship.

An independent Python oracle in `scripts/verify-su2-run-bundle-independent.py` separately checks the composed identity links, exact byte hashes, native termination/scheduler-signal guard, every saved unsteady time-step residual, and canonical resource/bundle digests in the synthetic grid-family fixture.

## Grid-family integration

Every `Su2GridFamilyMember` now carries the dry-run directory, field-preflight report and boundary-audit report in addition to its production lineage. Grid-family evaluation creates and replays the verified bundle for every member before deriving the terminal monitor quantity or computing GCI.

The grid-family report exports the preflight, boundary-audit and verified-run-bundle digests for each grid. A grid cannot borrow another member's dry-run receipt: the dry-run configuration and mesh must match that member's own production lineage.

## Negative controls

The synthetic fixture checks rejection of:

- A dry-run receipt from a different grid.
- A changed boundary-marker assignment.
- An omitted field from the preflight inventory.
- A changed mesh/domain or marker set through existing mesh-family invariants.
- A modified production history, convergence screen, native termination log or output through existing lineage replay.
- A scheduler interruption/signal line preceding an otherwise successful native window banner.
- An unsteady history with a missing time step or one inner residual above its configured threshold.
- Unsteady endpoint mode without a separate endpoint-specific admission screen.
- Any resource field represented as a measured value without a valid unit, method and source digest.

The fake executable, histories, meshes and GCI values are synthetic known-answer fixtures. They prove software contracts only; they are not SU2 CFD results.

## Evidence ceiling

`replayed_dry_run_multi_field_protocol_native_solver_termination_and_scheduler_signal_guard_and_every_saved_unsteady_time_step_inner_residual_screen_bound_to_production_history_and_inputs_unsigned_wall_time_only_other_resources_unknown_not_grid_independent_or_experimentally_validated`

This bundle proves that the declared evidence objects refer to one consistent run and can be replayed. It does not prove that every relevant equation/monitor was selected, that the selected thresholds are scientifically adequate, that a steady-state assumption is valid, that the grid is independent, that the turbulence/model-form assumptions are appropriate, or that predictions agree with experiment.

## Advancement criteria

The next increments are deliberately separate:

1. Add source/build provenance or signed attestations only when a trustworthy build/attestation mechanism exists.
2. Add measured CPU, peak-memory, GPU and energy telemetry with measurement provenance and uncertainty.
3. Add multiple preflight/boundary receipts only through a versioned schema that declares their roles and matching identities.
4. Bind the bundle directly into the multi-grid numerical-admission receipt, retaining raw residual encoding and monitor sets rather than caller-supplied scalar summaries.
5. Admit a pinned real SU2 build and fixed-geometry case. A real solver execution remains distinct from grid convergence and experimental validation.
