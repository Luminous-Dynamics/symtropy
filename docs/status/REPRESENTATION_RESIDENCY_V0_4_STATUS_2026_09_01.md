# Representation Residency v0.4 — Status

**Date:** 2026-09-01  
**Branch:** `world/cuf-v0.4-representation-residency`  
**Stacked on:** `world/cuf-v0.3-adaptive-fidelity` / draft PR #63  
**Status:** authored, release-gating layer, **not yet locally qualified**

## Landed on the branch

- `ActiveRepresentation` binding authority/scope/representation to current state digest;
- deterministic `RepresentationLease` with simulation-time minimum residency;
- domain-owned `RepresentationReleasePermit` bound to the exact current state digest;
- `ResidencyGate` with explicit `Retain`, `AwaitDomainPermit`, and `TransitionPermitted` outcomes;
- rejection of mismatched leases/permits;
- rejection of pre-issued release permits;
- rejection of future-dated permits;
- stale-permit rejection after authoritative state changes;
- no representation ordering inferred from identifiers;
- canonical Representation Residency Contract v0.4;
- fail-closed `scripts/qualify-cuf-v0.4-stack.sh`.

## Core safety property

Lease expiry does **not** release detail. It merely makes a domain release review eligible.

Even after the residency threshold, the common layer returns `AwaitDomainPermit` until the owning domain supplies evidence bound to the exact currently active state.

A permitted transition is still not the transition itself. The owning domain remains responsible for any representation transfer and for the `RepresentationTransferReceipt` / conservation-equivalence evidence required by v0.1.

## Why this matters for environmental simulation

Current domain ownership is already split across dedicated crates:

- `symtropy-terrain` owns volumetric `EarthChunk` ground state;
- `symtropy-fluid` owns SPH fluid state;
- `symtropy-basin` owns basin-scale water/soil/atmosphere/heat/toxin/ecological state;
- `symtropy-lifesim-core` owns field-based living-system primitives.

The next environmental tranche should connect these authorities through typed observations and causal transformation evidence rather than copying them into a new shared mutable environment struct.

## Qualification boundary

The connected authoring environment still does not provide the repository Rust/Nix toolchain, so no compile/test result is asserted.

Preferred full/private-monorepo gate:

```bash
nix develop --command bash scripts/qualify-cuf-v0.4-stack.sh
```

The script fails closed when the private `../mycelix-multiworld-sim` sibling required by `symtropy-world` is absent.

## Recommended next tranche

Build an environmental evidence bundle that can synchronize digest-bound Terrain/Hydrology/Climate observations at one body-cell scope and simulation instant, then add a Basin ingest boundary that records which source evidence causally produced a basin update. Do not make the bundle authoritative and do not copy mutable domain state into `symtropy-world`.
