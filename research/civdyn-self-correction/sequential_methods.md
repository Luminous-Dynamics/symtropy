# CIVDYN sequential causal-inference method comparison v0.1

## Purpose

This reference compares sequential inference rules on identical paired-outcome streams. It is a statistical-method benchmark, not a new civilization simulator and not a claim of empirical causal identification.

The five methods are:

1. Fixed-horizon one-sided directional tests at 2,048 pairs.
2. The conservative four-look z=2.8 reference inherited from the earlier noisy benchmark, now applied to nested looks.
3. O'Brien-Fleming four-analysis boundaries: 4.084, 2.888, 2.358 and 2.042.
4. Pocock four-analysis boundary: 2.361 at each look.
5. An equal-weight Gaussian mixture e-process evaluated after every paired observation.

The four group-sequential looks are at 512, 1,024, 1,536 and 2,048 pairs. The O'Brien-Fleming and Pocock values are tabulated two-sided overall-alpha-approximately-0.05 boundaries for four equally spaced analyses. They are used as the same standardized boundary for the two directional hypotheses. This is distinct from simply applying a nominal p-value at each look. See the [Penn State group-sequential reference](https://online.stat.psu.edu/stat509/Lesson10).

## Common stream design

Each of the five frozen causal regimes has 256 deterministic seed episodes. Each episode generates exactly 2,048 paired treatment/control differences. Both members of a pair share the same confounder value; their outcome noise is independent Gaussian with standard deviation 2.0. The known variance of a paired difference is therefore 8.0 in this synthetic reference.

All five methods receive the exact same observation stream, verified using a common 64-bit FNV-1a digest. Group-sequential analyses use nested prefixes of that stream, not independently sampled batches. This makes sample-use comparisons fair.

The earlier #4185 runner remains unchanged. It uses fresh batches of 2,048 matched pairs at each of four predeclared looks. This branch instead compares inference rules on one common stream capped at 2,048 pairs.

The regime values are endpoint snapshots at tick 8:

| Regime | Final true effect | Final observational association | Expected classification |
|---|---:|---:|---|
| Stable positive | +2 | +7 | Benefit established |
| Abrupt sign reversal | -3 | +2 | Harmful effect established |
| Gradual sign reversal | -2 | +3 | Harmful effect established |
| Confounded null | 0 | +5 | Inconclusive |
| Confounder-only shift | +2 | -3 | Benefit established |

These endpoint snapshots do not measure temporal diagnosis latency or demonstrate that a controller tracked the entire transition path. Observational association is exported for interpretation but is not used by the paired randomized-effect estimator.

## Decision definitions

The benchmark maintains three mutually exclusive outcomes:

- Harmful effect established: evidence supports a negative causal effect, tested against the composite null that the effect is nonnegative.
- Benefit established: evidence supports an effect above the declared action threshold of 0.5, tested against the composite null that the effect is at most 0.5.
- Inconclusive: neither directional claim has been established.

Failure to establish benefit is never automatically classified as harm.

## Gaussian mixture e-process

For paired differences X_i with known variance sigma squared = 8, define:

- Harm test: Y_i = -X_i, whose null has mean at most zero when the causal effect is nonnegative.
- Benefit test: Z_i = X_i - 0.5, whose null has mean at most zero when the causal effect is at most the minimum worthwhile benefit.

For each fixed lambda greater than or equal to zero, the process

E_n(lambda) = exp(lambda times sum(Y_i) - n times lambda squared times sigma squared / 2)

is a nonnegative supermartingale under the corresponding Gaussian composite null. A fixed convex mixture of these processes remains a supermartingale. This implementation uses equal weights over lambda values 1/16, 1/8, 1/4 and 1/2.

Each direction receives alpha = 0.025, giving an e-value threshold of 40. Ville's inequality controls optional stopping separately for each direction; a union bound controls either directional declaration at 0.05 per episode. This guarantee depends on the declared known-variance Gaussian sampling model. It is not distribution-free.

The 256 simulated episodes are Monte Carlo replications used to estimate operating characteristics. The per-episode alpha guarantee is not a family-wise guarantee across all 256 episodes, all regions, all agents or repeated real-world deployments. Operational use would require explicit multiplicity allocation, recycling or hierarchical testing.

## Interpretation

The benchmark can test:

- Correct implementation of the stated synthetic Gaussian procedures.
- Method comparisons on identical paired observations.
- Sample-use and operating-characteristic differences across the five frozen endpoint regimes.
- Optional stopping behavior under the declared null and variance assumptions.
- Whether lack of benefit evidence remains distinct from evidence of harm.

It cannot establish:

- Real-world causal identification or external validity.
- Validity under heteroskedasticity, heavy tails, serial dependence, missing outcomes or variance misspecification.
- Validity under adaptive treatment allocation unless the sampling assumptions are separately justified.
- Superiority over all sequential-inference methods or a general state-of-the-art claim.
- Optimal policy decisions or welfare improvements.

## Research extensions

1. Add exact/permutation or randomization-based inference for the paired assignment design.
2. Add a sequentialized terminal-test / Doob-martingale construction motivated by Koning and van Meer (JRSSB, 2026), "Anytime validity is free: inducing sequential tests": https://doi.org/10.1093/jrsssb/qkag050.
3. Compare likelihood-ratio, mixture-SPRT and confidence-sequence variants with independently checked assumptions.
4. Add heteroskedastic, heavy-tailed, autocorrelated, delayed and missing outcomes.
5. Extend to clustered, network-interference and adaptive-allocation experiments. The independent-pair model is insufficient for those settings.
6. Rejoin the statistical benchmark to the full changing-mechanism benchmark, keeping detection time, diagnosis time and intervention time separate.
7. Study costly sampling and adaptive allocation in light of Adusumilli (JPE, 2026), "Optimal Tests Following Sequential Experiments": https://doi.org/10.1086/742418.

## Run

    cargo fmt --manifest-path research/civdyn-self-correction/Cargo.toml --all -- --check
    cargo clippy --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets -- -D warnings
    cargo test --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets
    cargo run --locked --manifest-path research/civdyn-self-correction/Cargo.toml --bin sequential-methods-sweep
