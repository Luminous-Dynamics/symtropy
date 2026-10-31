# Lot-cluster bootstrap for manufacturing capability intervals

This standalone Rust reference compares resampling at the independent lot/campaign level against a deliberately naive window-level bootstrap.

## Statistical hierarchy

    independent lot / campaign
      -> production windows / subgroups
        -> measurement observations

The primary bootstrap resamples complete lots with replacement. Every selected lot contributes all of its windows and the observations within those windows. The naive control resamples windows individually and intentionally breaks the common lot-level effect. It is included to expose pseudoreplication, not as an acceptable inference method.

## Frozen stable DGP

- 20 independent lots.
- 4 windows per lot.
- 8 Normal observations per window.
- Global mean 0.5, lot random-intercept SD 0.05, within-window SD 0.04.
- LSL 0, USL 1.
- RNG: xorshift64* + Box-Muller, fixed seed; index selection uses modulo and is identified explicitly in the output.

The analytic target distinguishes:
- within-subgroup Cpk = 0.5 / (3 × 0.04) = 4.1667;
- marginal Ppk = 0.5 / (3 × sqrt(0.05² + 0.04²)) = 2.6029.

This is a deliberately simple stable, Gaussian, nested DGP to isolate the resampling-unit effect. It does not represent a real plant or prove that the Normal assumption is appropriate for a physical process.

## Outputs

- point Cpk/Ppk on the original synthetic campaign;
- cluster-bootstrap percentile 95% intervals;
- naive window-bootstrap percentile 95% intervals;
- interval widths, valid/invalid replicate counts;
- lot, window and measurement counts independently;
- optional coverage study with 500 outer campaigns, 250 bootstrap replicates, and 8/12/20/40/80 independent lots.

Coverage results are empirical and deterministic for the pinned seed. The benchmark reports stable and drift campaign outputs separately. Do not pool resampling replicates as if they were independent experiments.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-capability/cluster_bootstrap.rs -o /tmp/cluster-bootstrap-tests
    /tmp/cluster-bootstrap-tests

    rustc --edition=2024 -D warnings tools/manufacturing-capability/cluster_bootstrap.rs -o /tmp/cluster-bootstrap
    /tmp/cluster-bootstrap --verify
    /tmp/cluster-bootstrap --csv
    /tmp/cluster-bootstrap --coverage

## Interpretation

A wider cluster-bootstrap interval is not itself “better”; the correct resampling unit is the key. The coverage experiment compares whole-lot cluster intervals against the naive control under a known synthetic data-generating process. It should show how too few lots and treating windows as independent affect confidence-interval width and coverage. Do not assume 95% coverage simply because the nominal interval is 95%.

## Claim ceiling

This is a synthetic estimator-validation benchmark for lot-cluster bootstrap uncertainty on descriptive capability indices. It does not validate an empirical process distribution, establish production stability, implement ISO 22514-2:2026, certify parts or authorize release.
