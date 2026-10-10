# Living World Spatiotemporal Information Sufficiency V0

Status: normative design contract; documentation only.

## Purpose

A representation can carry the right *kind* of information and still be insufficient because that information is too coarse, too stale, or aggregated under incompatible temporal semantics.

## Core invariant

> Canonical process sufficiency is a claim about information **at a qualified spatial and temporal support**, not merely a matching field/type name.

## Canonical time

All freshness/cadence authority is expressed in canonical simulation time/ticks.

Wall-clock timestamps, render frames, GPU dispatch cadence, and observed FPS are not ecological freshness evidence.

Candidate policy concepts include:

- canonical observation/state tick;
- maximum accepted state age in ticks;
- maximum update interval in ticks;
- temporal aggregation window;
- integration/aggregation semantics;
- optional qualified temporal closure evidence.

## Spatial support

A capability that carries spatial information binds the support/resolution/schema over which it is meaningful.

A finer or equal exact resolution MAY satisfy a coarser process requirement when support semantics are compatible.

A coarser representation MUST NOT satisfy a finer exact requirement simply because both advertise `SpatialStructure`, `Occupancy`, or a similar category.

## Temporal semantics

The following are distinct unless a qualified conversion/closure says otherwise:

- instantaneous sample;
- mean over window;
- accumulated exposure/integral;
- minimum/maximum/peak;
- threshold-crossing history;
- moving-window statistic;
- decayed/hysteretic memory.

For example, a 24-hour mean moisture value cannot substitute for peak drought exposure for a threshold-sensitive process.

## Freshness

A process may require state no older than `A` canonical ticks.

A source older than that limit fails closed even when the underlying information was exact when observed.

Save/reload at the same canonical tick must reproduce the same freshness decision.

## Cadence

A representation updated every `U` ticks cannot authorize an exact process requiring interval `R < U` unless an explicitly qualified temporal closure or retained sufficient statistic covers that process.

Faster rendering does not refresh the ecological state.

## Closure scope

Spatial/temporal closures bind their qualification domain. An error certificate at one spatial support/window is not automatically valid at another.

## Plan staleness

Prepared process/realization/promotion/collapse plans bind the relevant source tick/revision/support semantics. If required sources become stale or are replaced before commit, the plan rejects rather than mixing old evidence with new authority.

## Qualification fixtures

- 1 m exact occupancy satisfies compatible 5 m requirement;
- 5 m occupancy rejects 1 m exact requirement;
- current-tick contact state passes strict freshness;
- stale contact state rejects;
- 10-tick update cadence rejects a 1-tick exact requirement;
- registered temporal closure may pass only under its exact model/domain/error contract;
- mean-over-window does not satisfy peak requirement;
- accumulated exposure does not equal an instantaneous sample;
- camera/FPS changes do not alter result;
- save/reload reproduces the same result.

## Relationship

This contract refines process-information sufficiency after registry/profile resolution. It does not authorize reconstruction of missing information or choose among multiple valid representations.

Relates to #228, #259, #263, #269, #270, #272, #273.
