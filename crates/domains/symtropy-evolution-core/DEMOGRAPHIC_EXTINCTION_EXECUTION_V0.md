# Demographic Extinction Execution V0

Status: implemented/static; not executable-qualified.

## Purpose

This contract defines deterministic aggregate-fidelity execution for `Extinction`.

Its two central identities are:

`extinction != census zero`

and

`removed live state != erased history`.

An extinct population is absent from successor live population and trajectory maps. Its final validated genetic-state and trajectory-point digests remain permanently bound into the extinction execution receipt.

## Required authority

Execution requires the exact current hereditary schema, source population structure, explicit successor population structure, source population states and trajectory points, source metapopulation snapshot, ordinal-zero demographic intervention cursor, `Extinction` event declaration, and demographic structure-transition receipt.

The successor structure must already satisfy the event-specific membership theorem: it removes exactly the population named by the event. V0 does not support an empty aggregate successor structure.

## Timing

Extinction is an instantaneous generation-G intervention before reproduction.

It does not advance biological generation time. The result snapshot remains in the same stochastic experiment and generation.

## Reference execution semantics

The executor:

1. identifies the exact population named by the event;
2. captures its final `PopulationGeneticStateDigest` and `PopulationTrajectoryPointDigest`;
3. removes that population from live population and trajectory maps;
4. leaves every other population state and trajectory point exactly unchanged;
5. captures the successor metapopulation snapshot under the explicit successor structure;
6. emits a deterministic extinction execution receipt and advances demographic history.

No random draw occurs in this reference operation. The event states that the population ceased to exist; it does not model the mortality process that caused that fact.

## Historical retention

The execution receipt preserves the extinct population's final live-state and trajectory digests even though the live objects are no longer present in the successor maps.

This is a historical anchor, not a fossil record. A later fossil/taphonomy authority may refer to extinction history, but extinction itself does not imply preservation, discoverability, sedimentary context, or any physical remnant.

## Result authority

`DemographicExtinctionExecutionResult` contains the complete post-extinction population map, trajectory-point map, successor-structure snapshot, successor demographic history cursor, and `DemographicExtinctionExecutionProvenance`.

The provenance binds:

- exact extinction execution model;
- event declaration digest;
- demographic structure-transition digest;
- source and successor structure digests;
- source and result snapshot digests;
- source history-cursor digest;
- experiment and generation;
- extinct population identity;
- final live population-state digest;
- final live trajectory-point digest.

Restored provenance is evidence-shaped data until `validate_current(...)` revalidates all current source/successor authority, checks the affected identity back against the event, deterministically repeats the removal, and verifies the successor history cursor.

## Empty-biosphere boundary

`PopulationStructureProfile` V0 requires at least one population. Therefore extinction of the sole remaining population cannot be represented by a census-zero pseudo-population or an empty structure.

Whole-biosphere extinction needs a future higher-level life-presence/world authority that can represent `no extant populations` explicitly. This executor must not weaken the population-structure invariant to simulate that case.

## Required invariants

V0 is intended to qualify:

- extinct population absent from successor state and trajectory maps;
- no census-zero state minted;
- all unaffected populations and trajectory points exactly unchanged;
- result membership exactly matches successor structure;
- final live state/trajectory digests preserved in provenance;
- receipt population identity equals the population named by the event;
- changed event/source snapshot/source structure/successor structure/structure transition invalidates authority;
- Serde-restored receipt requires deterministic revalidation;
- result remains at the same experiment and generation;
- root-only demographic cursor restriction remains in force;
- an empty aggregate successor structure remains unrepresentable in V0.

## Scientific limits

V0 does not establish the cause, duration, spatial pattern, or mechanism of mortality. It does not model individual deaths, corpses, decomposition, fossilization, ecological cascade effects, recolonization, extinction probability, chromosomes, ancestry, speciation, or world time.

Those are separate authorities whose outputs may causally reference this extinction event.

## Optimization rule

This operation is already deterministic and linear in the live map size. An optimization may change storage strategy but must preserve exact removal semantics, unaffected-state equality, last-state historical anchors, and the same authority/provenance checks.
