# Aero WPT mechanical capture envelope v0

**Status:** finite set-map contract validator and evaluator over synthetic approach-to-post-contact relations. It is not a contact-mechanics solver, FEM model, landing controller, structural qualification, observability estimator, safety interlock or flight/power permit.

## Research objective

Represent passive docking geometry as a set-valued map from each declared approach-state hypothesis to every possible post-contact outcome. A V-shaped guide, funnel or kinematic landing feature may reduce the set of residual positions, but a passive guide is not a sensor and must not be treated as proof of unique pose, receiver identity or authorization.

```text
C: X_approach -> 2^(X_post x Outcome x DomainStatus)
```

The evaluator preserves all possible successor states and reports capture-only coverage, mixed outcomes and explicit out-of-domain/unknown possibilities. It never converts a multi-state residual set into one nominal landing point.

## Input contract

The TSV header is exact and has fifteen columns:

1. case_id
2. evidence_class
3. decision_time_us
4. approach_set_sha256
5. receiver_config_sha256
6. guide_geometry_sha256
7. frame_sha256
8. calibration_sha256
9. initial_approach_states (semicolon-separated IDs)
10. approach_state_id
11. post_state_id
12. post_pose_id
13. outcome: CAPTURED, NOT_CAPTURED, JAMMED, REBOUND, SLIP, OUT_OF_DOMAIN, UNKNOWN
14. transition_in_domain (true/false)
15. valid_until_us

Rows repeat identical case metadata. Every declared initial approach state must have at least one possible transition row. A post-state ID maps to exactly one pose-equivalence ID. Duplicate approach->post edges and conflicting pose mappings are rejected. Every row's evidence must remain valid at decision time; an expired relation invalidates the entire case rather than being silently removed.

Digests are syntax-checked as lowercase SHA-256 only; referenced content is not independently fetched or hashed by this v0 tool.

## Evaluation semantics

For each approach-state hypothesis:
- CAPTURED_ONLY: every declared possible transition is in-domain and has outcome CAPTURED. All post-pose IDs are retained.
- A uniform non-capture outcome is reported as NOT_CAPTURED_ONLY, JAMMED_ONLY, REBOUND_ONLY or SLIP_ONLY.
- Mixed capture/failure or multiple distinct non-capture outcomes are reported as MIXED_OUTCOMES.
- Any out-of-domain transition takes priority and forces ABSTAIN_OUT_OF_DOMAIN_POSSIBILITY.
- Any UNKNOWN outcome forces ABSTAIN_UNKNOWN_POSSIBILITY unless an out-of-domain possibility is already present.

At case level:
- all approach states CAPTURED_ONLY: CAPTURED_SET_MAP_ONLY;
- some but not all approach states CAPTURED_ONLY: MIXED_CAPTURE_ENVELOPE;
- no approach state has capture-only support: NO_UNIFORM_CAPTURE;
- any out-of-domain/unknown possibility: corresponding ABSTAIN disposition.

The residual pose set is reported even when it contains more than one pose. The tool never emits UNIQUE_POSE, receiver authorization or a power/flight permit.

## Frozen synthetic cases

- complete-capture: every approach state has only in-domain CAPTURED successors; one approach state has multiple residual post-pose possibilities, which must all remain visible.
- mixed-capture: one approach state captures and another has NOT_CAPTURED support.
- out-of-domain: one possible captured successor is marked outside the declared envelope; the whole case must abstain.
- jam-rebound: failure outcomes must remain explicit rather than being collapsed into capture.

## Required properties

- Every declared approach state has transition coverage.
- Every possible post-state is preserved.
- Conflicting post-state/pose identities are rejected.
- Any possible out-of-domain edge invalidates whole-case capture qualification.
- UNKNOWN cannot silently disappear.
- Row permutation preserves the result.
- Mechanical capture never establishes pose uniqueness or authorization.

## Run

```sh
cargo test --locked --manifest-path tools/aero-wpt-mechanical-capture/Cargo.toml
cargo run --locked --manifest-path tools/aero-wpt-mechanical-capture/Cargo.toml -- research/aero-wpt-mechanical-capture-v0/complete-capture.tsv
cargo run --locked --manifest-path tools/aero-wpt-mechanical-capture/Cargo.toml -- research/aero-wpt-mechanical-capture-v0/mixed-capture.tsv
cargo run --locked --manifest-path tools/aero-wpt-mechanical-capture/Cargo.toml -- research/aero-wpt-mechanical-capture-v0/out-of-domain.tsv
```

## Claim ceiling

A passing software test proves only finite relation consistency. It does not establish physical capture probability, contact forces, structural integrity, landing-gear compatibility, repeatability, electromagnetic performance, exposure/EMC compliance, aircraft safety, regulatory applicability or permission to energize or fly. Physical claims require an instrumented low-consequence bench, calibrated pose/contact measurement, a reviewed test protocol and independent replay.