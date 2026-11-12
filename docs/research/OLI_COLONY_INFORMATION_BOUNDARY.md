# OLI colony information-boundary benchmark v0

**Status:** information modes implemented; Rust execution pending  
**Owner:** Symtropy / `symtropy-colony`  
**Parent:** [OLI-COLLECTIVE-001 #4578](https://github.com/Luminous-Dynamics/symtropy/issues/4578)  
**Related causal comparison:** [Symthaea OLI-ACTIVE-CAUSAL-001 #11614](https://github.com/Luminous-Dynamics/symthaea/issues/11614)  
**Machine-readable contract:** `benchmarks/oli/colony-information-boundary-v0.json`

## An important source-audit finding

The existing colony implementation described itself as a stigmergic foraging loop, but its legacy `food_score` searched the complete `food_sources` map to add a shortest-path hint for every candidate move. Its `home_score` also used a global shortest-path search and a Manhattan-distance hint to the nest.

That means the historical default was **oracle-assisted**, not a pure decentralized stigmergic system. Agents were not simply discovering food through local pheromone gradients; the movement score had hidden global route information.

This is a consequential evaluation flaw, not a cosmetic naming issue. A benchmark that attributes successful foraging to stigmergy while giving agents direct access to the hidden food map overstates what the environmental-memory mechanism demonstrated.

The correction is not to erase the legacy model. Keep it as a valuable omniscient upper bound, explicitly named and tagged, and compare it to models with honest information budgets.

## Three explicit information modes

The new `ColonyInformationMode` makes that boundary part of the executable configuration.

| Mode | Food-location guidance | Return guidance | Shared environmental memory |
|---|---|---|---|
| `OraclePathAssisted` | Global shortest-path hint over actual food-source map | Global nest shortest path + Manhattan hint | Reads local pheromones; agents deposit trails |
| `StigmergicFieldOnly` | Local food-pheromone values only; direct contact collects food | Local home-pheromone values only | Food sources emit the food field; agents deposit food/home trails |
| `IndependentLocalSearch` | Deterministic local exploration; no food-map hint | Declared nest landmark (Manhattan distance) | No food/home trail read/write; local obstacle/danger perception remains |

The old `ColonyWorld::new` remains backward-compatible and explicitly constructs `OraclePathAssisted`. New comparisons should call `new_with_information_mode` and choose a mode deliberately.

The independent baseline is not completely blind: agents retain direct food contact, local obstacle/danger sensing, and a known nest landmark. Those are declared capabilities, not hidden communication. Its key difference is the removal of shared food/home environmental memory.

## Executable invariants added

The Rust source now includes tests for:
- legacy constructor is tagged as oracle-assisted;
- stigmergic food scoring is invariant to changing hidden food coordinates when local pheromone observations are held fixed;
- stigmergic home scoring is invariant to changing hidden nest coordinates when the local home field is held fixed;
- the oracle mode retains its global path hint as an explicit upper bound;
- independent local search ignores food/home trail values;
- the independent mode emits no agent-generated food/home pheromone compared with a zero-agent control undergoing the same field diffusion.

These are source-level tests. They still require `cargo fmt`, compilation and exact-head CI execution.

## Frozen initial task

The initial reproduction case uses the existing test geometry:
- 32 × 16 grid;
- nest at (2,8);
- 64 agents, with the existing deterministic scout/forager role assignment;
- food source at (28,8), initial quantity 5,000;
- 180 ticks;
- same obstacles, hazards, initial states, inventory and movement budget in all modes.

The field and ant behavior are deterministic for this fixed scenario; current deterministic jitter is not a seeded stochastic PRNG. Do not report a confidence interval or claim independent seed replicates from multiple time frames of this one run.

## Metrics

Record each separately:
- food reserve at tick 180;
- first delivery tick;
- deliveries per 1,000 ant-ticks;
- fraction of agents that delivered;
- remaining food at source;
- food/home trail totals;
- danger exposure;
- mean path length per delivered unit;
- explicit information mode.

Trail entropy and pheromone coherence are descriptive environmental metrics. They are not universal measures of group intelligence.

## Paradigm shift: benchmark information access, not just the controller

For collective competence, equal agent count and equal simulation steps are insufficient. We must also account for:
- what each agent can observe locally;
- whether the environment carries information from one agent to another;
- whether a central planner sees the full map;
- whether agents possess a global landmark or route;
- whether the same field is available to all agents or only to nearby agents;
- what resource/energy/communication budget is spent to create and maintain environmental traces.

The information mode is therefore part of the benchmark identity, alongside environment, controller, initial state and update rule.

## Limitations and next qualification step

- The current source uses deterministic local jitter, not independent stochastic seed runs.
- The initial task is one simple open-grid layout. It is a development fixture, not enough to establish a general group advantage.
- The source still computes global `ColonyMind` telemetry; its `scout_ratio` is currently diagnostic and does not dynamically change ant roles. This global telemetry must not be represented as agent-local information or adaptive role control.
- The danger field is a local environmental sensor input; it is held constant across the information modes.
- A proper qualification must add mirrored source placements, obstacles, moving hazards, heterogeneous resource fields, map-scale changes, and independent scenario generation.
- Compare all three modes under identical world resources, agent count, tick limit, and direct-contact rules. The oracle-assisted mode is an upper bound only.
- A group-level collective-competence claim requires a reproducible advantage of the field-only system over independent local search and a mechanism-targeted ablation that removes environmental memory.

## Claim ceiling

This is an information-budget and implementation-assurance improvement to a synthetic colony simulator. It does not establish animal collective intelligence, biological cognition, or a general stigmergic superiority result.


## Seeded, paired ensemble contract (12 November 2026)

The colony now provides `new_with_information_mode_and_seed(..., seed)` and records the seed in `ColonyDebugMetrics`.

- Seed 0 retains the original deterministic jitter exactly for backwards compatibility.
- Non-zero seeds use a counter-based SplitMix64 jitter field keyed by seed, tick, ant index and candidate cell. It is deterministic and independent of the order in which candidate cells are enumerated.
- The implementation has no mutable RNG state in `next_target` scoring, so adding/reordering candidates cannot shift later random draws.
- The same non-zero seed is used across OraclePathAssisted, StigmergicFieldOnly and IndependentLocalSearch for paired comparisons.
- Frozen seed IDs: 1–30.
- Frozen map orientations: east→west, west→east, north→south and south→north.
- Obstacle families: none, barrier with gap, two-route corridor, localized danger strip.

The statistical unit is one independently seeded world run. Individual ants and time frames are nested observations, not independent replicates. Report paired per-seed differences and scenario-specific intervals. Do not pool map orientations until heterogeneity is shown. This is an implementation and protocol increment; no ensemble performance result is claimed yet.

