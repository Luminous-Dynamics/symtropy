# Locomotion Factorial Oracle V1

A dependency-free Rust known-answer challenge for the three-factor locomotion intervention design tracked by Symtropy issue #3462:

- **A — central rhythm / descending drive**
- **B — local proprioceptive joint-limit reflex**
- **C — peripheral load / proprioceptive feedback**

The first version is a design-identifiability oracle, not a neural or biomechanical simulator.

## What it qualifies

The outcome is represented by a three-factor, two-level factorial model:

    y = beta0 + betaA*A + betaB*B + betaC*C
      + betaAB*A*B + betaAC*A*C + betaBC*B*C + betaABC*A*B*C

Each factor is coded as -1 or +1. Coefficients therefore refer to this declared additive outcome scale and coding.

V1:
- validates unique factorial cells and finite outcomes;
- constructs the eight-column design matrix;
- reports numerical rank and whether all eight coefficients are identifiable;
- recovers coefficients only when the complete 2x2x2 design is present;
- explicitly abstains when a missing-cell or fractional design is rank-deficient;
- classifies main effects, pairwise interactions and the three-way interaction separately;
- includes synthetic known-answer tests and zero-interaction controls.

## Why this is the next gap

The model-discrimination oracle in #3454 checks frozen model forecasts and pairwise predictive separation. Issue #3462 proposes a factorial CPG-reflex-load experiment. This crate independently checks whether the intervention design can identify the main, pairwise and three-way interaction terms being claimed.

It prevents a common failure mode: interpreting a combined-treatment difference as synergy when the design cannot separate the three-way interaction from lower-order effects.

## Run

    cargo fmt --manifest-path research/locomotion-factorial-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-factorial-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-factorial-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

## Claim boundary

A full-rank design establishes algebraic estimability of the declared coefficients on the chosen outcome scale. It does not establish adequate statistical power, randomization, causal validity, absence of confounding, sample independence, biological truth, or a Drosophila mechanism.

The crate does not fit repeated-measures or hierarchical models, estimate animal/session-clustered uncertainty, choose biological interventions, simulate a CPG, represent receptor physiology, or produce a locomotor trajectory. Those remain separate gates. Repeated trials must be handled by a source-aware statistical layer before their outcomes are reduced to one factorial cell estimate.

A non-identifiable design is reported as non-identifiable; V1 never imputes an unobserved factorial cell or declares a unique mechanism from a rank-deficient matrix.
