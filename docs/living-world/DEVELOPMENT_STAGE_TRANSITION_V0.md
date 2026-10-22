# Development Stage Transition V0

## Purpose

Define how organisms change developmental state without forcing every life form into one universal stage machine.

## Core rule

Species-specific development owns real stage semantics.

The existing coarse `LifeStage` categories are useful cross-domain telemetry labels. They are **not** the universal biological program for every organism.

A species may define a versioned `DevelopmentPlan` whose stages and transitions reflect its biology.

Examples include, without implying universality:

- seed -> germinated -> seedling -> juvenile -> mature -> senescent;
- egg -> larva -> pupa -> adult;
- neonate -> juvenile -> subadult -> adult;
- dormant propagule -> active colony/network;
- vegetative -> flowering -> fruiting phases that cross-cut chronological age;
- phase-changing collective forms.

## Transition authority

A stage transition occurs from canonical biological conditions on authoritative simulation time.

Potential inputs include:

- accumulated developmental history;
- chronological/physiological age;
- body/structural size;
- resource state;
- temperature/light/moisture history;
- hormonal/developmental state when modeled;
- seasonal/phenological state;
- metamorphic thresholds;
- injury or environmental interruption;
- species-program requirements.

Renderer state, mesh choice, animation state, camera distance, and frame rate are not stage authority.

## Transition kinds

Species plans should distinguish at least:

- monotonic transitions;
- reversible/adaptive phases;
- dormancy/diapause transitions;
- metamorphic transitions that rebuild capabilities;
- terminal/senescent states;
- interrupted/failed transitions where biologically relevant.

Do not encode reversibility merely because an enum permits moving backward.

## Capabilities

Stage can alter canonical capabilities and ecological affordances, including:

- metabolic traits;
- locomotion;
- feeding/resource mode;
- habitat requirements;
- reproduction eligibility;
- pollination/fruiting roles;
- predator/prey vulnerability;
- sensory capability;
- body/plant developmental program;
- social role;
- dormancy/overwintering behavior.

Presentation changes derive from those biological changes rather than authorizing them.

## Transition atomicity

If a transition changes multiple future-bearing domains, the transition must publish a coherent new state.

For example metamorphosis must not expose `adult body plan + larval metabolism` unless an explicit transitional state permits it.

## Mapping to coarse telemetry

A versioned mapping may expose a species stage as coarse `LifeStage` telemetry for analytics/interoperability.

That mapping is lossy and cannot reconstruct the species-specific stage.

## Coarse fidelity

Offscreen/coarse ecology may retain a stage distribution or sparse stage-correlated strata when stage changes future processes.

A population representation that stores only total count is insufficient if stage changes mortality, reproduction, feeding, disease, biomass, or habitat use.

## Qualification

Test:

- deterministic transition under identical authoritative history;
- renderer/FPS independence;
- save/reload equivalence mid-stage and at boundaries;
- fine vs qualified coarse catch-up equivalence;
- prohibited backward transition rejection;
- capability changes occur atomically with stage;
- population coarse state preserves stage information whenever enabled processes require it;
- coarse telemetry mapping is explicitly lossy.

## Non-claims

V0 does not define universal stage names, timing, thresholds, hormone models, or real-species calibration.
