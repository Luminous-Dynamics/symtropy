# symtropy-spatial-topology — PB-04a

Status: **synthetic proof crate; implemented/static, not executable-qualified**

This crate is the first implementation tranche for PB-04 / #455. It proves the
smallest reusable idea before any real geometry adapter exists:

> one exact boundary snapshot can project into several independent topology
> facets without inventing one universal notion of “room connectivity”.

## What PB-04a owns

Only deterministic read-only projection structure:

- exact source references supplied by other authorities;
- externally decomposed region snapshots;
- interfaces between regions;
- independent typed facet relations;
- deterministic facet graphs;
- bounded/canonical aggregate validation.

Initial facets are:

- occupancy/access;
- air/pressure;
- acoustics;
- visibility;
- thermal;
- weather exposure.

## What it explicitly does not own

PB-04a does **not** own or claim:

- geometric decomposition;
- meshes/voxels/CSG/BREP;
- door/window/hatch/device authority;
- navigation/path planning;
- CFD/atmosphere/pressure simulation;
- acoustics propagation;
- thermal simulation;
- renderer visibility;
- weather simulation;
- structural physics;
- privacy;
- room-use semantics;
- `PlaceIdentity` / home association;
- any physical mutation.

`FacetRelation::QualifiedClass` is deliberately opaque. A class such as
`attenuation:door-closed` says only that the source projection supplied that
qualified relation class. PB-04a does not reinterpret it as a numerical dB,
permeability, conductance, or safety value.

## Why a separate proof crate is justified

Spatial topology is expected to have genuinely different consumers: habitation,
AI/navigation acceleration, atmosphere/pressure, audio, thermal projection,
weather exposure, vehicles, and orbital habitats. Keeping the read-only topology
vocabulary independent avoids forcing any one consumer’s authority semantics onto
the others.

The crate is temporarily an **isolated nested Cargo workspace**. It is not yet a
root Symtropy workspace member, deliberately avoiding root lock/resolution churn
while the main Fabrication/Construction/PB-01 stack is moving. Promotion into the
root workspace is a separate integration decision after executable qualification.

## Current deterministic fixtures

Static tests cover:

- region/interface insertion-order independence;
- closed door blocks body passage without implying perfect air/sound isolation;
- window blocks occupancy while preserving visibility and a distinct thermal relation;
- vent connects the air facet without body passage;
- open/closed door exact source state changes the topology;
- facet-profile selection does not mutate the boundary snapshot;
- same-authority/same-subject/same-revision competing digests fail closed;
- interfaces referencing unknown regions fail closed;
- graph neighbor queries are deterministic and symmetric.

These are synthetic topology fixtures. They do not prove the regions themselves
were correctly extracted from physical geometry.

## Next PB-04 tranches

PB-04b should add a versioned geometric decomposition contract and the **crooked
shelter** synthetic geometry proof. It should prefer deterministic quantized or
robust predicates where topology identity depends on geometry and must explicitly
bind tolerance/profile identity.

PB-04c should add dirty-region incremental recomputation and prove its output is
identical to a clean full recomputation for the same exact source/profile.

PB-04d should adapt one *qualified* realized-geometry source. The adapter remains
read-only and must re-resolve source revisions/digests rather than accepting stale
snapshots as current.

PB-04e should run the two-compartment pressure-habitat anti-overfitting proof.
It proves only pressure-connectivity topology representation, not accurate
transient decompression physics.

## Evidence gate

Before calling PB-04a executable-qualified, run pinned Rust against the exact
product head and record at least:

```text
cargo fmt --manifest-path crates/domains/symtropy-spatial-topology/Cargo.toml -- --check
cargo check --manifest-path crates/domains/symtropy-spatial-topology/Cargo.toml --all-targets
cargo test --manifest-path crates/domains/symtropy-spatial-topology/Cargo.toml
cargo clippy --manifest-path crates/domains/symtropy-spatial-topology/Cargo.toml --all-targets -- -D warnings
```

Also require a clean tracked source tree after execution. A queued run or this
README is not evidence.

References: #455, #449, #491, #492, #497.
