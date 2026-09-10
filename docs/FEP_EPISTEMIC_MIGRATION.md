# FEP epistemic migration

The live launcher FEP model currently uses six observation dimensions. FEP-03 preserves that dimensional contract while removing one invalid cross-person information flow: NPC cognition no longer receives the player's private biometric arousal.

The six slots are now constructed explicitly as:

1. NPC self energy fraction;
2. NPC self allostatic load;
3. perceived danger;
4. NPC self caution;
5. perceived water availability;
6. perceived power availability.

FEP-03 changes only slot 2's provenance. The current danger/water/power values still come from legacy authoritative world resources. They are named `perceived_*` at the construction boundary to make the intended endpoint clear, but this PR does **not** claim they are epistemically scoped yet.

The next migration should replace those values with estimates derived from observer-scoped claims/sensors while preserving the six-dimensional model until evidence supports changing the FEP configuration.

## Rules

- one person's private biometrics are not another agent's perception;
- self-state may be supplied directly when it is genuinely internal to that agent;
- world state should reach cognition through observation/belief projections, not arbitrary ECS/resource reads;
- migration should be channel-by-channel with deterministic regression tests rather than a single behavior rewrite;
- the existing authored archetype/name logic remains separate technical debt and is not silently conflated with epistemic migration.

## Qualification

Implemented/static only until exact-head FEP-feature compilation/tests and relevant gameplay regression checks execute successfully.