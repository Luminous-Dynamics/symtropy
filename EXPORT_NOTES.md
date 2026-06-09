# Symtropy Public Snapshot Export

Date: 2026-06-09

Source:

- Private monorepo: `/srv/luminous-dynamics`
- Source ref: `rhn/v0.10-cost-aware-metrics`
- Exported subtree: `symtropy/`

Scope:

- Reusable N-dimensional math, physics, geometry, simulation, rendering, terrain, networking, and demo substrate.
- Includes the broader current Symtropy workspace rather than only the older `symtropy-consciousness-physics` research crate.
- The initial checked public workspace is intentionally smaller than the full source tree and includes only self-contained substrate crates.

Excluded:

- Symthaea cognitive architecture and Broca internals.
- Mycelix governance/civic infrastructure.
- Private funding/outreach files.
- Local session artifacts and dirty workspace files.
- Root game launcher files and Mycelix/Symthaea integration demos that still depend on private-monorepo sibling crates.

Notes:

- This is a curated subtree snapshot, not a raw merge of the private monorepo lineage.
- Public `Luminous-Dynamics/symtropy-consciousness-physics` remains a narrower historical/research repository.
- This repository is intended to become the canonical public Symtropy math/physics substrate.
- The root `Cargo.toml` has been publicized as a virtual workspace for self-contained substrate crates only. Private-adjacent launchers/demos remain in-tree but outside the public workspace until their Symthaea/Mycelix path dependencies are split or replaced.
- `symtropy-render-bridge` also remains outside the initial public workspace because its current Bevy/Winit feature set is not CI-safe in the fresh public snapshot. It should be reintroduced after its Linux windowing/render features are made explicit.
