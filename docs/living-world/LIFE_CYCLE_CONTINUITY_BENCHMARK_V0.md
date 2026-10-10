# Life-Cycle Continuity Benchmark V0

## Purpose

Define qualification scenarios proving that reproduction, dormancy, development, recruitment, and generational turnover remain coherent across save/reload, region/fidelity changes, and active/coarse representation transitions.

## Core question

Does one committed biological lineage produce the same authoritative ecological future regardless of presentation, unload/reload, and qualified fidelity path?

## Scenario skeleton

Use a deliberately tiny deterministic species fixture with authored/versioned values suitable for testing rather than claiming real-species calibration.

A minimal scenario should include:

1. reproductive parent/source becomes eligible;
2. finite reproductive investment is paid;
3. one committed reproduction event creates hereditary/propagule authority;
4. propagule enters a dormant bank or equivalent pre-recruit state;
5. authoritative environmental history advances;
6. recruitment/establishment occurs;
7. recruit advances through at least one species-specific developmental transition;
8. parent and offspring continue under different fidelity/load paths;
9. optionally one lineage dies/decomposes in later benchmark revisions.

## Differential paths

Run the same canonical scenario through:

### L0 — uninterrupted fine execution

Reference path where practical.

### L1 — save/reload during reproductive investment

No double payment, refund, duplicate offspring, or altered heredity.

### L2 — save/reload with propagule dormant

Bank count, viability/history and hereditary class remain unchanged except for authoritative elapsed-time evolution.

### L3 — save/reload immediately around recruitment commit

Exactly-once recruitment: source authority and destination recruit cannot both remain counted.

### L4 — coarse offscreen catch-up

Use only a qualified sufficient-statistic closure. Compare declared preserved observables against fine execution.

### L5 — repeated promote/demote cycles

Approach/leave the recruit repeatedly. Count, heredity, stage, stock and future-bearing state must not drift.

### L6 — low/high presentation LOD

Visual representation changes without altering reproduction, bank, recruitment or development.

### L7 — region boundary transfer

Where propagule/recruit dispersal crosses regions, total ownership/count remains coherent and partitioning does not duplicate the recruit.

## Exact invariants

Where represented exactly, require exact equality for:

- source/recruit counts;
- reproduction/recruitment event identity;
- hereditary content/version;
- exact conserved stock ownership;
- propagule/recruit authority ownership;
- stage-transition identity/order;
- retry/idempotency records;
- source-state revision/generation where authority depends on it.

These cannot be relaxed by an approximation tolerance.

## Toleranced observables

Some biological closures may be approximate, e.g. population viability distributions or continuous physiological state. Each path must declare:

- observable;
- reference implementation;
- tolerance/error envelope;
- valid regime;
- promotion trigger when the closure exits that regime.

## Hysteresis / ecological memory

Include paired worlds with identical current habitat but different propagule-bank/reproductive histories.

Expected: future recruitment/succession may differ when the retained history is biologically relevant.

A coarse representation that collapses these worlds to the same state is insufficient for that enabled process.

## Stage transitions

Qualification must use the species-specific development plan. Coarse `LifeStage` telemetry alone is not sufficient evidence that real stage state survived.

Compare:

- stage identity;
- transition tick/history;
- capability changes;
- stage-correlated biomass/condition where relevant.

## Reproduction retry attack

Deliberately replay the same reproduction/recruitment request after simulated acknowledgement loss.

Expected:

- same committed result returned or fail-closed recognition;
- no second resource charge;
- no second offspring;
- no alternate heredity reroll.

## Materialization attack

Materialize/project the same population/recruit at multiple presentation or observer sites.

Expected: observers may duplicate views, never canonical offspring authority.

## Metrics

Track at minimum:

- count drift;
- exact stock drift;
- duplicate-authority incidents;
- hereditary divergence under identical event lineage;
- stage-transition divergence;
- propagule-bank state divergence;
- save/reload mismatch count;
- promotion/demotion accumulated error;
- region-partition divergence;
- retry idempotency failures.

## Evidence claim boundary

Passing demonstrates continuity/invariance for the tested life-cycle model and execution paths.

It does not prove real-world fertility, demography, evolution, or species calibration.

## Relationship

PR #230 defines the species, heredity, reproduction, development, propagule, investment and recruitment contracts. PR #170 defines the multiscale authority semantics those transitions must obey.
