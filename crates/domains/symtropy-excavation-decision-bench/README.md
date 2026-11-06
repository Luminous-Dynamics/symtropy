# Symtropy Excavation Decision Bench

Standalone deterministic evaluator for evidence-bounded excavation strategy eligibility and robust Pareto comparison.

## Scope

The crate evaluates one decision case against one declared profile:
- application-specific minimum evidence tier;
- geology and machine support;
- external engineering-qualification status/reference;
- common energy-boundary reference and resource-ledger completeness;
- independently measured primary outcome intervals;
- hard constraints;
- robust interval-separated Pareto dominance.

It returns Eligible / Ineligible / Indeterminate for every strategy, an eligible set, a robust Pareto frontier, and explicit abstention when nothing is eligible.

It does not:
- calculate a weighted scalar score;
- claim statistical significance from interval separation;
- infer causal treatment effects;
- calculate EVSI/EVPI without utility and observation models;
- authenticate sources or calibrations;
- qualify physical methods, certify geology, issue safety decisions, or control equipment.

## Run

```sh
cargo test --manifest-path crates/domains/symtropy-excavation-decision-bench/Cargo.toml
```

## Semantics

For maximization, A is robustly no worse than B when lower(A) >= upper(B).
For minimization, A is robustly no worse than B when upper(A) <= lower(B).
A dominates B only when no worse on all objectives and strictly better on at least one.

Overlapping intervals do not establish dominance. Ineligible or indeterminate candidates cannot dominate eligible candidates.

When primary outcomes require independent measurement, model, simulation, operator-reported and process-sensor roles are not silently promoted to physical ground truth.