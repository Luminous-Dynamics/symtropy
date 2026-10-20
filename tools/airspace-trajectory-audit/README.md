# Synthetic 4D Airspace Trajectory Audit

This isolated, dependency-free Rust crate is a research prototype for **pairwise, time-dependent trajectory conflict screening**. It operates only on synthetic/offline trajectory data and explicit scenario-supplied separation criteria.

It is intentionally a separate Cargo workspace so it can be checked without enabling Symtropy's large Bevy/physics feature stack.

## Model

- Each trajectory is a time-ordered sequence of local Cartesian x/y/z points in metres, with UTC Unix-millisecond timestamps.
- Every trajectory names its coordinate reference frame and vertical datum. Pairwise comparisons reject mismatches rather than converting with unknown transforms.
- Adjacent points are joined with piecewise-linear paths. The model does not imply a vehicle can physically execute the path.
- Each trajectory carries fixed horizontal and vertical uncertainty bounds. The effective conflict bounds add both trajectories' uncertainty bounds to the explicitly supplied criteria.
- Criteria include an explicit jurisdiction/scenario label, source reference, SHA-256 syntax, revision and validity interval. This code does not authenticate the source or define operational minimum separation.
- The algorithm clips overlapping path segments to the criteria-valid interval, finds the vertical-compatible time subinterval, then minimizes horizontal separation over that subinterval.
- A detected conflict takes precedence over partial criteria coverage. If no conflict is found while part of the temporal overlap lacks valid criteria, the result is not assessable.
- Outputs are deterministic for a fixed input and canonicalize trajectory ordering. Reports make the covered interval and source-authentication limitation explicit.

## Dispositions

- ConflictDetected: at least one possible conflict lies within a criteria-valid interval.
- NoTemporalOverlap: the declared trajectory windows do not overlap in time.
- NoConflictDetectedWithinEvaluatedWindow: no conflict was detected, and the criteria cover the full temporal overlap of the supplied paths.
- NotAssessable: the criteria do not cover the full overlap and no conflict was found in the assessed portion.

A no-conflict result is bounded to these supplied paths, linear interpolation model, coordinate frame, uncertainty bounds, and criteria. It is **not** a claim that a real airspace conflict cannot occur.

## Checks

Run these commands in an environment with Rust/Cargo installed:

- cargo test --manifest-path tools/airspace-trajectory-audit/Cargo.toml
- cargo clippy --manifest-path tools/airspace-trajectory-audit/Cargo.toml --all-targets -- -D warnings
- cargo fmt --manifest-path tools/airspace-trajectory-audit/Cargo.toml -- --check
- cargo run --release --manifest-path tools/airspace-trajectory-audit/Cargo.toml --bin benchmark_broadphase

The suite contains 19 unit tests plus six integration tests: 9,225 closed-form comparisons over synchronized parallel paths; 72,600 integer-rational comparisons over sloped/crossing segments and inclusive radii; exact inclusive-boundary tests; and three metamorphic invariants (pair argument-order reversal, collinear waypoint subdivision, and uniform time-origin translation). The rational oracle compares squared-distance numerators with integer arithmetic instead of reusing the production floating-point minimizer. The authoring environment has no local Rust/Cargo toolchain; use the dedicated CI job for exact-head execution.

## Conservative fleet broadphase (research v0.1)

`broadphase_candidate_pairs` generates a deterministic sweep-and-prune candidate list over expanded trajectory AABBs. Candidate indices refer to the original input slice; every returned pair must still go through `audit_trajectory_pair`.

The current implementation deliberately expands each trajectory box by the declared base separation threshold plus that trajectory's uncertainty on each side. This double-counts the base threshold across a pair, increasing false positives rather than risking a missed candidate. The computed margin is rounded upward and bounds are expanded outward by one representable float step. Non-finite bounds fail closed. The input fleet must use one shared frame and vertical datum, and the supplied criteria must cover the full fleet time span; otherwise the function rejects the call so missing time coverage cannot be pruned into a no-conflict claim.

The exact-head Linux CI benchmark on 2026-10-20 measured candidate-generation p50/p95 of 1/21 µs (10 trajectories), 27/39 µs (100), 230/338 µs (1,000), and 3.017/4.123 ms (10,000). At 10,000, 5,000 candidates remained from 49,995,000 possible pairs (99.9900% reduction) on the fixed paired-path fixture. Host hardware was not pinned; these are candidate-generation-only measurements, not end-to-end conflict-management performance or a comparison to operational systems.

A deterministic release-mode benchmark binary now emits CSV for 10, 100, 1,000 and 10,000 paths, using 11 repetitions per size and reporting p50/p95 runtime, candidate count, all-pairs count and reduction percentage. It asserts that all known paired-path conflicts are represented by the returned candidate set. The CI job runs this binary so the measurements land in the action log; latency is recorded rather than enforced against an ungrounded hardware-independent threshold.

CI includes a 96-trajectory synthetic fleet where broadphase candidates are checked against brute-force pairwise audits. The target is 100% candidate recall for every pairwise `ConflictDetected` result while pruning at least some pairs in this fixed fixture. This is only a bounded regression test; it is not a 10k-fleet performance result and does not constitute interval-arithmetic proof for arbitrary magnitudes. Later work must report candidate recall, false-positive ratio, pair counts, runtime and memory across fixed sizes (10, 100, 1,000, 10,000) on a pinned machine. A false-negative candidate is an unconditional failure; higher false positives are acceptable until performance is measured.

## Strict exclusions

This crate does not ingest live surveillance, filed flight plans, active NOTAMs, weather feeds or real-time restrictions. It does not perform geodetic/Earth-frame transformations, enforce aircraft acceleration/turn/climb envelopes, validate vehicle performance, provide traffic advisories, issue clearances, authorize dispatch, control a vehicle, or qualify real-world separation minima.

Do not connect it to operational ATC/UTM, flight-control, or dispatch systems. Real-world use would require independent mathematical verification, qualified aviation review, jurisdiction-specific authoritative data and standards, validated sensing/track uncertainty, system-level safety assurance and all applicable operational approvals.
