# Accessible Home Task Bench

A standalone Rust tool for checking synthetic home task chains against explicit requirement evidence.

## Why this exists

A plan can list useful features and still fail during a daily routine. This tool turns a declared chain of steps into an inspectable assessment that distinguishes a known task-blocking barrier from missing evidence. It is a fail-closed evidence-triage harness, not a physical home simulator.

## Run from the repository root

    cargo fmt --manifest-path tools/accessible-home-task-bench/Cargo.toml -- --check
    cargo test --manifest-path tools/accessible-home-task-bench/Cargo.toml --lib
    cargo run --manifest-path tools/accessible-home-task-bench/Cargo.toml -- --input tools/accessible-home-task-bench/fixtures/accessibility-task-benchmark-v0.1.json --output /tmp/accessible-home-task-report.json

The CLI returns exit code 2 for malformed fixtures, missing evidence metadata, unsupported states or expectation mismatch. Without --output, it prints JSON to stdout.

## Assessments

- blocked_by_declared_barrier: at least one capability required by the declared task has an explicit known_barrier.
- not_evaluable: no task-blocking barrier is declared, but a required capability is unknown, proposed, needs_site_measurement or needs_professional_review.
- evidence_complete_for_fixture: every required capability in the synthetic task is labelled verified_for_declared_scope. That status is fixture-only, not independent verification.

A declared barrier takes precedence over separate unknowns. Optional or advisory capabilities remain in the report but do not determine task feasibility. Missing or malformed data fails closed.

## Receipt and provenance

The report includes the SHA-256 digest of the exact input bytes, rule version, scenario/task assessment, capability evidence states, evidence references and review roles. Identical input bytes and a pinned build should serialize identically. Expected status mismatches prevent a successful report.

## Scope boundary

The fixture includes nine scenarios for night routes, overlapping vision/mobility needs, alerts during sleep, bathroom transfers, deafblind visitor notification, network loss, fatigue-aware meal preparation, sensory/cognitive routines and combined vision/hearing communication.

Nothing in this tool certifies dimensions, compliance, a device, a real route, fire/egress, or any home. Use paid resident co-design and qualified local review; test real tasks in the actual space. It complements the Leptos planner and existing housing evidence work.

Related:
- https://github.com/Luminous-Dynamics/symtropy/issues/1695
- https://github.com/Luminous-Dynamics/symtropy/issues/1687
- https://github.com/Luminous-Dynamics/symtropy/pull/1688
- https://github.com/Luminous-Dynamics/symtropy/pull/1596
