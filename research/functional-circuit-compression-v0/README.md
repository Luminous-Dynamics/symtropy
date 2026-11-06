# Functional Circuit Compression V0

Standalone, dependency-free Rust exhaustive oracle for [Symtropy #3764](https://github.com/Luminous-Dynamics/symtropy/issues/3764).

## What it qualifies

Given a frozen, source-qualified response decomposition, this crate exhaustively enumerates component subsets and identifies **all inclusion-minimal subsets** that preserve the full model's declared intervention-response signatures within per-output tolerances.

The response decomposition supports:
- independently removable components (neurons, synapses, pathways or sensory channels);
- structural dependencies (a synapse can only be retained when its endpoint neuron components remain);
- multiple interventions and behavioural contexts;
- multiple output channels with separately declared units/tolerances;
- higher-order synergy terms that contribute only when every required component is retained;
- explicit abstention when source coverage or component evidence is incomplete.

The implementation is an exact finite search over at most 20 components, with a hard evaluation budget. If the complete subset space exceeds the budget, it returns an error; it never emits partial results labelled exact.

## Scientific and statistical boundary

This crate is a **synthetic software oracle**, not a connectome parser, causal estimator, neural simulator, anatomical truth detector or biological discovery.

It consumes a frozen contribution-term expansion supplied by a separately qualified model adapter. Exactness means exhaustive enumeration relative to those supplied terms and declared absolute tolerances. It does not establish that the terms are biologically correct, that the full model is adequate, or that preserving the full model's predicted outputs implies real-world causal sufficiency.

A circuit is sufficient only for the declared scenarios, outputs, tolerances, source release, model version and intervention domain. Missing/unobserved source coverage blocks certification; unknown anatomy is never treated as zero or absent.

The algorithm reports all inclusion-minimal sufficient candidates. Multiple candidates can represent redundant or alternate sufficient pathways. A minimal circuit for walking is not thereby minimal or sufficient for turning, climbing, sensory gating, load adaptation or disturbance recovery.

## Model contract

For scenario s, output j, and retained component set S, the synthetic oracle computes:

    y_hat(s,j,S) = sum(delta(k,s,j) for terms k where required_components(k) is a subset of S)

Each term is active only when all components in its required set are retained. This supports additive contributions and explicit higher-order interactions. The full component set supplies the frozen reference.

A candidate is sufficient when every scenario/output satisfies:

    abs(y_hat(s,j,S) - y_hat(s,j,V)) <= epsilon(s,j)

where epsilon is the predeclared absolute tolerance in that output's units.

This is a deterministic response-signature contract, not a probabilistic predictive likelihood. A future adapter must add held-out trajectory proper scores and calibrated uncertainty before any biological sufficiency claim.

## Tests

Nine deterministic tests cover:
1. recovery of a necessary path with an irrelevant component removed;
2. discovery of alternative redundant minimal pathways;
3. preservation of a synergistic pair;
4. context-specific necessity across walking and turning;
5. endpoint dependencies for synapses and neurons;
6. abstention on incomplete source coverage or uncertain component identity;
7. fail-closed search-budget exhaustion;
8. invariance to input ordering;
9. rejection of unknown component references.

## Run

    cargo fmt --manifest-path research/functional-circuit-compression-v0/Cargo.toml -- --check
    cargo test --manifest-path research/functional-circuit-compression-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/functional-circuit-compression-v0/Cargo.toml --locked --all-targets -- -D warnings
