# Component Condition Transition Reference

Dependency-free Rust 1.96 reference for one narrow lifecycle-assurance problem:

**Can damage, inspection, repair and component replacement evidence be applied to the exact physical component generation and revision, without duplication, stale writes, evidence-class promotion or loss of prior history?**

## Included

- Exact physical-article, component-instance, configuration-revision and material-lot identity.
- Revision-checked, append-only condition events.
- Separate synthetic, model-estimate and measured evidence classes.
- Separate deterministic derived artifacts (such as cycle-count receipts) from raw measured data and model-estimate outputs.
- Damage-receipt adapter for the bounded Miner and constant-Delta-K Paris output shapes: binds exact component/material identity, source load history, cycle-record digest/profile, calibration scope, model-profile artifact, model output artifact, applicability and optional uncertainty before creating a condition event.
- SHA-256 *syntax* validation and explicit evidence-reference binding. This crate does not hash/authenticate source bytes.
- Mechanism-specific typed quantities for Miner accumulation, crack length, corrosion loss and wear volume.
- Explicit in-domain / out-of-domain / incomplete / inconclusive dispositions.
- Declared model profile for model estimates; uncertainty remains optional and unknown when not supplied.
- Single-clock event/recorded time roles; cross-clock events require an upstream transform and are rejected here.
- Atomic rejection of duplicate events, duplicate claims, stale revisions, wrong subjects, bad digests, invalid units, time regressions and invalid replacement lineage.
- Repair/reinforcement and replacement require at least one measured postcondition/initial-state claim; model estimates may accompany but cannot substitute for measured event evidence.
- Repair appends postcondition claims; it does not erase earlier damage/usage history.
- Replacement creates a new component-instance/configuration generation while preserving predecessor history.

## Deliberate boundaries

This is a reference contract, not a solver, fatigue calculator, load-history processor, material-flow ledger, structural-health score, RUL predictor or maintenance authorization. It does not compute SHA-256, authenticate evidence, check rights/signatures, estimate physical condition, qualify reuse, or establish structural safety. It verifies the continuity of supplied claims and receipts only.

The next composition adapters should consume the separately qualified outputs of Symtropy #2209, #4859, #4864 and #4862. The research contract is tracked in #4867.

## Local verification

```sh
cargo +1.96.0 fmt --manifest-path research/component-condition-transition/Cargo.toml -- --check
cargo +1.96.0 test --manifest-path research/component-condition-transition/Cargo.toml --locked
cargo +1.96.0 clippy --manifest-path research/component-condition-transition/Cargo.toml --all-targets --locked -- -D warnings
```

The isolated workflow runs these commands on the exact PR head. Until it does, source review and authored tests are not a passing compile/test claim.
