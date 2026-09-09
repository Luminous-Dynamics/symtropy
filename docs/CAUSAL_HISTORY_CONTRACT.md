# Causal History Contract v0.1

This document is a narrow implementation companion to the civilizational authority contract.

The existing `symtropy-game-state::EventEnvelope` already carries `causal_parents`. This contract defines the minimum validity rules that make those references usable as historical causality rather than decorative metadata.

## Required invariants

For one `EventChain<T>`:

1. Every declared causal parent must refer to an event already present in the same verified chain.
2. A causal parent must occur strictly before the child in chain order.
3. A child may not list the same causal parent more than once.
4. Parent order must not be semantically significant for verification.
5. Hash-chain validity remains independent from causal-graph validity: both must pass.
6. A causal edge means "the child explicitly depends on this recorded prior event". It is not a claim that the parent is the only cause.

## Deliberate boundary

This V0 contract does not yet introduce typed causal-edge kinds such as enabling, preventing, evidentiary, economic, or organizational. It only makes direct ancestry structurally trustworthy.

A later history crate may build richer explanation semantics on top of these validated event identities.
