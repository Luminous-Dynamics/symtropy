# Living World Fidelity Invariance Matrix V0

Status: normative Living World validation contract. Documentation only.

## Purpose

Define the differential tests required whenever Living World changes representation fidelity, execution cadence, region partitioning, persistence boundary, or presentation tier.

## Core principle

> Every optimization states which canonical observables it must preserve and how equivalence/error is measured.

"Faster" is not a valid optimization if it silently changes which organisms exist, where matter went, what history was retained, or what future processes can infer.

## Invariance dimensions

### Presentation invariance

Compare:

- headless vs rendered;
- low vs high presentation LOD;
- different renderer quality profiles/backends;
- stable vs rapidly changing camera distance.

Canonical ecology must remain identical for processes that presentation does not own.

### Temporal-fidelity invariance

Compare fine per-tick execution to qualified coarse catch-up for processes claiming a sufficient coarse closure.

Examples:

- ecological cadence catch-up;
- cumulative developmental exposure;
- population/cohort processes;
- field updates where a coarse scheme is qualified.

Report exact equality or a declared bounded error by observable.

### Population refinement invariance

Compare:

`coarse -> active refinement -> unchanged/canonical reduction -> coarse`

for conserved/sufficient observables such as:

- count;
- exact biomass/stock;
- required marginals/strata;
- region occupancy;
- process residuals;
- group membership where applicable.

Promotion/demotion must not create free hunger recovery, distance, health, reproduction, or identity.

### Persistence invariance

Compare uninterrupted execution:

`tick 0 -> T`

with:

`0 -> K -> save -> reload -> T`.

Future canonical observables and event/authority state must agree under the relevant deterministic contract.

### Region-partition invariance

Where process semantics claim partition independence, compare equivalent worlds simulated as:

- one region;
- multiple regions with border/halo exchange;
- different valid worker scheduling.

Canonical macro-observables must agree exactly or within explicit qualified tolerance.

### CPU/GPU/backend differential

When GPU/parallel implementations replace reference CPU paths, compare against the reference implementation on frozen fixtures.

Do not infer equivalence solely from visual similarity or average statistics when exact/reference observables are available.

## Observable capability matrix

Each process should list required observables/capabilities, for example:

| Process | Required capability examples |
|---|---|
| simple drought response | stage/condition + qualified habitat history |
| age-dependent disease | age × disease state |
| local infection | spatial/contact structure |
| predation/contact | active spatial/body/contact authority |
| plant growth | structural target + exact resource settlement + developmental state |
| persistent learned fear | individual memory/identity continuity |
| decomposition | exact stock/reaction ownership + residual state |

A representation may execute a process only when it supplies the required information or a separately qualified closure.

## Error budgets

Approximate closures may declare an error budget only for observables/processes that permit approximation.

Exact authority requirements—exclusive ownership, integer count, committed identity, exact conserved stock where required—cannot be waived by a statistical error bound.

If an approximate closure approaches/exceeds its qualified regime, the fidelity controller must promote richer state before the process relies on invalid information.

## Anti-drift sequences

Test repeated cycles, not just one transition:

- P0↔P4 presentation thrashing;
- coarse↔active ecology repeated hundreds/thousands of times;
- repeated region migration;
- repeated save/reload;
- repeated projection/realization rejection;
- repeated fractional-flux settlement.

Tiny one-way errors that look harmless once can accumulate into large ecological drift.

## Required evidence format

For every differential report record:

- two compared implementation/config identities;
- exact starting-state identity;
- ticks/time horizon;
- preserved observables;
- observed differences;
- tolerance/error budget and rationale;
- PASS/FAIL/NOT-ESTABLISHED;
- whether the result is exact, statistical, or perceptual.

## Non-goals

This contract does not require every fidelity level to preserve every microscopic detail. It requires explicit proof that discarded detail is irrelevant or bounded for the processes permitted at that representation.