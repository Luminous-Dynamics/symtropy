# Construction robot safety envelope evaluator v0.1

Dependency-free synthetic reference evaluator for [CONSTRUCTION-ROBOT-SAFETY-001 #3317](https://github.com/Luminous-Dynamics/symtropy/issues/3317).

## Run

```sh
cargo test --locked --manifest-path benchmarks/construction-safety-envelope/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/construction-safety-envelope/Cargo.toml --all-targets -- -D warnings
```

## Scope

The evaluator binds a runtime observation to a machine, as-built revision, tool, task, site, ODD, policy and standards-profile revision. It checks declared human–machine separation, motion intensity, sensor health/freshness/calibration/confidence, independent sensor groups, operator state, mode and an externally observed emergency-stop state.

It emits a deterministic disposition and reason list only. It does not output a velocity, torque, actuator command, PLC signal, stop command, emergency-stop reset or permission to operate.

## Important authority boundary

- Numeric thresholds are inputs from a machine/site/application-specific policy owner. Fixture values are synthetic and must never be copied into a real machine safety case.
- Standards profile identifiers are provenance references; this crate does not parse standards or establish compliance.
- The emergency-stop state is an externally observed input. The evaluator cannot clear, reset, bypass, or actuate an emergency stop.
- This evaluator is not safety-rated and cannot be used in a physical safety loop.
- Unknown/stale/missing/conflicting evidence fails closed to hold/review. A hard stop cannot be outweighed by a low-risk score or another green input.
- The existing `JointSafetyAuthority` and qualified safety-rated control systems remain independent; this benchmark does not replace them.

## Synthetic response states

- WithinSyntheticEnvelope
- RequireProgressiveMitigation
- HoldForHumanReview
- StopRequiredBySyntheticPolicy
- EvidenceInconsistent
- UnsupportedODD

These are simulation dispositions, not control outputs. Progressive mitigation is only a declared requirement for external, independently validated supervisory logic; no mitigation action is executed here.
