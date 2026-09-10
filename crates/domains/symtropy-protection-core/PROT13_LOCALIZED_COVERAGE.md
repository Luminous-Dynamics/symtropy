# PROT-13 — localized directional coverage

PROT-13 adds semantic contact coverage to the generic protection stack without moving collision, targeting, anatomy, or world geometry into `symtropy-protection-core`.

The owning physics/body/vehicle/structure integration resolves a contact into:

`ProtectionContact { region, approach }`

The protection domain then answers only:

`which configured protection layers cover that contact, and how much of the already-resolved effect do they transform?`

## Authority boundaries

- Physics/collision owns where an effect contacted and the approach direction.
- A body/vehicle/structure model owns the meaning and validity of semantic region IDs.
- Protection owns layer coverage and effect transformation.
- Active fields own fictional charge/thermal state but do not own contact geometry.
- Health owns neither protection nor collision; it consumes only later residual-effect mapping results.
- AI does not receive hidden coverage or charge state merely because these structures exist.

`CoverageMask` can constrain regions, approach sectors, both, or neither. Empty region/sector sets mean unrestricted coverage on that dimension.

Uncovered layers are skipped without consuming their capacity. A directional active field likewise leaves its charge and thermal state unchanged when a contact bypasses its configured coverage.

## Why semantic regions

A semantic region is deliberately not a human anatomy enum. The same contract can represent `body:chest`, `vehicle:engine-bay`, `habitat:pressure-wall-3`, or `frame:left-knee-emitter`. This keeps the generic protection domain reusable and prevents a combat-specific body schema from becoming engine authority.

## Evidence boundary

All field and protection values remain fictional/game-simulation semantics. PROT-13 does not claim real armor, force-field, injury, or weapon performance.

## Qualification

Implemented/static only until exact-head formatting, Cargo check/test/Clippy, serialization review, and lockfile validation run successfully.