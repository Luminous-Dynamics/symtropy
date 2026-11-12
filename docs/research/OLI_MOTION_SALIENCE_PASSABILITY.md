# OLI motion-salience and source-linked propagation benchmark v0

**Status:** OLI operationalization implemented; source paper profile pinned; exact source equation/data admission pending  
**Owner:** Symtropy / `symtropy-lifesim-core::motion_salience`  
**Manifest:** `benchmarks/oli/motion-salience-passability-v0.json`  
**Related:** [OLI collective-motion benchmark PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670)

## Why this is a paradigm shift

Collective order is not the same thing as information propagation. A group can have high global polar order while a localized pulse fails to spread, while background fluctuations activate agents, or while a common external field creates correlated responses with no direct agent-to-agent influence.

Li et al. (2026), *Chaos, Solitons & Fractals* 212, article 119091, DOI [10.1016/j.chaos.2026.119091](https://doi.org/10.1016/j.chaos.2026.119091), frame the problem as **selective admission**. An intermediate Motion Salience Threshold (MST) window supports source-linked and temporally outward propagation. Too-low thresholds admit background activity; too-high thresholds starve the relay. Their abstract explicitly reports that high global order alone does not capture source-specific propagation.

The 2025 precursor, Li et al., DOI [10.1016/j.chaos.2025.116585](https://doi.org/10.1016/j.chaos.2025.116585), studies an active/inactive mechanism in which particles either selectively respond to a salient neighbour or align with a local average. Its threshold/noise sweeps report different transition behaviours, including hysteresis at low noise.

This suggests an important OLI distinction:

```text
high global order
    ≠
successful source-linked transmission
    ≠
causal inter-agent influence
    ≠
intelligence or agency
```

## Source values versus OLI choices

The accessible 2026 article reports a reference simulation with:
- N=800 particles in a periodic 50×50 square;
- v0=1, lambda=10, interaction radius R=5;
- deactivation angle 10 degrees;
- dt=0.1;
- threshold MT from 0.5 to 5.0 in steps of 0.5;
- noise eta in {0, 0.05, 0.1, 0.15, 0.2}.

The exact seed protocol and full code/data were not available in the retrieved source text. Do not invent or claim to have reproduced them.

The new Rust module implements an explicit OLI variant:

[
s_{ij}(t) =
rac{left|operatorname{wrap}_{(-pi,pi]}
left(eta_{ij}(t)-eta_{ij}(t-Delta t)ight)ight|}{Delta t}.
]

A neighbour is admitted if it lies within the declared interaction range and its measured salience is at least the configured threshold. This uses an absolute wrapped finite-difference relative-bearing rate. The paper’s accessible text says salience is the rate of change of relative bearing, but its complete exact equation and inactive-state update have not yet been independently inspected. Consequently this module is **not** presented as a source-faithful reproduction.

The rate estimate is cadence-dependent. A two-sample wrapped difference cannot detect arbitrarily fast turns between observations. The implementation flags bearing increments near pi as an angular-sampling warning; it cannot prove aliasing is absent when the warning is false. OLI must vary cadence and preserve target-process timescale/observability metadata.

## Executable implementation

`crates/domains/symtropy-lifesim-core/src/motion_salience.rs` currently contains:
- finite/positive input validation and wrapped angular differences;
- inclusive threshold admission and range exclusion;
- an explicit near-angular-sampling-limit warning;
- a matched pulse/sham source-response summary;
- common-random-number requirements;
- treatment, sham and incremental response fractions;
- median response time, maximum source radius and outward temporal-order concordance;
- explicit failure reasons for poor participation, excessive sham activation, weak incremental response or absent outward ordering;
- per-noise threshold-sweep analysis that identifies contiguous passable intervals and whether the tested interval is bounded by failed points on both sides.

The sweep API refuses duplicate (noise, threshold) coordinates and refuses a report whose embedded configuration differs from its sweep coordinate. These guards prevent silent mixing of runs.

## Benchmark protocol

### 1. Exact controls

The implementation tests:
- wrapping from +179° to −179°;
- inclusive threshold equality;
- out-of-range exclusion;
- warning near angular sampling limit;
- invalid cadence rejection;
- source-linked outward pulse passage;
- common-driver/sham activation that fails source specificity despite full participation;
- closed-gate participation starvation;
- rejection of unmatched sham or mismatched random streams;
- bounded intermediate passability windows per noise level;
- independent noise slices and duplicate/mismatched-coordinate rejection.

These are structural/synthetic controls, not reproduction of the Li et al. experiment.

### 2. OLI sweep

The OLI manifest pre-registers a staged parameter grid using the reported N, domain, speed, radius, timestep, threshold and noise values, with 30 independent seeds per point as an **OLI target**, 1,000 burn-in steps and 3,000 measurement steps. The seed schedule and horizon are OLI choices, not claims about the source article.

The following controls are run under matched initial states and common random numbers:
1. pulse treatment: one declared source agent receives a pulse;
2. matched sham: no pulse;
3. gate removed: separates order from salience-gated admission;
4. gate closed: tests participation starvation;
5. common external driver: creates co-change without direct interactions;
6. shuffled source identity: exposes post-hoc source attribution.

### 3. Primary metrics

Report the global polar order beside:
- fraction of agents admitted by the salience gate;
- treatment response fraction;
- sham response fraction;
- incremental response fraction;
- maximum source-linked propagation radius;
- response-time distribution and median;
- pairwise outward temporal ordering;
- local polar order and its coverage;
- false source-attribution rate;
- threshold-window bounds independently for each noise slice;
- risk/coverage under temporal downsampling.

The main result is a two-dimensional passability map over noise and threshold, not a single averaged order score. Preserve each seed-level map. A bounded interval is evidence only within the tested grid and the chosen passability criteria; it is not automatically proof of a physical phase transition.

### 4. Provisional OLI pass criteria

The manifest gives initial engineering criteria:
- treatment response fraction ≥0.10;
- sham response fraction ≤0.05;
- treatment-minus-sham incremental fraction ≥0.10;
- outward temporal-order score ≥0.75.

These numbers are **provisional OLI design thresholds**, not thresholds reported by Li et al. They must be tuned on a development partition, frozen before blind evaluation, and reported with sensitivity analysis. Publish the continuous metrics even when a run does not pass.

### 5. Statistical rules

- Independent seed × (noise, threshold) is the experimental unit.
- Time frames and particles inside a trajectory are repeated observations, not independent cases.
- Bootstrap across independent seeds, not across frames.
- Show finite-size and noise sensitivity before claiming a stable passability regime.
- Keep the source reproduction and OLI extension as separate tables.
- Report all no-pass/low-participation cases; do not select only the successful threshold band.

## How OLI can exceed the current benchmark

A strong next-generation extension would infer a **passability surface** rather than merely locate one threshold window. It should estimate:
- threshold × noise × population-density regions where transmission is robust;
- probability of source attribution conditional on propagation distance;
- propagation latency and radial front speed;
- sensitivity to observation cadence, occlusion and common drivers;
- adaptation when the environment changes the salience/noise statistics.

A useful eventual score is *source-specific propagation at fixed false-attribution budget*, reported alongside participation and latency. This is more meaningful than maximizing global order or raw response count.

The highest bar is prospective prediction: choose the threshold/gate setting from a development set, then predict the passability region on unseen seeds, densities and interaction rules. Report calibrated abstention when the interaction model is inadequate.

## Data admission and claim ceiling

The exact 2026 source equation, inactive-state dynamics, code, seed law and trajectory outputs remain to be acquired. Until that happens, all Rust outputs must be labelled as OLI-variant simulations. No natural collective, biological intelligence or communication claim is implied.

Polar order, source-linked propagation, mutual information, transfer entropy, or response-time ordering alone do not establish cognition, agency, intention, or consciousness.
