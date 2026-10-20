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

The suite contains 19 unit tests plus an independent integration oracle with 9,225 closed-form comparisons over synchronized parallel paths and exact boundary tests. This oracle is deliberately separate from the implementation's closest-approach minimizer. The tests are authored but have not been executed in the current authoring environment.

## Strict exclusions

This crate does not ingest live surveillance, filed flight plans, active NOTAMs, weather feeds or real-time restrictions. It does not perform geodetic/Earth-frame transformations, enforce aircraft acceleration/turn/climb envelopes, validate vehicle performance, provide traffic advisories, issue clearances, authorize dispatch, control a vehicle, or qualify real-world separation minima.

Do not connect it to operational ATC/UTM, flight-control, or dispatch systems. Real-world use would require independent mathematical verification, qualified aviation review, jurisdiction-specific authoritative data and standards, validated sensing/track uncertainty, system-level safety assurance and all applicable operational approvals.
