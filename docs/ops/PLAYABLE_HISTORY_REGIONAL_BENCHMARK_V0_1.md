---
title: Playable History Regional Benchmark
version: 0.1
status: implementation-spec
scope: integrated validation of campaign compilation, faction initiative, discovery, ordinary life, revisit, absence, and worldline variation
owner: production/qa/narrative/simulation
related:
  - ../tech/PLAYABLE_HISTORY_CONTENT_COMPILER_AND_WORLDLINE_VARIATION_RUNTIME_V0_1.md
  - ../tech/HISTORICAL_PRESSURE_FACTION_CLOCK_AND_CAMPAIGN_STATE_RUNTIME_V0_1.md
  - ../tech/DISCOVERABLE_HISTORY_ARCHIVE_RUIN_AND_ENVIRONMENTAL_STORYTELLING_RUNTIME_V0_1.md
  - PLAYABLE_HISTORY_CONTENT_PACKET_STANDARD_V0_1.md
---

# Playable History Regional Benchmark

## Objective

Prove that one authored region can carry historical texture through ordinary life, systemic campaign progression, player absence, revisit, and worldline divergence.

The benchmark is intentionally smaller than the world-coherence benchmark. It is a production gate for playable content.

## Region Fixture

The reference region contains:

```text
1 legendary settlement
2 outlying communities
1 corporate or successor utility
1 diaspora route
1 informal service network
8 named inhabitants
4 institutions
3 historical root events
12 physical or documentary traces
2 cultural seasons
1 major campaign packet
3 minor activity arcs
```

Recommended exemplar: **Nine Pumps Commons and its drought corridor**, though any equivalent region may be used.

## Timeline

```text
Day 1–3: ordinary life and baseline work
Day 4: first visible pressure
Day 5–8: discovery and faction initiatives
Day 9: public disagreement
Day 10–14: repair, care, logistics, and cultural preparation
Day 15: threshold crisis
Day 16–20: transition and aftermath
6-month player absence
5-year player absence
worldline fork before the threshold crisis
```

## Required Systems

- campaign packet loading;
- pressure and faction clocks;
- NPC initiative;
- material service dependency;
- evidence discovery and custody;
- rumor and public opinion;
- ordinary-life scheduling;
- cultural season state;
- revisit presentation;
- worldline persistence and migration.

## Test Variants

### Baseline Authored

Deterministic authored activities and dialogue.

### Dynamic Faction Initiative

Faction planning enabled; generated language disabled.

### Full Bounded Compilation

Validated activity composition and optional grounded language rendering.

### Player-Absent

The player leaves before visible pressure and returns after six months.

### Forked Worldline

One branch prevents the initial damage; another experiences it.

## Hard Failures

The benchmark fails if:

- the campaign waits indefinitely for the player;
- ordinary life disappears during pressure;
- a faction acts without resources, authority, route, or knowledge;
- dialogue changes authoritative state;
- historical evidence becomes globally known upon pickup;
- defeating leadership deletes the utility;
- absent-player progression creates or destroys matter without cause;
- named people vanish into cohort simulation;
- all outcomes restore the same settlement;
- the fork duplicates unique assets or people;
- the region can only explain itself through journal text;
- a cultural event appears without preparation or labor.

## Quantitative Evidence

Capture:

- deterministic replay hashes;
- campaign wake counts;
- pressure updates;
- faction initiative validation rates;
- activity completion and failure paths;
- material conservation residuals;
- save size and migration time;
- active-region and background CPU budgets;
- localization strings and voiced words;
- evidence-access violations;
- worldline uniqueness checks.

## Human Playtest Questions

1. Could players describe why the conflict existed before them?
2. Did they understand what the contested institution genuinely provided?
3. Could they identify at least three inhabitants by life outside the campaign?
4. Did ordinary life make the outcome matter?
5. Were evidence and interpretation distinguishable?
6. Did player absence feel causal rather than punitive?
7. Were at least two outcomes morally and practically defensible?
8. Did the post-campaign settlement feel changed but still alive?
9. Could players recognize the worldline difference without a summary screen?
10. Would they return after the campaign for reasons other than rewards?

## Promotion Gate

Playable-history systems may move beyond `I0` only when the benchmark bundle includes:

```text
source package
compiled package
schema report
replay traces
save and migration evidence
performance captures
rights and privacy report
accessibility review
localization smoke test
blind playtest report
known limitations
```

## Governing Principle

> **The benchmark succeeds when a region feels like it had a past before the player and will have a future after them.**
