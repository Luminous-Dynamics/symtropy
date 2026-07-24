---
title: Earth Atlas Implementation Debt Patch README v0.2
version: 0.2
scope: Earth Atlas implementation debt package navigation
owner: documentation/world-design
status: supporting
package_role: readme
project: Symtropy
domain: Earth Atlas / Implementation Hardening
recommended_path: docs/earth-atlas/EARTH_ATLAS_IMPLEMENTATION_DEBT_PATCH_README_V0_2.md
extends:
  - EARTH_ATLAS_HARDENING_PACK_README_V0_1.md
  - WORLDLINE_MECHANICAL_DELTA_SCHEMA_V0_1.md
---

# Earth Atlas Implementation Debt Patch v0.2

## Purpose

This package resolves the next layer of implementation debt identified after the Earth Atlas Hardening Pack v0.1.

The Hardening Pack proved the thesis:

```text
The Earth Atlas becomes real when the map changes what the player can do.
```

This patch answers the next question:

```text
Can the team build those changes in sequence without hidden design ambiguity?
```

## Design Debts Resolved

1. **Ghost Mine continuity ambiguity**  
   The Choked Valve Court needs a clear answer: is the Ghost Mine a concurrent problem, a completable objective, or both?

2. **Chronicle overuse without a core spec**  
   Many docs rely on Chronicle consequences, but the Chronicle needs a minimum viable event structure, reader model, effect model, and UI rule.

3. **Field Deck overlay precedence**  
   Cultures now suppress and prioritize modes. The system needs conflict resolution when overlays overlap.

4. **Green Cover Lie resolution**  
   The toxic-remediation mission seed needs a mechanic for exposing failure without discrediting ecological repair.

5. **Road Choir mobile accountability**  
   Stopping rights are specified, but accountability for harm caused by mobile actors needs enforceable mechanics.

6. **Peninsula Refuge coalition dynamics**  
   The culture has strong internal factions, but their coalition behavior needs to generate emergent conflict.

7. **Antarctic xeno-contact pressure semantics**  
   `xeno_contact_pressure` now has a named source in Antarctica. The atlas needs rules for low, medium, and high values elsewhere.

8. **Subglacial Listening Fault vertical slice**  
   The xeno-contact hook needs a playable mission equivalent to the Choked Valve Court.

## Included Docs

```text
CHOKED_VALVE_COURT_GHOST_MINE_CONTINUITY_PATCH_V0_2.md
CHRONICLE_MVP_SPEC_V0_1.md
FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
GREEN_COVER_LIE_RESOLUTION_MECHANIC_V0_1.md
ROAD_CHOIR_MOBILE_ACCOUNTABILITY_MECHANICS_V0_1.md
PENINSULA_REFUGE_COALITION_DYNAMICS_PATCH_V0_1.md
XENO_CONTACT_PRESSURE_SEMANTICS_PATCH_V0_1.md
SUBGLACIAL_LISTENING_FAULT_VERTICAL_SLICE_V0_1.md
```

## Build Order Recommendation

```text
1. Chronicle MVP Spec
2. Field Deck Overlay Precedence
3. Choked Valve Court Ghost Mine Patch
4. Green Cover Lie Resolution
5. Road Choir Mobile Accountability
6. Peninsula Coalition Dynamics
7. Xeno Contact Pressure Semantics
8. Subglacial Listening Fault Vertical Slice
```

Why this order:

```text
Chronicle and Field Deck rules are shared infrastructure.
The Choked Valve Court is the first buildable slice.
The Green Cover Lie and Road Choir accountability extend Southern Africa.
Peninsula dynamics and xeno semantics prepare Antarctica.
The Subglacial Listening Fault vertical slice comes last because it depends on Chronicle, Field Deck, xeno-pressure, and Machine Archive behavior.
```

## Package Mantra

```text
A great worldbuilding idea is not finished until its ambiguity becomes playable.
```
