# Orbital eccentric passive-terminal evaluator v0.1

A synthetic challenger for the algebraic terminal passive-safety conditions in Appendix A (Eqs. A1-A7) of Stephenson & Schaub, Two Approaches to Autonomous Spacecraft Inspection: Shielded Reinforcement Learning versus Probabilistic Roadmaps, Journal of the Astronautical Sciences (2026-09-28), https://doi.org/10.1007/s40295-026-00631-w.

This package does not reproduce the relative-element expansion or compute the interval bounds. It consumes outward-enclosed values from a separately qualified provider and checks the theorem's branch and margin logic.

## Mathematical contract

The source paper's first-order eccentric relative-motion model provides an along-track expression and lower/upper bounds y_lower/y_upper. Its keep-out ellipsoid has along-track support threshold rho_y. The two inequalities are:

    g_low  = delta_a * (y_lower - rho_y) <= 0
    g_high = delta_a * (y_upper + rho_y) <= 0

This evaluator consumes:
- delta_a = [delta_a_lower, delta_a_upper];
- a lower enclosure y_lower for the source expression's lower bound;
- an upper enclosure y_upper for the source expression's upper bound;
- rho_y_upper, a conservative upper bound on the ellipsoid's along-track support threshold;
- exact terminal action, target, keep-out, transform, frame, time-basis and dynamics-profile scope.

All integer position-like values use the same declared scaled unit, carried by the upstream provider. The evaluator uses signs and strict comparisons; it does not infer scale conversions or units.

For a strictly negative delta_a interval, the secular-outbound branch requires:

    y_lower > rho_y_upper

For a strictly positive delta_a interval, it requires:

    y_upper < -rho_y_upper

Equality is not a strict clearance result. If delta_a touches or crosses zero without being exactly [0,0], the branch is indeterminate; the evaluator will not blend or average sign branches.

## Periodicity is not clearance

The paper's exact delta_a = 0 case gives periodic relative motion. This evaluator deliberately reports PERIODIC_RELATIVE_MOTION_ONLY until a separate complete-period keep-out witness is supplied. It reports PERIODIC_PASSIVE_SAFE only when that witness:
- begins exactly at the terminal action epoch;
- binds the identical action, target, keep-out geometry, transform, frame, time basis and dynamics profile;
- covers at least one complete relative period;
- has complete time coverage;
- was classified COMPLETE_CONDITIONAL_CLEAR by the independent horizon composer.

This is a conservative evidence composition contract. The horizon composer itself remains conditional on its supplied per-segment bounds. It does not turn periodicity into a physical proof.

## Supported model and limitations

The first lane is restricted to the paper's first-order eccentric relative-motion chart and its assumptions. The provider must establish valid relative-element and y-bound enclosures in the declared frame, and the keep-out support bound must correspond to the exact protected-object geometry.

Out-of-model force effects such as J2, drag, SRP, target maneuver/attitude changes, actuator input or unmodelled frame transitions remain OUT_OF_SCOPE or INDETERMINATE unless another validated enclosure covers them. The evaluator does not derive y_lower/y_upper, rho_y, validate positive definiteness of K, construct an orbital propagator, authenticate source bytes or prove that a bound provider is physically valid.

A passing synthetic case establishes only the declared interval-algebra contract. The receipt hard-codes claim_ready=false, source_bytes_authenticated=false, authority_granted=false, and physical_validation=NOT_ASSESSED.

## Frozen corpus and independent oracle

The CSV fixture contains 38 positive, negative and boundary cases. The Rust evaluator and independent standard-library Python oracle must agree on the complete JSON receipt. The oracle uses its own parser and branch evaluator; it does not import or call the Rust implementation.

## Reproduce

    cargo fmt --manifest-path tools/orbital-passive-safety-terminal-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-passive-safety-terminal-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-passive-safety-terminal-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/orbital-passive-safety-terminal-bench/fixtures/passive-terminal-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/orbital-passive-safety-terminal-bench/Cargo.toml --bin orbital-passive-safety-terminal-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/orbital-passive-safety-terminal-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Ownership

- #7687 owns the first-order eccentric terminal passive-safety algebra challenger.
- #7573/#7575 own the typed finite-horizon and safe-continuation claim envelope.
- #7659/#7683/#7685 own one-segment chord clearance and complete piecewise horizon composition.
- #6801/#6820/#6876 own validated trajectory-model remainder, polynomial enclosure and first-contact isolation where applicable.
- Mycelix source custody/authentication remains separate.

No operational safety claim, orbital dynamics validation, flight readiness or maneuver authority follows from this synthetic package.
