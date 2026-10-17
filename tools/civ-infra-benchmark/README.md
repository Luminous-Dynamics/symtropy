# CIV-INFRA synthetic corridor benchmark

This is a small, standalone Rust reference harness for the first CIV-INFRA synthetic corridor case. It deliberately lives in its own Cargo workspace, outside Symtropy's main workspace, so that the bounded experiment does not pull in or disturb the full Bevy application dependency graph.

## Run

From the repository root:

    cargo fmt --manifest-path tools/civ-infra-benchmark/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/civ-infra-benchmark/Cargo.toml
    cargo run --locked --manifest-path tools/civ-infra-benchmark/Cargo.toml -- tools/civ-infra-benchmark/fixtures/corridor-v0.1.json

The CLI reads the exact fixture bytes, validates the schema and declared metric coordinate frame, calculates SHA-256 over the bytes, derives maximum longitudinal grade from station/elevation points, integrates declared cut/fill cross-section areas using the trapezoidal rule, and emits route length, grade, cut, fill, and total-earthworks metrics in the report. It independently recomputes the principal findings and schedule through separate reference helpers; numeric metrics are checked against an explicit tolerance. Invalid input or verifier disagreement returns a non-zero exit code.

## Current synthetic fixture

The frozen fixture contains three intentionally different candidate alignments. The report emits derived length, maximum absolute grade, cut volume, fill volume and total earthworks, each normalized to a stable nine-decimal precision for deterministic comparison. Result dispositions distinguish physical constraint infeasibility, inconsistency between declared and derived evidence, unknown/review-required cases, and feasibility for the explicitly modelled synthetic checks.

| Candidate | Length | Derived maximum grade | Cut | Fill | Total earthworks | Synthetic disposition |
|---|---:|---:|---:|---:|---:|---|
| A-shortest | 880 m | 8.0% | 3,520 m³ | 1,760 m³ | 5,280 m³ | Infeasible: grade, earthworks and drainage limits |
| B-low-earthworks | 980 m | 4.2% | 1,470 m³ | 980 m³ | 2,450 m³ | Infeasible: power-duct clearance |
| C-reference | 1,080 m | 3.5% | 2,160 m³ | 1,620 m³ | 3,780 m³ | Feasible for explicitly modelled synthetic checks |

The cross-section cut/fill areas, profile, utility locations, flow/capacity values and constraints are invented fixtures. The grade and volumes are calculated from those inputs; they are not derived from surveyed terrain or a full corridor solid. C is not a real-world qualified road design.

It also includes one deliberately overcommitted excavator schedule. The reference checker should reject that schedule, while a test moves one work package to an adjacent half-open time window and checks that the false overlap disappears.

## Claim ceiling and limits

- All geometry and quantities are invented synthetic test values, not a real site or design recommendation.
- V0 supports only an explicitly declared local metric coordinate frame; it rejects other units instead of attempting an implicit transform.
- The utility envelope is an axis-aligned coarse proxy. It is not a CAD solid-intersection engine or a substitute for a qualified utility survey.
- Utility evidence distinguishes bounded unknown (a conservative station/lateral envelope exists but depth or other important properties remain unknown) from unbounded unknown (no useful location envelope exists). Unbounded unknown utilities block clearance claims for every candidate; absence of geometry is never treated as clearance.
- Maximum longitudinal grade is derived from the candidate's piecewise-linear station/elevation profile and checked against the declared grade. The declaration is retained as a consistency check, not trusted as the result.
- Cut and fill volumes are derived by trapezoidal integration of the supplied cross-section cut/fill areas. Those areas themselves are synthetic inputs; they are not derived from a terrain mesh, surveyed ground surface or designed corridor solids.
- Drainage demand/capacity values are declared fixture attributes and compared directly; V0 does not derive flow capacity from a hydraulic solver.
- The reference checker is a separate recomputation path in the same codebase, not an independently maintained implementation or third-party audit. It compares independently calculated metrics within a stated numerical tolerance.
- This package does not generate road geometry, solve an alignment, qualify pavement/structures/geotechnical design, produce IFC/InfraGML, or authorize procurement/construction.
- No benchmark target should be reported as passed until this exact source is formatted, compiled, and the tests run; a queued or missing CI status is not evidence of a pass.

Implementation follow-up: https://github.com/Luminous-Dynamics/symtropy/issues/1732  
Benchmark specification: https://github.com/Luminous-Dynamics/symtropy/pull/1733  
Engineering integration parent: https://github.com/Luminous-Dynamics/symthaea/issues/7697
