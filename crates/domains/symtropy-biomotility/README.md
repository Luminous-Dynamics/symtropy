# symtropy-biomotility

CPU-first reference models and benchmark analysis for biological movement.

## First slice: porous bacterial chemotaxis

The first executable model represents a run/event process with distinct:

- run durations conditioned on the preceding run direction;
- short reorientation events (operationally <= 1 second), with turn-angle samples conditioned on the preceding direction;
- long mechanical-trap events (> 1 second), with a pooled turn-angle distribution.

This separation is important for the Pseudomonas putida porous-agar evidence base. A passive trap is not an active motor turn, and a model that pools them can erase the directional mechanism under investigation.

The code is an **event-based reference kernel**, not a validated flagellar motor model, hydrodynamic solver, pore-resolved simulator, growth model or biological prediction system. It includes seeded rotational diffusion during runs (Euler–Maruyama-style heading increments) and sampled event durations/angles, but it does not yet resolve agar pore geometry, cell-body hydrodynamics, flagellar motor states, nutrient diffusion, population growth or ensemble mean-squared displacement.

## Research anchor

Beier et al., "Dual chemotaxis strategy of bacteria in porous media: Run-time and turn-angle bias", *Biophysical Journal* 125(14), 3570–3579 (2026), DOI [10.1016/j.bpj.2026.06.002](https://doi.org/10.1016/j.bpj.2026.06.002).

The paper reports that P. putida uses both run-time and turn-angle bias in porous agar, with turn-angle bias making the larger contribution in the studied conditions. It distinguishes short motor-induced reorientation events from longer passive traps.

Analyzed trajectories: [Zenodo DOI 10.5281/zenodo.17592316](https://doi.org/10.5281/zenodo.17592316). The deposit description identifies wild-type 0.25% and 0.3% agar trajectories and MotAB/MotCD mutant datasets.

**Rights status:** the Zenodo file-level licence, record version, file list and checksums must be verified before copying or redistributing its data. The 2026 article states that raw data and original analysis/simulation code are available from the authors upon reasonable request; its analysed trajectory data are deposited separately. Until dataset rights and schema are verified, the crate contains no downloaded, embedded or redistributed biological data.

## Literature-reported reference values

These are reported by the paper and are not the output of this crate:

| Quantity | 0.25% agar | 0.3% agar |
|---|---:|---:|
| Wild-type tracks | 17,703 | 18,714 |
| Mean run duration | 0.299 s | 0.232 s |
| Runs up-gradient after short events | 56.7% | 55.6% |
| Runs up-gradient after long events | 44.9% | 46.8% |

The paper's agent-based simulation found that, in agar, turn-angle-bias-only produced about 72% of the combined-strategy drift, while run-time-bias-only produced about 25%. These are results from the authors' simulation protocol, not experimental measurements or results reproduced here.

## Build and test

This is an isolated Cargo workspace so it can be built without the full Symtropy workspace:

~~~bash
cargo test --manifest-path crates/domains/symtropy-biomotility/Cargo.toml
~~~

The model uses only the Rust standard library. The seeded generator is for deterministic research fixtures only; it is not cryptographic.

## Mechanistic ablations

`simulate_with_bias_mode` exposes four matched experiment arms:

| Mode | Run-time distribution | Short-event turn-angle distribution |
|---|---|---|
| `None` | Pooled / direction-independent | Pooled / direction-independent |
| `RunTimeOnly` | Conditioned on preceding-run direction | Pooled / direction-independent |
| `TurnAngleOnly` | Pooled / direction-independent | Conditioned on preceding-run direction |
| `Combined` | Conditioned on preceding-run direction | Conditioned on preceding-run direction |

The unconditional sample sets must be derived from the same admitted condition and observation protocol as the directional distributions. Do not substitute arbitrary constants or fit each ablation on a different split. The convenience `simulate` function uses `Combined`.

Rotational diffusion acts during runs and is controlled by a declared coefficient and integration step.

## Interpretation of conditional orientation metrics

The summary attaches an event in record i to the run in record i+1. The final event has no observed following run and is excluded from the conditional denominator. Therefore short-event and long-trap fractions are not calculated from the run that occurred before the event. Direction is classified at the beginning of the following run; rotational diffusion during that following run is a separate trajectory effect.

## Next qualification gates

1. Verify dataset rights, exact version, file listing, checksums, schemas and experimental-unit metadata.
2. Map source event data into a documented, versioned normalized record format.
3. Reproduce the reported conditional statistics with replicate-aware uncertainty, keeping plate/day/track grouping intact.
4. Reproduce Datta et al.'s intermittent active-particle baseline, including rotational diffusion and mean-squared displacement, before claiming incremental value from event separation.
5. Compare no-bias, run-time-only, turn-angle-only and combined models under identical seeds and parameter/data budgets.
6. Add geometry-resolved pore obstruction and passive trapping as an independent physical process.
7. Generalize only after held-out validation across media conditions and other organisms.

No wet-lab work, bacterial culture, ecological release, or physical intervention is authorized by this software.
