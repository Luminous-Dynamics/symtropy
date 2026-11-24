# EHD-PIV phase design metrics

Dependency-free Rust reference evaluator for temporal phase coverage and independent-cycle replication in periodic PIV acquisition.

## Why

A nominal sampling percentage hides whether a periodic acquisition observes many phases once or a smaller set of phases repeatedly. The June 2026 SVGP-KAN synthetic pulsed-impinging-jet paper reported that 11 coprime phases with one sample per phase (0.55%) performed worse than 20 non-coprime repeats at one phase (1.0%), while 81 phases with two samples per phase (8.1%) performed best in its tested configuration. The two-sample/phase observation is a source-specific result, not a universal threshold.

## Inputs and independence

Each observation contains a phase fraction in [0,1) and a dependence-cluster ID, usually a forcing-cycle ID. Several frames from the same cycle in the same phase bin count as several raw observations but only one independent cycle.

## Diagnostics

- occupied/empty phase bins and phase coverage;
- singleton and under-replicated bins;
- min/median independent-cycle replication;
- maximum cyclic gap including the wrap-around interval;
- normalized phase-occupancy entropy;
- total observations and distinct dependence clusters.

Thresholds are declared in the caller's config. No physical claim or universal minimum sample-per-phase rule is encoded.

## Scope

This V0 kernel evaluates a supplied phase schedule. It does not infer forcing frequency/phase, detect periodicity, plan new acquisition times or prove that different dependence-cluster IDs are statistically independent. The independence graph and cluster IDs must come from a separately qualified experiment design.

The two-independent-cycle threshold is configurable and source-inspired only; it is not hard-coded as a universal condition.

## Run

    cargo fmt --manifest-path tools/ehd-piv-phase-design/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-phase-design/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-phase-design/Cargo.toml --all-targets -- -D warnings
