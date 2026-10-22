# Plant Damage and Repair V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how structural damage becomes persistent canonical plant history instead of a temporary visual decal, and how repair/remodeling may occur without silently restoring pre-damage structure.

## Core invariant

> Damage mutates biological structure first; rendering observes the damaged structure.

A branch may not be canonically severed because a mesh fragment disappeared, and reloading/re-LODing may not heal damage.

## Damage targets

Damage addresses stable biological structure coordinates, not Bevy entities, mesh indices, bone indices, or triangles.

Candidate target classes include:

- root segment;
- stem/trunk segment;
- branch segment;
- bud/meristem;
- leaf/foliar cohort;
- reproductive organ;
- bark/cambial region;
- attachment/junction.

## Damage state

A structural element may retain future-bearing state such as:

- integrity;
- wound/open-surface state;
- hydraulic/transport impairment;
- infection susceptibility;
- cambial continuity;
- attachment strength;
- dead/live status;
- breakage cause/provenance;
- repair/remodel state.

Exact fields are implementation-specific and versioned.

## Breakage transaction

Canonical breakage must be derived from qualified physical/ecological authority such as load, material resistance, damage, excavation/cutting, browsing, fire, or another explicit cause.

Conceptually:

`qualified cause + current structural state -> breakage plan -> validate topology/resource consequences -> atomic structural commit -> derived debris/presentation`

If breakage transfers matter from living tissue to detached living/dead stock, that stock transfer must settle atomically with the topology mutation when exact accounting is enabled.

## Topology invariants

After any committed damage operation:

- every live structural node has a valid canonical parent/path or an explicitly supported detached state;
- removed children cannot remain reachable as live attached structure;
- structural identifiers are never silently reused for a different organ;
- cached presentation geometry can be rebuilt solely from canonical structure;
- a stale damage command cannot target a newly created element through identifier reuse.

## Repair is not rewind

Repair/remodeling creates new biological history.

A wound may:

- compartmentalize;
- callus over;
- sprout replacement shoots;
- strengthen adjacent tissue;
- remain a weak point;
- become infected;
- fail to repair.

Repair does not erase the original damage event or reconstruct an impossible pristine past unless a species/content process explicitly models regeneration capable of doing so.

## Growth interaction

R2/R3 developmental state and structural damage interact through future growth actions.

Examples:

- loss of apical dominance may activate lateral buds;
- repeated browsing may create a persistent architecture;
- storm damage may bias later crown rebuilding;
- root injury may reduce future shoot growth and increase drought sensitivity;
- cambial damage may generate persistent scar morphology.

These are consequences of retained state, not shader randomization.

## Collapse / persistence

A damaged plant may collapse to a coarse representation only if that representation retains all damage information needed by every enabled future process.

If exact scar topology, missing limb identity, local transport impairment, or individual-specific history matters and cannot be represented coarsely, collapse is C1/C2/C3 under the Living World collapse-admissibility contract.

## Presentation

Rendering may derive:

- broken surfaces;
- exposed wood;
- bark tears;
- scars;
- dead limbs;
- fungal colonization cues;
- debris;
- asymmetrical crown recovery.

But presentation cannot heal, infect, sever, or regrow canonical structure directly.

## Qualification direction

At minimum test:

1. severing an internal branch removes/detaches the correct canonical subtree;
2. stale element references fail closed after topology mutation;
3. render teardown/rebuild preserves identical canonical damage state;
4. save/reload preserves wound/repair trajectory;
5. identical hereditary plants with and without one early pruning event diverge structurally in a deterministic fixture;
6. repair consumes required resources before new structure appears;
7. exact matter transfer, when enabled, reconciles across living/detached/dead stocks;
8. damaged plants cannot collapse into a coarse state that would erase future-relevant injury information.

## Non-goals

This contract does not define a universal fracture mechanics solver, tree surgery model, pathogen model, fire model, or artistic wound shader.