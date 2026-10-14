# Living World Composite Authority Context V0

Status: normative design contract; documentation only.

## Purpose

A mature ecological process often needs information owned by several canonical stores at once. Process sufficiency therefore cannot remain permanently limited to one monolithic representation object.

Naively unioning capabilities is unsafe: exact stores may describe different snapshots/scopes, duplicate mutable ownership, or carry only separate marginals where a process requires an explicit cross-store relation.

## Core invariant

> Canonical process sufficiency may compose across multiple authority stores only through an explicit consistency context. Composition preserves declared information; it never invents an undeclared relation between separately exact facts.

## Candidate context

A resolved source SHOULD bind at least:

- registered source/representation identity and schema;
- authority scope (region/population/entity/domain);
- non-reused snapshot/state token or explicit consistency relation;
- source revision/generation;
- registered capabilities/evidence;
- ownership/reconciliation role where mutable exact authority is involved;
- applicable spatial/temporal qualification.

A `CompositeCapabilityContext` is a deterministic set of mutually compatible resolved sources plus any explicit cross-store relation capabilities required by the process.

## Heterogeneous authority is allowed

A coherent process snapshot may include different store types, for example:

- population state for headcount and marginals;
- sparse disease cohort relation store;
- ecological ledger for exact conserved accounts;
- spatial/contact index;
- Level-A active-owner store;
- Level-I identity/biography store;
- qualified environmental driver store.

The goal is consistency, not one giant state struct.

## Snapshot consistency

Sources may compose only when they prove the consistency required by the process.

That MAY mean one shared snapshot token, or an explicitly qualified relation between asynchronously updated sources under the spatiotemporal contract.

Individually Exact facts from incompatible revisions do not form an exact same-snapshot process context.

Prepared mutating operations bind every source revision/token on which their sufficiency proof depended. Any incompatible required-source advance before COMMIT invalidates the whole plan.

## Cross-store relation rule

Separate exact variables do not imply their joint relation.

Examples:

- exact age marginal + exact disease marginal != exact age×disease;
- occupancy marginal + genotype marginal != genotype×habitat;
- current position + social-group membership != contact history;
- aggregate biomass + Ledger Living total != one-to-one population/ledger ownership allocation.

If a process needs such a relation, the context contains an explicit exact or qualified relation capability whose provenance is itself registered/inspectable.

## Ownership rule

Information composition is not duplicate authority ownership.

Several stores may contribute read facts, but each mutable exact ecological quantity still has one canonical owner unless a typed hierarchical/reconciliation contract explicitly permits a parent/child accounting relationship.

Two incompatible exclusive ownership claims make context assembly fail closed.

## Context assembly

Conceptually:

`CAPTURE -> RESOLVE -> CHECK SCOPE -> CHECK SNAPSHOT -> CHECK OWNERSHIP -> CHECK CROSS-RELATIONS -> EVALUATE`

Only after successful assembly may process-information sufficiency be evaluated against the composite context.

The result SHOULD retain an explanation mapping each process requirement to the source/evidence that satisfied it, enabling Observatory diagnostics.

## Reachability interaction

A new process requirement need not always transform the population representation itself. A compatible retained sidecar authority store may already provide the missing information.

Conversely, adding another marginal source does not create missing covariance.

Fidelity planning therefore considers:

1. already available compatible authority sources;
2. legal transition/reconstruction paths;
3. composite contexts that satisfy the process;
4. deterministic canonical selection among reachable sufficient contexts.

## Determinism

Context identity/evaluation is independent of:

- source insertion order;
- thread scheduling;
- renderer/camera state;
- hardware load;
- hash-map iteration order.

## Qualification fixtures

- headcount from PopulationState + exact carbon account from Ledger satisfies a process requiring both;
- population snapshot G + ledger snapshot G-1 rejects a same-snapshot requirement unless an explicit temporal consistency rule permits it;
- age marginal + disease marginal rejects exact age×disease;
- explicit exact age×disease relation store at compatible snapshot passes;
- duplicate exclusive biomass ownership claims reject;
- one required source mutates after PREPARE -> commit rejects with zero partial mutation;
- equivalent source order -> identical context/report;
- removing one required source makes process insufficient;
- save/reload reconstructs the same context identity from canonical source metadata.

## Relationship

This contract generalizes single-representation sufficiency without weakening source authority. It composes registry ownership, spatiotemporal validity, promotion provenance, atomic process activation, and deterministic fidelity selection.

Relates to #154, #210, #252, #263, #269, #270, #271, #272, #273, #279, #280, #281.
