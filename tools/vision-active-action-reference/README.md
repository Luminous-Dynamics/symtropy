# Offline visual active-action evaluator reference

This is a small standard-library-only Rust reference for post-commit evaluation of action selection in active vision.

## Run locally

```sh
rustc --edition=2024 --test tools/vision-active-action-reference.rs -o /tmp/vis-act-tests
/tmp/vis-act-tests
```

The file intentionally sits outside the Cargo workspace. It does not download models, require a GPU, or access hidden simulator state itself.

## What it scores

The evaluator receives:
- the policy's already-committed action;
- an evaluator-private set of paired counterfactual candidate losses and declared candidate resource estimates;
- the realized task loss after execution (including action failure/fallback);
- actual resource charges, where an absent measurement is unknown, not zero;
- a declared loss profile and a resource profile with separate units and per-trial/run-wide budgets.

It reports:
- chosen-action counterfactual value versus KeepCheap (positive means lower evaluator-side loss);
- realized task loss and regret relative to the **per-trial, declared-budget-feasible counterfactual minimum**;
- per-axis actual resource totals, unknownness, and budget status.

This is arithmetic for the evaluator, not an online allocator, observation engine, hidden-state oracle, or non-interference proof. The producer must never receive candidate losses, alternate-action outcomes, hidden truth, or future frames before committing its action. The simulator/orchestrator must enforce that causal boundary separately.

## Scope and nonclaims

- Candidate costs are declared estimates used for per-trial action eligibility; actual charged values are recorded separately.
- Actual energy is not inferred from latency, FLOPs, GPU time, or sensor counts. Use a calibrated power measurement or mark it NOT_MEASURED.
- The run-level reference uses an exact bounded dynamic program over declared run-budget axes (up to 100,000 unique resource states). It fails closed rather than silently approximating when the cap is exceeded or no globally feasible plan exists. This oracle uses declared candidate cost estimates, not measured costs; actual budget compliance is reported separately from measured charges.
- A fresh observation is counted only when an eligible acquisition action executed and the declared sensor channel actually returned an observation. Render output, predictions and unexecuted views are not fresh evidence.
- It does not implement a policy, action feasibility engine, belief update, active camera control, or model comparison.
- Tests authored here are not evidence of passing until the standalone command executes successfully on the exact commit.

Related research: [VIS-ACT-001 issue #2239](https://github.com/Luminous-Dynamics/symtropy/issues/2239) and [protocol PR #2240](https://github.com/Luminous-Dynamics/symtropy/pull/2240).
