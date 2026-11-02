# SU2 verified run bundle v0.1

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
7. One production lineage binding for configuration, geometry, mesh, history and declared additional outputs.

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
- The production history, convergence screen and lineage all reference the exact production receipt and history bytes.
- Every declared convergence criterion passes. An empty, failed or insufficient criterion set cannot be promoted.
- Every active `CONV_FIELD` and `CONV_WINDOW_FIELD` is both imported from production history and covered by a passing independent post-run screen.
- Every time-window field has a terminal monitor-stability screen; a residual-only pass cannot stand in for a `TAVG_*` field.
- The lineage binding identifies a valid production geometry digest.

The geometry is production-bound because the dry-run field and boundary checks need the configuration and mesh, but do not themselves authenticate the geometry file. Its identity is taken from the production run's staged-input receipt.

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
- Raw history, parsed history, convergence-screen and production-lineage digests.
- Typed resource observations and their canonical digest.
- A deterministic bundle SHA-256 over the full report fields and typed resource observations.

`verify_su2_verified_run_bundle` re-reads both run directories, replays all component auditors and recomputes the composite report. The report and receipts are unsigned integrity objects: they do not attest the host, compiler, linked libraries, build provenance, trusted execution environment or source authorship.

An independent Python oracle in `scripts/verify-su2-run-bundle-independent.py` separately checks the composed identity links, byte hashes and canonical resource/bundle digests in the synthetic grid-family fixture.

## Grid-family integration

Every `Su2GridFamilyMember` now carries the dry-run directory, field-preflight report and boundary-audit report in addition to its production lineage. Grid-family evaluation creates and replays the verified bundle for every member before deriving the terminal monitor quantity or computing GCI.

The grid-family report exports the preflight, boundary-audit and verified-run-bundle digests for each grid. A grid cannot borrow another member's dry-run receipt: the dry-run configuration and mesh must match that member's own production lineage.

## Negative controls

The synthetic fixture checks rejection of:

- A dry-run receipt from a different grid.
- A changed boundary-marker assignment.
- An omitted field from the preflight inventory.
- A changed mesh/domain or marker set through existing mesh-family invariants.
- A modified production history, convergence screen or output through existing lineage replay.
- Any resource field represented as a measured value without a valid unit, method and source digest.

The fake executable, histories, meshes and GCI values are synthetic known-answer fixtures. They prove software contracts only; they are not SU2 CFD results.

## Evidence ceiling

`replayed_dry_run_multi_field_steady_and_time_window_protocol_and_boundary_audit_bound_to_production_history_screen_and_inputs_unsigned_wall_time_only_other_resources_unknown_not_grid_independent_or_experimentally_validated`

This bundle proves that the declared evidence objects refer to one consistent run and can be replayed. It does not prove that every relevant equation/monitor was selected, that the selected thresholds are scientifically adequate, that a steady-state assumption is valid, that the grid is independent, that the turbulence/model-form assumptions are appropriate, or that predictions agree with experiment.

## Advancement criteria

The next increments are deliberately separate:

1. Add source/build provenance or signed attestations only when a trustworthy build/attestation mechanism exists.
2. Add measured CPU, peak-memory, GPU and energy telemetry with measurement provenance and uncertainty.
3. Add multiple preflight/boundary receipts only through a versioned schema that declares their roles and matching identities.
4. Bind the bundle directly into the multi-grid numerical-admission receipt, retaining raw residual encoding and monitor sets rather than caller-supplied scalar summaries.
5. Admit a pinned real SU2 build and fixed-geometry case. A real solver execution remains distinct from grid convergence and experimental validation.
