# symtropy-fauna-core

A dependency-light Rust core for evidence-aware animal capability profiles and
taxonomy-scoped coverage reporting.

## Current scope

- Typed animal-profile and capability-claim values.
- Separate dispositions for eligible, indeterminate, and rejected profiles.
- Explicit source permissions, attribution, taxon identity, developmental-stage
  matching, and synthetic-versus-real evidence boundaries.
- Duplicate-ID and duplicate-capability rejection.
- Canonical, order-independent SHA-256 digests.
- Coverage reports pinned to a caller-supplied taxonomy source/release/scope.
- Frozen synthetic-only profile-oracle cases in
  `fixtures/animal-capability-profiles-v1.json`.

This is not an animal simulator, species database, behavioral policy, taxonomic
authority, or biological validation claim. The 26 archetype profiles and oracle
values are synthetic. A `covered` taxon must have a matching eligible real-taxon profile; omissions
stay `not_reviewed`. This is **profile-registry coverage**, not proof that the
species' anatomy, behavior, physiology, or ecology is comprehensively simulated.

## Focused qualification

From the repository root:

```sh
cargo fmt --check --manifest-path crates/domains/symtropy-fauna-core/Cargo.toml
cargo test --locked --manifest-path crates/domains/symtropy-fauna-core/Cargo.toml
cargo clippy --locked --manifest-path crates/domains/symtropy-fauna-core/Cargo.toml --all-targets -- -D warnings
```

These commands are proposed gates; they count as passing only when executed
successfully against the exact commit under review. Queued or skipped checks are
not a pass.
