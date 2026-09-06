# Living World Ecological Flux Arbitration v0

Status: companion dynamics contract for deterministic same-tick resource competition and conservative multi-process updates.

## Problem

Independent ecological processes may fire on the same authoritative simulation tick and compete for the same finite source:

- multiple populations consuming one food pool;
- roots and fungi drawing from one nutrient/water account;
- decomposers competing for detritus;
- predators targeting overlapping prey availability;
- infrastructure/ecology drawing from the same water/storage source.

If each process mutates canonical state immediately in ECS/system iteration order, whichever process runs first can consume resources first. Then scheduler order, thread timing, or collection ordering becomes ecological law.

Living World must make scarcity resolution explicit.

## Core update theorem

For simultaneously scheduled processes that are intended to observe the same ecological instant:

```text
canonical pre-state
        |
        +--> process A intent
        +--> process B intent
        +--> process C intent
        |
        v
validate + arbitrate shared constraints
        |
        v
one atomic settlement batch
        |
        v
canonical post-state
```

A process computes a typed proposal/flux intent from the same authoritative snapshot. It does not directly consume contested canonical quantity before arbitration.

## Flux intent

A flux intent states what a process wants to transfer/change without granting itself the resources.

Conceptually:

```rust
pub struct EcologicalFluxIntent<Source, Destination> {
    pub source: Source,
    pub destination: Destination,
    pub quantity: ConservedQuantity,
    pub requested: ExactAmount,
    pub policy_class: FluxPolicyClass,
    pub stable_order_key: FluxOrderKey,
}
```

The concrete types may differ.

Intent generation must be deterministic from canonical pre-state and authoritative process inputs.

## Scarcity

When total demand does not exceed source availability, all valid intents may be satisfied exactly.

When demand exceeds supply, an explicit policy decides allocation.

Valid policy families may include:

- proportional allocation;
- ecological priority/trophic policy;
- reservation/ownership rights;
- minimum-guarantee then proportional remainder;
- deterministic auction/competition model;
- physically ordered contact/event time when the model truly has sub-tick chronology.

The critical requirement is that the rule is part of the ecological model, not an accident of system iteration order.

## Batch atomicity

The resolver preflights the entire settlement batch:

- source capacities;
- destination overflow;
- count/biomass consistency;
- authority ownership;
- mutually exclusive outcomes;
- cross-quantity reaction constraints;
- external input/output limits;
- handle/source-generation validity.

Then the batch commits atomically or fails closed according to the domain's transaction policy.

A partial commit followed by failure is not acceptable for one declared atomic ecological instant.

## Stable ordering is not allocation policy

A canonical stable key is still useful for:

- exact ties;
- replay serialization;
- deterministic diagnostic output;
- resolving genuinely discrete mutually exclusive events when policy says one wins.

But stable ordering must not silently become the default scarcity policy.

For example, sorting species IDs and allowing each to consume until food is gone is deterministic but biologically biased.

## Multi-rate cadence

`EcologicalCadence` can determine which processes are due on an authoritative tick.

Cadence answers **when a process proposes work**.

Flux arbitration answers **how simultaneously due processes share constrained state**.

The two responsibilities remain separate.

If a higher-level integrator intentionally defines sequential substeps (for example, photosynthesis before nightly respiration), that chronology is explicit model semantics rather than incidental execution order.

## Snapshot semantics

For one arbitration group, every intent should normally read the same canonical pre-state.

A process must not see another peer process's uncommitted mutation simply because it happened to execute later on a CPU thread.

Where feedback within the same tick is scientifically required, model it as explicit staged phases/substeps with named boundaries.

## Extensive versus intensive state

Flux arbitration primarily governs extensive/conserved quantities and discrete authority transfers.

Intensive fields may be updated by numerical solvers with their own stability rules, but any coupling that consumes/produces canonical extensive quantity crosses the explicit settlement boundary.

## Population outcomes

A population process might submit intents such as:

```text
grazer cohort -> consume plant biomass
population -> release detrital biomass
population A -> migrate living biomass/count to region B
fungal stratum -> absorb nutrient mass
```

If an intent changes both count/stratum state and conservation accounts, those deltas belong to one settlement transaction.

## Predation

Predation can involve discrete prey selection rather than fungible mass alone.

At coarse scale, a permutation-invariant mortality intent may consume prey count/biomass from a stratum.

At Level A, specific prey handles may participate in canonical events.

Two predators cannot both successfully kill the same active prey simply because their systems read the pre-state concurrently. Conflict resolution must reject or order mutually exclusive claims deterministically according to canonical event/sub-tick semantics.

## Reproduction and shared resource budgets

Competing reproduction processes may request limited nutrients/energy/space.

Birth count should be resolved from allocated resource rather than each process independently assuming full availability.

This prevents oversubscription followed by compensating deletion.

## Parallelism

Intent generation is naturally parallelizable because it is read-only over the shared pre-state plus process-local deterministic inputs.

Arbitration/settlement provides the serialization boundary for contested authority.

This gives Symtropy a route to parallel ecological simulation without surrendering deterministic results to thread scheduling.

## Hierarchical arbitration

Large worlds may arbitrate locally first and globally only where accounts overlap.

For example:

```text
cell/patch local fluxes
   -> region settlement
   -> cross-region boundary transfers
```

The hierarchy must preserve conservation and cannot allocate the same source capacity independently at two levels.

## Qualification requirements

Evidence must establish at least:

1. permuting intent generation/system iteration order does not change final canonical result;
2. parallel and serial intent generation produce equivalent intents/settlement;
3. total allocated amount never exceeds source capacity;
4. unconstrained demand is satisfied exactly;
5. oversubscribed demand follows the declared policy exactly;
6. exact ties use stable deterministic rules;
7. failed batch preflight leaves every participating canonical state unchanged;
8. no active organism/resource can be consumed twice in one mutually exclusive settlement set;
9. multi-quantity biological updates reconcile with conservation/accounting atomically;
10. explicit staged/sub-tick models are distinguishable from accidental sequential execution;
11. replay of the same pre-state + intents + policy produces identical settlement;
12. stress tests randomize insertion/thread order and demonstrate order independence.

## Design principle

**Determinism is not enough if the deterministic answer depends on arbitrary iteration order. Ecological competition must be resolved by an explicit biological/economic/physical rule over shared intents, then committed once.**
