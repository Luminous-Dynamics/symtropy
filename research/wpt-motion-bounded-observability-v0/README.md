# WPT motion-bounded observability v0

**Status:** finite, robust motion/observation relation solver over a frozen synthetic transition corpus. This is not a continuous dynamics model, sensor estimator, flight controller, electromagnetic solver, inverter controller or safety interlock.

## Research objective

A static probe policy can correctly distinguish a receiver pose at the beginning of acquisition and still be wrong at the end: the receiver may move while the probe is in progress. This crate propagates every declared possible current state through a bounded transition relation and evaluates observations at the end of each action.

For an action a and observed outcome o:

```text
B_(k+1)(o) = { s' | exists s in B_k:
                       s' in T_a(s) and o in O_a(s') }
```

Every possible transition and every possible observation outcome is retained. The decision tree terminates only when all remaining state hypotheses map to one pose-equivalence ID. If any reachable transition leaves the qualified domain, an action's validity horizon expires, a transition is missing, or no complete budget-feasible tree exists, the solver abstains.

## Ownership

- #4115 owns modality-specific pose equivalence and observation semantics.
- #4214 / PR #4239 owns the static exact adaptive probe policy and qualified probe actions.
- #4080 owns session lifecycle and the upstream aircraft motion envelope.
- #4197 owns transmitter field/configuration candidates.
- #3472 owns measured physical energy planes and synchronized metrology.
- This crate owns finite motion propagation and the exact small-instance policy oracle only.

The input transition and observation relations are declared upstream evidence. This tool does not infer them.

## Input contract

The TSV contains one row per action / source-state / possible-successor edge. All rows repeat identical case metadata and action metadata.

Columns:

1. case_id
2. evidence_class
3. initial_states (semicolon-separated state IDs)
4. frame_sha256
5. calibration_sha256
6. energy_budget_uj
7. latency_budget_us
8. max_actions
9. max_ripple_ppm
10. min_zvs_margin_permille
11. max_service_drop_ppm
12. action_id
13. action_kind (PASSIVE_OBSERVE or ACTIVE_EXCITATION)
14. energy_cost_uj
15. duration_us
16. ripple_ppm
17. zvs_margin_permille
18. service_drop_ppm
19. from_state
20. from_pose_id
21. to_state
22. to_pose_id
23. outcomes (semicolon-separated set-valued observation classes)
24. transition_in_domain (true / false)
25. valid_until_us (absolute elapsed time from episode epoch)

State IDs are finite model states; pose IDs are the upstream pose-equivalence classes. Multiple state IDs may map to one pose ID. Multiple successors and multiple outcomes are represented by repeated edges and set-valued outcome lists. All state-to-pose mappings must be consistent across the case.

An action is usable only if every possible successor for every state in the current belief is present, remains in-domain, and is within the action validity horizon. An undefined transition is not treated as impossible; it makes that action unavailable.

## Objective

The finite solver enumerates feasible robust decision trees and retains a Pareto frontier over:
- worst-case cumulative incremental probe energy;
- worst-case cumulative acquisition latency;
- maximum number of actions on any branch.

The selected tree minimizes those values lexicographically in that order, with stable action/outcome ordering. Hard power-quality limits and episode budgets are feasibility gates, never soft penalties.

## Frozen controls

- stationary identity transitions: reproduces the #4239 passive-first result (15 synthetic µJ, 7 synthetic µs);
- moving ambiguity: a probe that yields the same observation for two different current poses must not falsely admit a unique pose; a second action resolves the ambiguity;
- invalid reachable transition: action is rejected and the solver abstains;
- expired action validity: action is rejected and the solver abstains;
- malformed, inconsistent or missing transition records: input is rejected.

## Validation targets

- 100% agreement with an independent exhaustive oracle on the frozen small-state corpus;
- exact stationary identity-transition equivalence to the #4239 fixture's 15 µJ / 7 µs adaptive result;
- zero plan branches ending with multiple pose-equivalence IDs;
- zero use of actions with out-of-domain reachable successors;
- zero use of actions beyond their validity horizon;
- 100% abstention when no complete feasible policy exists;
- deterministic output under row permutation.

All values in fixtures are synthetic software-test values. They are not measured physical excitation energy, aircraft motion bounds, calibrated localization performance, exposure measurements, or safety limits.

## Run

```sh
cargo test --locked --manifest-path tools/wpt-motion-bounded-observability/Cargo.toml
cargo run --locked --manifest-path tools/wpt-motion-bounded-observability/Cargo.toml -- research/wpt-motion-bounded-observability-v0/stationary-identity.tsv
cargo run --locked --manifest-path tools/wpt-motion-bounded-observability/Cargo.toml -- research/wpt-motion-bounded-observability-v0/moving-ambiguity.tsv
```

## Claim ceiling

A passing result establishes finite-set software semantics only. It does not validate a real motion envelope, physical probe waveform, electromagnetic or sensor model, calibration coverage, aircraft safety, electrical protection, regulatory compliance, operational admission, or superiority over a physical WPT system. The outputs are never power or flight permits.
