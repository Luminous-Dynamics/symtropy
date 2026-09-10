# Player Building Program v0.2

Status: **program architecture / sequencing contract; no runtime PASS claim**

This document keeps the player-building program coherent while PB-01 hardening and the downstream Construction/Fabrication stack are still being qualified.

The goal is not to maximize subsystem count. The goal is to preserve one causal construction language from direct hand placement through settlements, vehicles, spacecraft, and stations while giving the player progressively more expressive tools.

## Program theorem

```text
creative authoring
-> exact proposal semantics
-> explicit proposal-to-physical compilation
-> existing physical work authorities
-> realized structure
-> derived spatial/place/use projections
-> scalable fidelity
```

At every stage:

```text
proposal != permission
preview != execution
design != workmanship
space != place
place != home
home != ownership
coarse representation != new physical truth
```

## Lean architecture rule

Do **not** create a new crate merely because a new PB number exists.

A new crate is justified only when at least one of these is true:

1. it owns a distinct authority boundary that should remain dependency-isolated;
2. it has multiple real consumers whose reuse would otherwise create circular dependencies;
3. it needs an independently versioned wire/persistence contract;
4. its test/evidence surface benefits materially from isolation.

Otherwise prefer a module inside an existing appropriate crate.

The PB numbering tracks proof/program stages, not crate count.

## Current ownership map

### `symtropy-player-building`

Owns only proposal semantics:

- exact authored intent;
- exact proposal plan;
- proposal identities/references;
- proposal-only operations;
- constraint/projection data that remains non-authoritative;
- future bounded hostile proposal ingress if it can stay dependency-light;
- potentially blueprint semantic types if they remain tightly coupled to proposal generation.

It must not own:

- conserved matter;
- Fabrication process truth;
- Construction execution/work/staging;
- structural solver truth;
- utilities;
- ownership/civic permission;
- place/home identity;
- renderer/UI state.

### Existing Fabrication + Construction stack

Remains the physical execution spine.

PB-02 adapts *into* this stack. It does not replace or wrap it with a second executor.

### Authoring/editor layer

PB-03 mutable drafts, selection, undo/redo, snapping, guides, numeric entry and editor preferences are **not** automatically a new domain authority.

Prefer an application/bridge module first. Extract a reusable headless authoring crate only when deterministic draft transforms/diffs have at least two concrete consumers such as player UI + collaborative/server tooling.

### Spatial topology

PB-04 is the strongest candidate for a distinct reusable domain crate because AI, audio, thermal, atmosphere, navigation acceleration, habitation and later ships/stations can all consume it.

If implemented separately, keep it read-only over exact boundary/provider snapshots.

### Place/home semantics

PB-05 likely belongs in a reusable place/social/world domain rather than the builder UI. `PlaceIdentity` must survive construction UI changes and may represent homes, workshops, landmarks, ships, institutions, ruins and other socially meaningful locations.

Do not put character psychology or ownership authority into that crate merely for convenience.

### Fidelity

PB-08 should begin construction-specific. Extract a generic fidelity/closure substrate only after at least one other domain can consume the same types without weakening their meaning.

LifeSim closure work is an architectural precedent, not a dependency justification by itself.

## Dependency / proof graph

```text
PB-00  Player Building Authority
  |
  v
PB-01  Proposal IR
  |
  +--> #439 hardening
  |      |- H1 exact existing-target closure
  |      |- H2 canonical serializer-independent identity
  |      |- H3 content-addressed fork/merge ancestry
  |      |- H4 bounded hostile ingress
  |      `- H5 authored-rotation identity policy
  |
  v
PB-02  proposal -> exact physical compilation bridge
  |
  +------------------+
  |                  |
  v                  v
PB-03 authoring      PB-07 blueprints/delegation
  |                  |
  +--------+---------+
           |
           v
PB-06 integrated shelter causal proof
           |
      +----+----+
      |         |
      v         v
PB-04 spatial  PB-05 place/home
      |         |
      +----+----+
           |
           v
PB-08 hierarchical fidelity
           |
           v
PB-09 rover anti-overfitting proof
           |
           v
PB-10 settlement infrastructure composition
           |
           v
PB-11 pressure/orbital habitat proof
```

The diagram is a dependency intent, not a mandate that implementation must be serial. For example PB-03 draft UX or PB-04 synthetic topology fixtures can be developed in parallel, but no downstream stage may claim physical integration before its actual authority dependencies are qualified.

## PB-00 — universal authority contract

Reference: #426.

Exit meaning: the semantic direction is accepted. This stage is documentation only and does not qualify runtime behavior.

## PB-01 — authority-safe proposal IR

Reference: #438 with hardening gate #439, identity spec #441 and ingress spec #442.

PB-01 exits only after the primary runtime path—not a side theorem module—enforces:

- exact existing target closure;
- content-addressed collaborative ancestry;
- serializer-independent canonical identity;
- schema/profile compatibility boundary;
- bounded hostile ingress for any direct untrusted decode surface;
- frozen authored-rotation identity semantics;
- deterministic plan DAG and replay validation;
- exact current-head execution evidence.

Do not stabilize PB-01 simply because its design documents are good.

## PB-02 — compilation receipt / authority crossing

Reference: #440.

The essential product API should remain narrow:

```text
exact intent + exact PB plan + exact adapter context
-> pure deterministic candidate compilation
-> sealed compilation receipt
-> exact lower process resolution
-> Existing ExecutableFabricationPlan
-> Existing ExactConstructionSite
```

No proposal identity laundering into lower IDs. No hidden matter or work mutation during compile.

PB-02 should first support a tiny profile: two newly authored elements + one join. Then exact-target repair. Do not start with a generic universal adapter registry.

## PB-03 — player authoring freedom

Reference: #446.

Product principle:

> Drafting should be forgiving; sealing should be exact; reality should remain causal.

Minimum authoring capabilities before broad content expansion:

- free placement;
- optional deterministic snapping;
- numeric precision;
- semantic undo/redo;
- inspectable constraints;
- copy/pattern/parametric operations;
- blueprint instantiation;
- semantic diff;
- collaborative fork/merge semantics;
- Symthaea as a non-privileged design companion.

Do not make ordinary building feel like mandatory CAD. Precision is available, not compulsory.

## PB-04 — faceted spatial topology

Reference: #455.

Avoid one universal room graph. Derive geometric regions and separate topology facets for at least:

- occupancy/access;
- air/pressure;
- acoustics;
- visibility;
- thermal exchange;
- weather exposure.

Dynamic interfaces bind exact door/window/vent/hatch state. Incremental recomputation must be observationally equivalent to full recomputation under the same profile.

First proof: crooked terrestrial shelter. Second proof: two-compartment pressure habitat.

## PB-05 — place and home emergence

Reference: #449.

Core identity chain:

```text
realized physical lineage
-> persistent PlaceIdentity
-> observed use/history/relationships
-> character-relative HomeAssociation
```

Make attachment emerge from continuity, possessions, routines, people, repairs, names, light, sound and history. Avoid manipulative attachment meters and universal `is_home` flags.

## PB-06 — integrated shelter vertical

Reference: #492.

This is the first milestone that should feel like a game feature rather than a collection of contracts.

The shelter must prove in one causal loop:

- uneven physical site preparation;
- freeform/non-grid authored geometry;
- real accounted material;
- incomplete construction + save/load;
- derived spatial topology;
- at least one connected service;
- PlaceIdentity before HomeAssociation;
- repeated habitation/personal possessions;
- damage + repair/replacement;
- workmanship/maker provenance;
- blueprint change-order without history rewrite;
- deterministic headless replay of claimed deterministic state;
- negative authority-bypass tests;
- deterministic capture review;
- owner playtest.

The first home should be worth remembering before the system attempts a city.

## PB-07 — portable blueprint / delegation layer

Reference: #493.

Blueprints are inert, bounded, declarative design artifacts. They may carry parameters, constraints, interface expectations, design lineage and external profile references.

Instantiation creates a new PB proposal subject. Delegated workers, robots, factories and shipyards execute ordinary lower-authority work.

Key distinction:

```text
design provenance != maker provenance != process evidence
```

This lets the same blueprint produce distinct hand-built, crew-built and factory-built realizations.

A future Mycelix attribution/compensation bridge may bind licenses/payments to exact blueprint derivation, but offline core construction must not depend on it.

## PB-08 — hierarchical fidelity

Reference: #491.

Do not implement one global LOD integer.

Fidelity is process/information-relative. Every coarse profile declares what survives and which processes it may approximate. Unsupported events trigger bounded refinement or revalidation.

Distance/frame rate can influence scheduling; it cannot change canonical reality.

First proof: exact shelter -> dormant summary -> targeted refinement -> no duplication/healing/history loss.

Second: neighborhood with shared utilities.

Third: orbital pressure event that refines only the required frontier.

## PB-09 — rover anti-overfitting proof

Before spacecraft building, construct one small useful rover using the same proposal, blueprint, fabrication, construction, structural/interface and provenance semantics.

A rover is deliberately chosen because it exposes assumptions hidden by static buildings:

- moving coordinate frames;
- assemblies/subsystems;
- wheels/suspension or equivalent locomotion interfaces;
- onboard power/control;
- repair/replacement;
- mass distribution;
- mobile PlaceIdentity edge cases if inhabited/stored.

If PB requires a separate magical vehicle-building ontology, fix it here.

## PB-10 — settlement infrastructure composition

A settlement is not one giant construction object.

Compose:

```text
places
+ buildings
+ parcels/sites
+ roads/paths
+ power
+ water
+ waste
+ communications
+ logistics
+ delegated work
+ institutions/economy elsewhere
```

Infrastructure networks retain their own authorities. The player-building layer supplies goals/designs/change orders, not universal city truth.

Use hierarchy and PB-08 fidelity so one repair does not activate the entire settlement.

## PB-11 — pressure/orbital habitat proof

Only after rover + fidelity proofs.

Demonstrate the same construction language with domain-specific requirements for:

- pressure boundaries;
- hatches/airlocks;
- power;
- life-support interfaces;
- thermal rejection;
- docking/structural connections;
- modular repair/isolation;
- moving/orbital frames.

This proves the ontology scales from home to station without claiming complete spacecraft engineering.

## Cross-program invariants

Every PB successor should be reviewed against these invariants:

### Causality

No UI, AI, blueprint, preview, room classifier, place label, LOD system or convenience adapter can manufacture physical completion.

### Conservation

Building, dismantling, salvage and representation changes do not create/delete accounted matter outside explicit owning processes.

### Exact lineage

Important identities/revisions/content refs remain explicit across design, compilation, execution, repair, place history and fidelity transitions.

### Explainability

A player-facing restriction should be traceable to the relevant rule/evaluator/authority. Risky-but-simulatable designs should generally receive consequences/warnings rather than arbitrary prohibition.

### Freedom

Snapping, grids, patterns, blueprints, automation and Symthaea assistance save labor; they do not become mandatory ontologies.

### Scale

Large structures compose from bounded units and typed summaries instead of one unbounded proposal or one always-hot entity graph.

### Offline-first core

Core building does not require Mycelix, cloud AI, external marketplaces, online licensing checks or remote services. Those may add optional capabilities later.

### Evidence discipline

Static design != runtime implementation.

Implemented/static != executable-qualified.

A green parent != a green successor.

A provider PR merge run != exact product-head proof unless the exact executed subject is bound and recorded.

## Performance philosophy

Performance should come from representation and scheduling architecture, not weakened semantics.

Preferred techniques include:

- bounded proposal units;
- incremental dirty-region recomputation;
- graph cut-set refinement;
- spatial partitioning;
- sleeping/dormant entities;
- process-relative coarse closures;
- batch rendering/meshing as presentation only;
- deterministic job scheduling where canonical outcomes depend on order;
- asynchronous-looking UI work that does not hide authority state.

Avoid optimization by deleting provenance, silently skipping physical processes, turning distant buildings invulnerable, or making unloaded utilities generate resources.

## Player experience north star

The strongest test is not 'how many building pieces exist?'

A successful Symtropy building system should let a player:

- improvise a crooked shelter quickly;
- understand and override assistive snapping;
- discover physical consequences rather than arbitrary grid rules;
- gradually replace improvised parts with engineered ones;
- delegate repetitive work without losing causal continuity;
- recognize their home from layout, objects, light, sound and history;
- preserve a meaningful old beam while rebuilding everything around it;
- reuse/share/remix designs without cloning physical identity;
- eventually apply the same mental model to a rover, town, ship or station.

If those experiences all emerge from the same authority spine, the system is succeeding.

## Immediate execution order

Until hosted runner capacity clears:

1. keep `building/pb01-proposal-ir-v0.1` frozen at the exact H1/H3 transformer input;
2. review #441 canonical schema-2 identity grammar and #442 bounded ingress independently;
3. do not call H1/H3 or H2/H4 executable-qualified;
4. avoid PB-02 runtime implementation against moving/unqualified downstream authority heads;
5. develop synthetic/read-only PB-03/PB-04 proofs only when they do not contaminate the frozen PB-01 lineage;
6. when H1/H3 actually executes, inspect the exact compiler/test result before moving the product branch again;
7. implement canonical identity against that resulting exact primary path;
8. only after PB-01 is hardened and exact-head green, begin the minimal PB-02 two-elements+join adapter.

This sequence favors fewer rewrites over maximum parallel code volume.