# Demographic intervention proof bundle V0

> **Historical B4A contract.** This one-step scaffold is superseded on the B4B6 stack by `DEMOGRAPHIC_PROOF_BUNDLE_V2.md`, which adds validated-prefix authority and heterogeneous multi-step replay. V0 remains here to document the fail-closed staging history; it is not the current proof-bundle execution contract.

Status: B4A implemented/static authority scaffold; not executable-qualified.

## Purpose

`DemographicInterventionProofBundle` is an ordered, revalidatable transcript for demographic interventions that occur within one biological generation.

It exists because a non-root `DemographicInterventionCursor` is only evidence-shaped data. The cursor identifies an ordered history position, but it cannot prove by itself that every predecessor intervention actually executed.

## Core theorem

`non-root cursor-shaped data != revalidated demographic history`.

Current B3 executors deliberately require an ordinal-zero root cursor. B4A does **not** relax that boundary.

## B4A scope

B4A introduces:

- `DemographicInterventionProofBundle`;
- `DemographicInterventionProofStep`;
- a closed `DemographicExecutionEvidence` enum over the existing V0 demographic executors;
- exact bundle identity independent of Serde/JSON byte layout;
- deterministic replay/revalidation from an exact root source cut.

Because existing B3 receipts themselves still require root validation, B4A can currently prove:

- an empty transcript representing exactly the root cut;
- one exact demographic execution from the root.

It deliberately **cannot yet authorize a second same-generation execution**. Two independently root-valid receipts cannot be concatenated and treated as one chain.

B4B must introduce a validated-predecessor authority/token and refactor executor/receipt validation so a proven non-root source cursor can be consumed without becoming self-authorizing.

## Root authority

Bundle declaration and revalidation require exact current:

- `HereditarySchema`;
- root `PopulationStructureProfile`;
- root population-state map;
- root trajectory-point map;
- root `MetapopulationSnapshot`.

The bundle binds the exact schema digest, root structure digest, root snapshot digest, experiment identity, and generation.

## Ordered proof steps

Each `DemographicInterventionProofStep` contains:

- one `DemographicEventDeclaration`;
- one `DemographicStructureTransition`;
- one explicit successor `PopulationStructureProfile`;
- one closed typed `DemographicExecutionEvidence` result.

The V0 execution-evidence variants are:

- census resize / bottleneck / explicit no-op;
- founder or recolonization;
- structural extinction;
- conservative population split;
- pulse admixture.

Execution-evidence kind must match event kind. The enum is intentionally closed; it is not a generic global event bus.

## Replay rule

Starting from the exact root cut, replay proceeds in vector order. For each step the validator:

1. requires the event to remain at the root experiment and generation;
2. revalidates the event and demographic structure transition against the current cut;
3. dispatches to the exact matching execution-provenance validator;
4. requires the execution result and history cursor to revalidate;
5. adopts the step's successor structure, result state/trajectory maps, result snapshot, and result cursor as the next current cut.

Partial prefix success never authorizes the declared final bundle state.

## Final authority

The bundle stores and revalidates:

- final population-structure digest;
- final metapopulation-snapshot digest;
- final demographic-history-cursor digest.

For an empty bundle these are exactly the root authorities.

## Canonical bundle identity

The bundle digest binds:

- proof-bundle model/version;
- root authorities;
- ordered step count;
- for every step in vector order:
  - execution-evidence kind tag;
  - event declaration digest;
  - structure-transition digest;
  - successor-structure digest;
  - execution-provenance digest;
  - result-snapshot digest;
  - result-cursor digest;
- final authorities.

Step order is causal information and is never canonicalized as a set.

Serde/JSON encoding is transport only and is not semantic identity.

## Fail-closed staged chaining

B4A intentionally demonstrates this attack rejection:

1. create root-valid receipt A;
2. independently create root-valid receipt B against the same unchanged biological state;
3. place A then B in a vector;
4. attempt to declare the vector as a two-step history.

The bundle must reject it because B's predecessor cursor is the root rather than A's successor cursor.

This remains true even when A was a biological no-op and therefore source/result snapshots are equal.

## B4B successor

B4B should add a non-serializable or otherwise non-forgeable-by-construction validated-predecessor token minted only by successful proof-bundle replay. That token should bind at least:

- exact schema digest;
- exact current structure digest;
- exact current snapshot digest;
- exact current history-cursor digest;
- experiment/generation;
- proof-bundle digest or equivalent replay authority.

Executors may then accept either:

- direct ordinal-zero root authority; or
- a current source cut plus an exact validated-predecessor token.

A raw non-root cursor remains insufficient.

## Storage boundary

V0 proof steps may embed full aggregate execution results for direct audit/replay. Same-generation intervention chains should be short.

This bundle is not the billion-year history store. Long deep-time histories should later checkpoint/content-address these bundles through Symtropy's persistence/causal-ledger infrastructure.

## Non-goals

No world-time authority, global event-log replacement, signature/consensus claim, cross-generation ancestry, arbitrary plugin execution variant, conflict-resolution scheduler, ecology, selection, or speciation is introduced here.
