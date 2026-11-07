# LOCO Robust Viability V0

Standalone, dependency-free Rust reference oracle for [LOCO-VIABILITY-001 / #4058](https://github.com/Luminous-Dynamics/symtropy/issues/4058).

## The question

A state can satisfy instantaneous safety constraints and still be unrecoverable. A foothold or contact action can look safe now while leaving the next support phase with no safe continuation.

V0 represents a locomotor abstraction as a finite nondeterministic transition system. Each state is marked instantaneously safe or unsafe; each action has a non-empty set of possible successors. The successor set is adversarial: every declared outcome must be safe.

## Semantics

For safe set K and action-successor relation T(s,a):

    Pre_R(X) = {s in K : exists a, T(s,a) is nonempty and T(s,a) subseteq X}

Finite-horizon viability:

    V_0 = K
    V_(h+1) = Pre_R(V_h)

Maximal robust controlled-invariant set:

    V_* = greatest fixed point of Pre_R

Robust recovery to target set R:

    Reach_0 = R intersect K
    Reach_(h+1) = Reach_h union Pre_R(Reach_h)

The recovery horizon is the smallest h for which a state enters Reach_h. A recovery action must send every declared successor to a state with a strictly smaller recovery horizon (or into the target).

## Implemented

- Typed state/action/successor records with stable string identities.
- Fail-closed validation of blank/duplicate identifiers, unknown source/successor states, duplicate successors, empty outcome sets, unsafe recovery targets and resource limits.
- Finite-horizon robust viability.
- Greatest-fixed-point robust controlled-invariant set.
- Robust reach-to-recovery with worst-case recovery horizon.
- All admissible actions per state, not just one chosen policy.
- Rejected-action counterexample successor witnesses.
- Separate classification for instantaneously unsafe, robustly viable and safe-but-not-viable states.
- Deterministic BTreeMap/BTreeSet outputs independent of input row order.
- Thirteen deterministic known-answer / adversarial tests, including brute-force subset enumeration on small graphs.

## Interpretation boundary

The graph is an abstraction supplied by the caller. V0 proves a property of that finite graph only. It does not generate physical transition sets, prove that a simulator has been overapproximated, or establish continuous-time/hybrid robot safety.

A physical recoverability claim requires a sound abstraction relation: every physically possible successor must be included in the corresponding graph successor set. If that cannot be established, the correct result is not a physical certificate.

## Next ladder

1. Validate the finite semantics against interval and brute-force oracles.
2. Construct conservative contact-mode / gait-phase graph abstractions.
3. Add interval reachable tubes across continuous flow, event guards and reset maps.
4. Compare against CBF screening, safe MPC, RL/QP shielding and viability-aware gait switching.
5. Connect to #4004 contact uncertainty, #3964 transition hysteresis, #3475 recovery scorecards and #4044 energy measurements.

## Validation

    cargo fmt --manifest-path research/loco-robust-viability-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-robust-viability-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-robust-viability-v0/Cargo.toml --locked --all-targets -- -D warnings
