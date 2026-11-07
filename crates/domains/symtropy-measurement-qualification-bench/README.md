# Symtropy Measurement Qualification Bench

Standalone, read-only qualification evaluator for particle-image measurement references and resolution-aware sample adequacy.

Implements [issue #3961](https://github.com/Luminous-Dynamics/symtropy/issues/3961), complementing #3910 morphology and #3934's physical pilot plan.

## Run

    cargo test --manifest-path crates/domains/symtropy-measurement-qualification-bench/Cargo.toml

## v0.1 capabilities

- Reference-artifact checks with certified values/uncertainty, repeated measurements, distinct sessions/operators/orientations and declared maximum error.
- Conservative reference bias envelope: absolute mean bias + certificate's expanded uncertainty (with coverage factor) + maximum observed repeat deviation + half descriptor-resolution quantum.
- Spatial resolution and feature-size admission; measurements below the declared resolution limit are not converted to zero.
- Exact parent morphology case, interval, PSD-bin and descriptor definition binding.
- Unique particle identities, distinct raw-image/segmentation artifacts, one observation per independently declared acquisition block, and a particle-size uncertainty interval wholly inside the exact parent PSD bin; clustered samples are rejected until a cluster-aware method is implemented.
- Exact, distribution-free order-statistic confidence intervals for predeclared p05/p50/p95 quantile targets under the explicitly declared independent random-sampling design.
- Precision gate: each interval width must satisfy a predeclared descriptor-unit maximum.
- Censored values remain censored and prevent an exact quantile qualification.

## Statistical scope

For independent observations X_1,...,X_n, the count B of observations at or below the population p-quantile has Binomial(n,p) distribution. The interval [X_(r), X_(s)] has coverage P(r <= B <= s-1). v0.1 searches order-statistic bounds and uses declared descriptor-domain limits only where a finite sample bound is unavailable.

The interval is distribution-free **conditional on the IID sampling premise**. It does not correct selection bias, hidden acquisition clustering, unrepresentative material collection, operator bias, changing geology, censoring, or domain shift. One particle per independently declared acquisition block is required in v0.1; cluster-aware bootstrap and finite-population corrections are future work.

The reference bias envelope is intentionally conservative and is not a full GUM uncertainty budget. It does not combine correlated components by RSS or imply metrological traceability beyond the references supplied by the experiment owner.

## Non-claims

This crate is not ISO certification, scanner qualification, a complete GUM implementation, field measurement proof, a mine operating envelope, or a transport model. Synthetic fixtures are arithmetic conformance only.
