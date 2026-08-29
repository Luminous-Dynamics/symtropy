# Reality Ledger v1.1 — Symtropy transactional world adapter

Date: 2026-08-29

## Purpose

Bind the already-qualified v1D four-ghost world mechanics to the host-neutral
Symthaea Reality Ledger without letting the renderer mint authority or blur
committed and counterfactual worlds.

## Source boundaries

This branch starts from the qualified v1D Symtropy source head:

- HEAD `a00fc96245a06509a8c8e387ac19280b145b91ac`
- TREE `1881607e4525d9c50f40fffbcb9ac527b9eb8ec8`

The optional `reality-ledger-adapter` feature pins the host-neutral
`symthaea-reality-ledger` dependency to exact construction commit
`ffa27ea1a0fa2bac69df6008adfdd2167b8e29c0` from branch
`world/reality-ledger-v1.2`. That dependency is a new, unqualified lineage and
must be qualified together with this adapter.

## Mapping

Committed baseline:

```text
Symtropy committed studio
  -> RealityLayer::DigitalCommitted
  -> WorldOrigin::DigitalHost(bevy/symtropy)
```

Each proposal ghost:

```text
proposal branch
  -> distinct WorldDescriptor
  -> RealityLayer::Counterfactual
  -> parent = committed studio world
  -> relation = CounterfactualOf
```

A rendered candidate is admitted only when its existing four-ghost contract is
valid and its GPU capture carries a non-empty artifact digest. The adapter then
creates a typed `WorldObservationBundle` bound to:

- world ID + lineage;
- base revision;
- deterministic StudioFrame;
- rendered semantic scene-state digest;
- stable camera identity;
- exact render-fidelity identity;
- one explicit render plane for the artifact receipt.

This first adapter intentionally requires one artifact per observation plane.
Color/depth/object-ID fusion should be performed by adding independently
validated receipts to one `WorldObservationBundle`, not by pretending one blob
is several planes.

## Materialization

Selection still does not grant mutation authority. For a selected proposal the
adapter can construct a `TypedCounterfactualCommitReceipt` only after an
external authority-receipt digest is supplied. The typed source scene-state
must equal the typed committed after-state; equal-looking hashes in different
domains cannot satisfy the gate.

## Non-goals

This adapter does not:

- make a counterfactual committed;
- grant `ArtPort` mutation authority;
- turn dream worlds into counterfactuals;
- establish physical grounding;
- establish consciousness or subjective presence;
- scalarize artistic value.
