# ErgoStream structural-transfer witness v0.1

**Status:** finite Rust reference contract; implementation PR is stacked on Symtropy #7152
**Tracking:** [ERGO-STREAM-TRANSFER-001 #7171](https://github.com/Luminous-Dynamics/symtropy/issues/7171)

## Purpose

The initial ErgoStream microcosm can score segmentation and within-world equivalence recovery. This module adds a distinct question: can a learner-proposed correspondence map one complete finite structure to another without silently collapsing different relationships?

This is a validator for a proposed witness, not a mapping-discovery algorithm.

## Structural correspondence

The learner supplies:

- a total one-to-one mapping of source node IDs to target node IDs;
- a total one-to-one mapping of relation instances;
- a total one-to-one mapping of domain-local relation kinds;
- one explicit participant transformation per mapped relation kind.

The v0.1 transforms are:

- `PreserveOrder`: directed participant order is unchanged;
- `ReverseOrder`: directed participant order is reversed;
- `Unordered`: undirected incidence is compared as a set after sorting.

Validation rejects incomplete or non-injective maps, unknown identifiers, relation-kind collisions, arity or directedness mismatches, incompatible transforms, and incidence counterexamples.

This version deliberately requires a total correspondence over the supplied finite structures. Partial analogies and substructure scopes need an explicit scope contract in a later revision; they must not be smuggled in by omitting inconvenient relations.

## Qualitative intervention-effect correspondence

A structural map does not imply that causal behavior transfers.

The separate intervention validator accepts a precommitted action mapping and an expected qualitative effect transform (`Preserve` or `Negate`). It compares that mapping with the supplied qualitative effect observations. Missing evidence is an error; contradictory evidence rejects the proposed effect mapping. Its result field, `all_declared_effects_match`, means only that every supplied qualitative observation matches the declared transform. An empty mapping returns `false` and is not qualified.

The caller must freeze the proposed action/effect mapping before disclosing held-out intervention outcomes. This Rust function cannot enforce that temporal custody rule, authenticate intervention execution, establish intervention validity, quantify effect uncertainty, or prove that observations are independent. Matching qualitative signs are not, by themselves, a causal-transfer result.

## What this establishes—and what it does not

Passing tests establish only that this implementation validates the declared finite correspondence rules on its test fixtures. They do not establish:

- that a learner can discover useful mappings;
- that the source and target structures are scientifically or causally equivalent;
- that the intervention records are authentic or independently generated;
- that the hidden evaluator is isolated from an adversarial process;
- transfer to continuous dynamics, multimodal data, robotics, or real environments;
- superiority over any external method or benchmark.

## Required negative controls

The tests include incidence changes, relation-kind collisions, non-injective node mappings, incomplete relation maps, incompatible directedness transforms, and intervention-effect contradictions. Future world-pair generators must add matched-marginal false analogies, observationally non-identifiable pairs, and transformations that preserve superficial patterns while changing the underlying mechanism.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
