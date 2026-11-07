# Aircraft mass-properties module — v0.1

**Module:** `src/mass_properties.rs`  
**CLI:** `mass_properties_audit`  
**Parent research contract:** [AER-01 whole-aircraft closure #3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945)  
**Underlying civil-aero implementation:** [AERO-CIVIL-001 #2878 / draft PR #2880](https://github.com/Luminous-Dynamics/symtropy/pull/2880)

## Scope

This increment adds a dependency-free Rust mass ledger that computes:
- total mass from itemized installed components;
- centre of gravity from their aircraft-frame locations;
- a symmetric inertia tensor about the computed total-aircraft CG;
- rigid-body component inertia translated with the parallel-axis theorem;
- explicit unresolved states for unknown mass or unknown location;
- rejection of duplicate item IDs, invalid units/values, malformed configuration digests and stale configuration reuse.

The module supports point-mass and rigid-body entries. Tensor field order is `[Ixx, Iyy, Izz, Ixy, Ixz, Iyz]`, with the off-diagonal terms stored as the actual symmetric matrix entries.

## Critical fail-closed semantics

- `mass_kg = None` means unknown mass, never zero.
- Known mass with unknown geometry permits a known total mass, but withholds CG and inertia.
- Unknown mass withholds the complete total, CG and inertia.
- A partial known subtotal is diagnostic only and must not be used as aircraft mass.
- Every report is labelled `screening_only`.
- The configuration SHA-256 check verifies syntax and equality with the requested identity. It does not authenticate the upstream manifest or the physical correctness of its contents.

A complete report means only that the supplied finite ledger has enough internally consistent numbers for the stated arithmetic. It is not an approved weight-and-balance schedule, flight-envelope assessment, loads model, structural adequacy finding, airworthiness determination or permission to fly.

## Data and independent replay

Three exact-schema CSV fixtures are provided:
- `data/mass_properties/point_mass_fixture.csv` — nine synthetic point masses with hand-calculable mass/CG/inertia.
- `data/mass_properties/unknown_mass_fixture.csv` — the same ledger plus an unknown cooling-system mass; the complete result must remain indeterminate.
- `data/mass_properties/rigid_body_fixture.csv` — a rigid body with local inertia plus a point mass, checking parallel-axis translation.

Run the Rust audit:

```bash
cargo run --locked --manifest-path tools/civil-aero-benchmark/Cargo.toml \
  --bin mass_properties_audit -- \
  tools/civil-aero-benchmark/data/mass_properties/point_mass_fixture.csv
```

The independent standard-library Python oracle recomputes mass, CG and inertia from the raw fixture rather than calling Rust production code. It is a finite synthetic test oracle, not an independent engineering certification.

A dedicated CI workflow checks formatting, unit/negative-control tests, strict Clippy, deterministic CLI replay, all three fixtures, independent Python agreement, and rejection of mixed configuration identities.

## What is not integrated yet

The existing steady-level mission-energy `Case` model remains a screening approximation using its existing airframe-mass plus payload fields. This mass-properties module is not yet coupled into that mission evaluator, hybrid dispatch, structural loads, trim, control or aircraft-level MDO. The next integration must bind the exact mass-ledger digest to the mission/configuration request and prove that mass changes propagate into mission demand and all downstream discipline identities. It must not silently substitute partial subtotal mass.

## Next engineering step

1. Review and qualify the CSV/module interface and independent oracle.
2. Add a mission adapter that consumes only a complete mass-properties report and preserves its exact configuration digest.
3. Connect #3900 phase-resolved dispatch and #3903 installed battery evidence without cloning their models.
4. Add moving fuel/payload state snapshots, so takeoff, landing and contingency CG/inertia are recomputed from each exact state.
5. Compare sequential sizing and coupled MDF/IDF or other formulations only after the adapter has passed its own finite oracle.

**Claim ceiling:** The module and fixtures verify bounded arithmetic and input/reuse safeguards on synthetic examples only. No real aircraft mass, CG, inertia, mission performance, stability, structural loads, safety or airworthiness claim is made.
