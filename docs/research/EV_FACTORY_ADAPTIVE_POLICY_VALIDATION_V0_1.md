# EV factory adaptive lifecycle policy validation v0.1

**Issue:** [EV-FACTORY-ADAPTIVE-VALIDATION-001 #2450](https://github.com/Luminous-Dynamics/symtropy/issues/2450)  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_adaptive_policy_validation.rs`  
**Status:** five-fold leakage check over synthetic lifecycle profiles; not evidence of industry generalization

## Why leave one family out?

The adaptive-policy frontier contains several threshold rules that use profile signals such as active fraction, product-change rate, reuse success and redeployability. A selector must never look at the cost result for the family it is being scored on. This validation runs five folds: four families form the policy-selection set, and the fifth family is held out for scoring.

For each fold:
1. Select the lowest mean-NPV deployable policy using only the four training-family rows, including its declared decision overhead.
2. Separately select the lowest mean-NPV **fixed** policy from those same training families.
3. Apply the already-declared selected policy rule to the held-out family signals. The tool independently recomputes the chosen strategy from the catalog’s signal values and checks it matches the adaptive report.
4. Score that policy and the fixed comparator on the held-out family only after both choices are fixed.
5. Record regret against the held-out family’s cheapest lifecycle strategy as a non-deployable reference, never as an implementable policy.

The perfect-information oracle is excluded from policy-selection candidates. Tie-breaking is deterministic by policy ID. The input chain is verified by hashing the exact catalog bytes and report bytes, checking the lifecycle report's source catalog digest, and checking the adaptive policy report's digest links to both.

## What this benchmark reports

- The held-out family, the four training families, and the policy selected strictly on training data.
- Training mean NPV for that policy and for the training-selected fixed comparator.
- Held-out policy NPV including decision/measurement overhead, held-out fixed comparator NPV, and the signed difference (negative means the selected policy cost less).
- Held-out regret versus the family-specific perfect-information cost lower bound.
- Selection counts across five folds, mean and worst held-out policy cost, mean cost difference versus the fixed comparator, and whether the selected policy was adaptive or fixed in each fold.

Five folds are a protocol check, not five IID statistical samples. The scenario families are deliberately different engineering stress cases, and equal-weight averaging does not estimate real-world scenario frequencies.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_lifecycle_frontier -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_lifecycle_v0_1.json \
  > lifecycle-report.json

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_adaptive_lifecycle_policies -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_lifecycle_v0_1.json lifecycle-report.json \
  > adaptive-policy-report.json

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_adaptive_policy_validation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_lifecycle_v0_1.json \
  lifecycle-report.json adaptive-policy-report.json
```

## Required negative control

The unit test changes only the held-out family's policy cost after forming the training subset. The training-selected policy must remain unchanged. CI additionally verifies every family-policy row against the exact lifecycle result and charges the declared overhead; it rejects a changed catalog with an old lifecycle report, a changed lifecycle source digest, duplicated/missing policy rows, and an oracle in the deployable candidate set.

## Research comparison

The October 2026 paper *Adaptive robust optimization for sustainable assembly line design under product evolution* addresses a richer robust resource-selection problem under product evolution and considers rental/leasing/salvage-type options: https://doi.org/10.1016/j.ijpe.2026.110136. This LOFO check is a much simpler decision-evaluation protocol; it does not reproduce a robust MILP/C&CG optimizer or claim equivalent performance.

NIST's manufacturing digital-twin work emphasizes VVUQ and evidence interoperability as separate requirements from reproducible execution: https://doi.org/10.6028/NIST.IR.8620. LOFO can expose obvious policy overfit/leakage in this test suite, but it cannot substitute for time-separated measured traces, calibrated signal quality, or independent plant validation.

## Claim ceiling

All costs and profiles remain synthetic cost units. No policy is validated on a real plant, and five synthetic families cannot establish a state-of-the-art generalization claim. A passing LOFO result only verifies that the policy selection protocol keeps each held-out family out of training-policy selection.

Economic recommendation, production release, quality release, electrical/battery safety and jurisdictional compliance remain false or unresolved. No supplier, manufacturer, plant, landlord, integrator or agency was contacted; no quotes were requested.
