# Living Environment Integration Contract V0

Status: design/authority contract only. This document does not claim a runtime LENV implementation, production Terrain world embedding, qualified hydrology coupling, or a completed Living Valley.

## 1. Purpose

Symtropy already has distinct authorities for Terrain, water/hydrology, ecology, construction, signals, spatial topology, fauna and presentation. The Living Environment (LENV) contract defines how those authorities may compose into a history-bearing environment without introducing a second mutable copy of the world.

The target is a world in which physical and ecological history can alter future observations and behavior while remaining causally inspectable, persistent across admissible fidelity transitions, and unavailable to agents except through qualified evidence paths.

Core theorem:

```text
owning physical/ecological authorities
        -> exact/versioned observations
        -> process-specific interpretation/coupling
        -> bounded intent/effect proposal
        -> owner validation + settlement
        -> changed canonical world
        -> new observations / traces / future behavior
```

LENV is integration and evidence structure. It is not a monolithic `EnvironmentWorld`.

## 2. Authority firewall

LENV must not duplicate or silently replace existing authority.

At minimum:

| Fact / process | Owner or source family |
| --- | --- |
| realized Terrain geometry | qualified Terrain authority |
| Terrain exact metric | qualified Terrain metric vocabulary + authorized world binding |
| Terrain placement into an exact target frame | authorized spatial-embedding binding |
| substrate/material state | Terrain / Universal Matter |
| conserved water | one-water / hydrology authority |
| thermal energy / temperature truth | selected thermal authority |
| ecological stocks / vegetation state | Living World ecology |
| structural plant topology | structural-flora authority |
| organism identity / biography | Living World population / Level-I authority |
| chemical/acoustic/vibration signal traces | qualified signal authority |
| air/weather/acoustic/visibility/thermal connectivity | typed spatial-topology projections over exact sources |
| construction / destruction | owning physical Construction/Terrain authority |
| pre-atmosphere weather | explicit external forcing evidence |
| visual grass, water, debris, decals | presentation only |
| causal-history index | references over source evidence, never a replacement owner |

Convenient read access never transfers ownership.

A normalized field, renderer material, Bevy entity, mesh, collider, camera state, debug overlay or AI blackboard must not become canonical environmental truth merely because it is easy to query.

## 3. Exact source evidence before environmental convenience

LENV should reuse exact source receipts rather than rename them.

For qualified Terrain, a bounded geometry query receipt already binds the observed material cells and enclosing Terrain snapshot provenance. Exact Terrain metric vocabulary is a separate subject. Production physical placement is also a separate authority problem.

Therefore:

```text
valid Terrain geometry receipt
    != authorized metric for this world
    != authorized placement in a target frame
    != current multi-domain environmental context
```

A production Terrain contribution to a LENV observation context must consume the applicable qualified source evidence and authorization bindings. LENV must not derive them from `GlobalTransform`, renderer placement, mesh bounds, collider extents or convenient caller coordinates.

Synthetic/local-lattice fixtures may use explicitly test-authorized bindings, but they must not claim production world embedding.

## 4. Snapshot-consistent environmental observation

Cross-domain environmental reasoning requires explicit scope and currentness compatibility.

A consumer must not silently combine:

```text
Terrain@revision A
Water@revision B
Vegetation@revision C
Exposure@old geometry D
```

and call the result one current world state unless the source relationships are admitted by a qualified snapshot/consistency rule.

Conceptually a LENV observation context binds:

```text
spatial scope / frame relation
canonical time or interval
source authority refs
source revision / digest / continuation identity
freshness / hold / aggregate semantics
units and quantity kinds
evidence class
observation profile / applicability
```

Source enumeration order must not change context identity or sufficiency.

Missing evidence remains missing. A skipped or stale sample must not be silently replaced by current hidden world truth.

## 5. World facts are not body-relative affordances

Environmental facts and interaction estimates are separate semantic layers.

Examples of environmental facts:

- exact local geometry / slope support;
- material/substrate observation;
- water depth or moisture evidence;
- vegetation cover / obstruction;
- ice, snow or sediment where qualified;
- current exposure/topology.

Examples of body/process-relative estimates:

- horse footing confidence;
- dog paw support;
- human slip/sink envelope;
- wheeled-robot traversability;
- energetic traversal cost;
- stability margin.

The intended boundary is:

```text
qualified environmental evidence
+ body/contact capability
+ proposed action/load profile
+ explicit interaction model/profile
    -> faceted interaction estimate
```

No universal `terrain_friction`, `terrain_quality` or `is_traversable` scalar may stand in for every body and process.

Interaction estimates do not mutate Terrain or move an agent. They are evidence for downstream affordance/action decisions.

## 6. Effect proposals do not mutate owners

When an action may change the world, the causal path is explicit:

```text
qualified action/contact evidence
    -> effect proposal
    -> owner validates current source/context
    -> atomic/idempotent settlement
    -> new canonical owner state
    -> later observation changes
```

Preparing or rendering an effect is not settlement.

Stale source revisions, invalid scope, insufficient capacity, duplicate retry or conflicting authority must fail without partial mutation.

## 7. Environmental traces are owner-specific

There is no universal mutable `Trace` authority.

### 7.1 Physical traces

Terrain/material authority may own future-bearing physical consequences such as:

- footprints / hoofprints where physically represented;
- ruts / local displacement;
- compaction or constitutive ground state where modeled;
- excavation / erosion scars;
- persistent deformation.

### 7.2 Biological/ecological traces

Living World may own:

- trampled vegetation;
- browsed/grazed biomass;
- broken plant structure;
- deadwood / detritus;
- nests, burrows or dens where biologically represented;
- seed/nutrient/waste deposition;
- recovery state.

### 7.3 Signal / chemical traces

Signal authority may own:

- surface scent;
- airborne/local chemical evidence;
- pheromone/marking signals where supported;
- synthetic beacons or markers.

### 7.4 Constructed/material-history traces

Construction or another physical owner may own:

- repairs / patches;
- persistent debris;
- damage/breach state;
- constructed signs / navigation aids;
- wear/scars only when grounded in an owning process.

LENV may index references to these facts for explanation. The index is not their mutable truth.

## 8. No cause telepathy

The simulator may know the provenance of a trace without granting that provenance to an in-world observer.

For example:

```text
canonical hoofprint / scent exists
    -> receiver samples qualified local evidence
    -> raw percept
    -> uncertain interpretation
    -> belief / memory
```

must not collapse into:

```text
receiver learns exact horse identity and route because engine metadata knows it
```

Only evidence legitimately available to the receptor/model may cross the epistemic boundary.

Developer Observatory data is never an ANIMA sensor.

## 9. Niche construction

LENV must admit closed feedback in which behavior changes the world and the changed world alters later behavior.

Reference trail loop:

```text
repeated traffic
    -> owner-settled physical / vegetation disturbance
    -> changed future environmental observation
    -> changed body-relative affordance / route evidence
    -> changed later route choice
    -> further traffic
```

A `Trail=true` tag may be a derived analysis/presentation classification after causal state exists. It must not substitute for the causal process when the game claims emergent trail formation.

Other future niche-construction examples include grazing patches, nests, caches, burrows, damming/engineering species and synthetic markers. Each remains subject to its physical/ecological owner.

## 10. External forcing is not atmosphere authority

Until a qualified atmosphere/weather authority owns state and exchange, weather-like inputs remain explicit forcing evidence.

A V0 forcing record may carry, when available:

- precipitation;
- wind;
- air temperature;
- humidity;
- solar/irradiance;
- precipitation phase;
- source/profile/version;
- spatial and temporal scope.

Optional values may be absent.

Hydrology may admit precipitation as an explicit external water input. This does not imply an atmospheric reservoir existed or lost water.

Vegetation, traces, Terrain/material processes, ANIMA and presentation may consume forcing only through their explicit coupling models.

## 11. Local exposure and microclimate are projections

Local environment can differ under common external forcing because geometry, vegetation, water and topology differ.

Potential projection facets include:

- precipitation exposure;
- wind shelter;
- solar shade/exposure;
- thermal coupling;
- wet/dry persistence estimate;
- acoustic weather exposure;
- scent-transport context.

A local microclimate projection binds its exact source observations and model/profile. It does not become atmosphere authority.

A tree fall, wall breach, opening change, flood, canopy change or new roof must invalidate affected projections through exact source revision, not by cosmetic scene notification alone.

## 12. Path-dependent environmental state

Some processes may require future-bearing internal state that is not recoverable from current presentation alone.

Examples, only where a selected qualified profile requires them:

- ground compaction / structural state;
- cumulative disturbance;
- wetting/drying branch state;
- vegetation damage/recovery stage;
- erosion susceptibility;
- channel/bank morphology continuation;
- freeze/thaw history;
- repeated mechanical exposure.

The owning domain retains the minimum sufficient constitutive/recovery state.

LENV does not own generic hysteresis.

If a model is intentionally non-hysteretic, LENV must not add hidden path state merely in the name of realism.

## 13. State is preferred to an unbounded event log

Future processes should retain the minimum process-sufficient state rather than requiring every historical cause forever.

For example, a qualified route-wear process may consume cumulative traffic/disturbance state without retaining every anonymous contact.

A source-specific scent process may require more detailed provenance and therefore reject that same aggregate.

Information may be compacted only when enabled/future-bearing processes remain sufficient.

## 14. Recovery and decay are canonical processes

Environmental state disappears or recovers only through a declared process.

Examples:

- scent decays through signal policy;
- vegetation recovers through ecology;
- ground state relaxes through a selected material/recovery model;
- snow tracks disappear through snow/melt/accumulation authority;
- debris disappears only through physical/ecological/cleanup processes;
- scars are repaired/covered/replaced through owner state changes.

Unload, camera distance, renderer LOD, ECS despawn, memory pressure and save-file compaction are not recovery mechanisms.

## 15. Process-relative fidelity

LENV introduces no global environment LOD integer.

Every future-bearing datum/profile declares:

- owner;
- consuming processes;
- exact/derived/approximate status;
- persistence requirement;
- admissible coarse representations;
- preserved observables/invariants;
- information lost by collapse;
- promotion/refinement semantics;
- decay/recovery owner.

Fine-to-coarse collapse is permitted only when the destination remains sufficient for every enabled process that must continue.

Promotion may restore exact retained state, apply an explicitly nonhistorical refinement where semantics allow it, request higher-fidelity evidence, or fail closed. It may not fabricate discarded exact history.

## 16. Inactive-world continuation

A region that is not rendered or interactively loaded may continue only through processes whose selected representation is qualified for that interval and required information.

Possible examples:

- hydrology recession/routing under qualified closure;
- vegetation growth/recovery under qualified ecological closure;
- scent decay under qualified signal policy;
- decomposition settlement;
- no exact body-contact process when body/contact microstate is unavailable.

When no sufficient representation exists, the safe outcomes are explicit pause, refinement or failure—not arbitrary hidden evolution.

## 17. Save/reload equivalence

For every claimed exact/toleranced observable, the participating profiles should support the test:

```text
0 -> T2 uninterrupted
```

versus

```text
0 -> T1 -> save/reload -> T2
```

with equivalent declared world results and exactly-once settlements.

Forcing cursors, future-bearing trace state, constitutive/recovery state and fidelity/continuation identity must survive when required by future behavior.

Presentation pixels/audio need not be bit-identical unless a presentation subsystem independently claims that property.

## 18. Environment Observatory

The Observatory may index retained source evidence and typed causal relations without becoming authority.

Useful causal edge classes include:

- observed-from;
- proposed-from;
- settled-by;
- transferred-to;
- invalidated-by;
- accumulated-into;
- decayed/recovered-by;
- refined/collapsed-from;
- perceived-from;
- decided-from;
- presented-from.

The exact schema may evolve; distinct semantics must not be flattened into an undifferentiated parent graph.

## 19. Causal history cone

Given a retained event/evidence/state ref, tooling may traverse qualified outgoing causal edges to answer:

> Which later retained facts can be traced through known causal links to this intervention?

Graph reachability does not mean the initiating event is the sole sufficient cause.

Missing retained ancestry is reported as incomplete/unknown, never invented.

## 20. First-divergence differential

For two compatible scenario lineages:

1. verify compatible code/model/scenario identities;
2. verify common initial commitment when claimed;
3. align canonical time/event coordinates;
4. compare declared authority outputs;
5. identify the earliest declared divergence;
6. locate retained causal evidence for that divergence when available;
7. follow later differences only through retained qualified edges;
8. assert selected unchanged controls.

Incompatible lineages must not be presented as one clean counterfactual experiment.

## 21. Unchanged-control requirement

A historical simulation must prove not only where effects propagate, but where they do not.

A remote patch or agent with no physical, hydrological, ecological, signal, social or construction path from a controlled local intervention should remain identical/toleranced until a legitimate causal path reaches it.

This is a central Living Valley requirement.

## 22. Ground That Remembers reference cell

The first executable LENV target should be deliberately narrow.

Start with equivalent small ground patches:

- H0: no traffic;
- H1: light traffic;
- H2: repeated traffic;
- optional H3: different body/contact profile;
- optional H4: qualified wet-history variant.

Required causal chain:

```text
contact/action evidence
    -> surface/environment observation
    -> effect proposal
    -> owner settlement
    -> persistent physical/ecological state
    -> later observation differs
    -> later interaction/route evidence differs
```

An untouched control patch must remain unchanged except for declared background processes.

If production Terrain mutation semantics required by the fixture are not yet qualified, use an explicitly synthetic/reference owner fixture. Do not mutate presentation/runtime `EarthChunk` state and call that production authority.

## 23. Ground That Remembers hostile tests

At minimum:

- clean control remains unchanged;
- light/repeated traffic causes declared state divergence;
- duplicate retry cannot apply an effect twice;
- stale owner revision rejects atomically;
- visual footprints/grass rendering cannot change canonical state;
- untouched nearby control remains unaffected without a modeled propagation path;
- save/reload preserves future-bearing state;
- unload without canonical time/process advance cannot heal it;
- recovery occurs only through declared process;
- downstream surface interaction changes only after world state changes;
- Observatory reconstructs retained action -> proposal -> settlement -> state ancestry.

## 24. Living Valley convergence theorem

The eventual bounded valley should compare two runs from one sealed initial world, with one controlled historical intervention.

Preferred future example, once one-water prerequisites are qualified:

```text
explicit storm forcing
    -> conserved water changes
    -> surface / support evidence changes
    -> scent / vegetation / exposure history changes
    -> animal crossing / route behavior changes
    -> traffic / trail history changes
    -> later agent evidence / memories diverge
```

Each arrow must be an implemented and qualified boundary, not narrative glue.

A smaller owner-qualified intervention is preferable to a fake storm if hydrology prerequisites are not ready.

## 25. Living Valley evidence gates

### G1 — authority
Every consequential fact has one owner or a clearly labeled read-only projection.

### G2 — environmental causality
Forcing, water, Terrain, vegetation, traces and physical changes interact through typed models/settlements.

### G3 — externalized memory
At least one action leaves a persistent world consequence that a later agent can legitimately observe.

### G4 — niche construction
Repeated behavior changes future behavior through changed world state, not a hidden behavioral counter.

### G5 — epistemic agents
Agents receive qualified evidence/percepts and may remain uncertain or wrong. No Observatory/world-truth shortcut.

### G6 — historical continuity
Save/reload, unload, inactive-time evolution and qualified fidelity changes preserve every declared future-bearing invariant.

### G7 — history cone / counterfactual
The Observatory identifies the first divergence and selected causal descendants between compatible histories while disconnected controls remain unchanged until reached.

### G8 — presentation / return
After a meaningful simulated absence, one-way presentation makes accumulated canonical changes legible enough for qualitative owner playtest.

## 26. Qualification classes

Every result should state which class it establishes:

1. schema / authority integrity;
2. deterministic execution / replay;
3. conservation / currentness / causal settlement;
4. differential history sensitivity;
5. presentation legibility;
6. human/owner playtest.

A convincing muddy path does not prove hydrological conservation, real soil mechanics or empirical ecological calibration.

## 27. Performance discipline

Correctness comes first in a bounded cell.

Later performance work may use:

- sparse traces;
- dirty-region recomputation;
- process-driven fidelity;
- cohort ecology;
- multirate hydrology;
- event-driven agent wakeups;
- procedural presentation materialization;
- CPU/SIMD/GPU execution under an equivalence contract.

Hardware load, render FPS or camera state must not silently choose a different canonical history.

## 28. Non-goals

V0 does not establish:

- a monolithic environment simulator;
- a global weather/climate model;
- universal soil or terramechanics accuracy;
- global per-blade vegetation simulation;
- molecular scent simulation;
- universal environmental hysteresis;
- complete Earth ecosystem calibration;
- forecast skill;
- production open-world scale;
- a consciousness claim;
- permission to bypass source qualification because the integrated scene looks plausible.

## 29. Relationship to existing contracts

This contract composes with, but does not replace:

- Living World process-information/fidelity authority;
- causal fauna and animal niche-construction contracts;
- exact Terrain authority and its metric/spatial-binding successors;
- one-water/hydrology authority;
- typed spatial-topology projections;
- ANIMA epistemic/action contracts;
- Observatory evidence discipline.

Open implementation/planning issues include #1239 through #1248 and #1247 for the eventual Living Valley milestone.
