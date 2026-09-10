# HEALTH-01 — residual effect to condition proposal

HEALTH-01 closes the authority gap between generic protection and persistent person health without making either domain depend on the other.

The bridge implements:

`residual effect + semantic contact + explicit body-model rule -> ConditionProposal`

A proposal is **not** an authoritative condition. The owning world/health integration must explicitly materialize and insert the proposal into the correct person's `ConditionSet`.

## Rules and specificity

Each `EffectConditionRule` is versionable by stable rule identity and binds:

- an effect kind;
- either one exact semantic protection/body region or an explicit wildcard;
- a positive reference magnitude;
- a maximum fixed-point severity;
- a condition kind;
- declared capability effects.

Exact region rules take precedence over wildcard rules. Duplicate rule keys fail closed. Unknown effect/region pairs fail closed instead of silently producing a generic injury.

The explicit `reference_magnitude` is the quantization boundary between protection's abstract floating-point effect scale and health's deterministic 0..10,000 severity representation. It is a game/body-model policy parameter, not a medical claim.

## Authority boundaries

- Physics/contact integration owns where the effect arrived.
- Protection owns how the incoming effect was transformed and what residual remains.
- The body-model provider owns effect-to-condition mapping rules.
- The bridge returns a proposal only.
- The health-domain `ConditionSet` remains authoritative for committed persistent conditions.
- Exoframe assistance, shield recharge, AI belief, Muse state, and presentation cannot commit or erase a condition through this bridge.

## Multi-body support

There is deliberately no universal human injury table. Separate worlds/body archetypes can supply different rule sets for humans, animals, fictional species, cybernetics, robots, or other embodied agents while sharing the same protection and capability machinery.

## Evidence boundary

This is deterministic game/simulation semantics. It is not medical diagnosis, treatment guidance, injury prediction, or a real weapon/armor model.

## Qualification

Implemented/static only until exact-head formatting, Cargo check/test/Clippy, serialization review, and lockfile validation run successfully.