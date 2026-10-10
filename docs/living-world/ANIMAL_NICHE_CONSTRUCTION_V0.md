# Animal Niche Construction V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how animals physically and ecologically modify habitat so the world is not merely a container for fauna.

## Core invariant

> Canonical animal actions may create persistent environmental consequences through explicit ecological transactions; renderer effects cannot substitute for those consequences.

## Candidate niche-construction effects

Depending on species and action, animals may alter:

- vegetation biomass/height through grazing/browsing;
- seed dispersal and recruitment opportunity;
- nutrient deposition through waste/carrion;
- soil disturbance, compaction, aeration, and burrowing;
- trails/path usage;
- nest/den/rookery structures;
- woody debris through gnawing/breakage;
- hydrology through dams/channels/wallows;
- pollination/reproductive success;
- prey spatial distribution through predation pressure;
- scavenging/decomposition access;
- disease/parasite transmission;
- local signal/odor landscape.

## Action -> settlement -> habitat

Conceptually:

`canonical animal action -> ecological effect intent -> validate/scarcity/stock settlement -> habitat/structure mutation -> derived fields -> future organisms`

An animation of grazing or digging does not remove vegetation or soil unless the canonical ecological transaction commits.

## Avoid hardcoded species pairs

Where possible, niche construction composes through capabilities and environmental affordances rather than pairwise code paths.

Examples:

- grazer capability + edible vegetation affordance;
- burrow capability + diggable substrate;
- pollinator capability + compatible floral resource;
- dam-building capability + movable material + flow geometry.

Specialized species behaviors can extend the generic affordance substrate.

## Conservation

Where niche construction transfers exact material stock, use the Living World stock/settlement architecture.

Examples:

- consumed vegetation moves into animal/metabolic/waste pathways rather than disappearing;
- deposited waste enters appropriate stock/reaction paths;
- moved woody material remains accounted for when exact stock authority is enabled.

Derived habitat fields do not mint/destroy exact stocks.

## Spatial memory

Persistent modifications remain after the animal leaves when the physical/ecological process says they should.

A burrow, trail, dam, grazed patch, dung deposit, or disturbed soil cannot vanish merely because the actor becomes offscreen.

## Trophic cascades

Indirect consequences may emerge through shared habitat/processes rather than scripted chains:

`predator pressure -> prey distribution -> grazing pressure -> vegetation structure -> habitat/light/soil effects`

These feedbacks should be testable at coarse ecological fidelity without requiring every animal to remain individually active.

## Qualification direction

At minimum test:

1. canonical grazing reduces the authorized vegetation/resource state, not just its mesh;
2. animal unload does not erase persistent habitat modification;
3. repeated coarse vs active niche construction preserves declared stock/spatial observables;
4. derived fields cannot double-apply the same canonical habitat mutation;
5. capability/affordance mismatch fails closed;
6. finite substrate/resource prevents unlimited construction/consumption;
7. a simple predator-pressure fixture can alter prey/grazing spatial statistics without hardcoded vegetation rewrite;
8. save/reload preserves persistent constructed/disturbed habitat state.

## Non-goals

This contract does not prescribe detailed beaver engineering, burrow meshing, pollination biology, animal digestion chemistry, or universal trophic coefficients.