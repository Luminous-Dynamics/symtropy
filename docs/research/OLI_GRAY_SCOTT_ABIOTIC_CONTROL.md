# OLI abiotic hard-negative generator: Gray–Scott reaction–diffusion v0

**Status:** deterministic CPU reference implementation, awaiting Rust validation  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Related:** [OLI-ABIOTIC-001 #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572), [OLI-M0-001 #4427](https://github.com/Luminous-Dynamics/symtropy/issues/4427), [validator PR #4428](https://github.com/Luminous-Dynamics/symtropy/pull/4428)

## Research purpose

OLI needs known-truth abiotic systems that can exhibit persistent, structured, visually striking organization without being labeled life or intelligence. This increment adds a Gray–Scott reaction–diffusion solver as one synthetic control family.

It is an abiotic mathematical generator. It is not a plasma model, a laboratory instrument, an origin-of-life model, or evidence that any natural system has the same mechanism.

## Implementation

Module: `crates/domains/symtropy-lifesim-core/src/gray_scott.rs`

- CPU-first `f64` reference implementation, no new dependencies.
- Two normalized fields U and V.
- Gray–Scott reaction terms:
  - U: diffusion, autocatalytic consumption, feed.
  - V: diffusion, autocatalytic production, feed/kill removal.
- Explicit Euler integration with fixed timestep.
- Nine-point Laplacian.
- No-flux boundary approximation using clamped neighbour coordinates.
- Bounded [0,1] state values; all numerical clipping is accumulated in the step report.
- Explicit uniform-patch interventions and a seeded noisy-patch initializer; every pseudorandom draw is generated from a supplied seed and is part of the experiment input.
- Per-step accounting for total U/V before and after, reaction transfer, feed input, V removal, net diffusion derivatives and clipping corrections.
- Spatial metrics: mean U/V, V variance, V gradient energy, high-V occupied fraction.

Default parameters:
- (D_u=0.16)
- (D_v=0.08)
- (F=0.035)
- (k=0.065)
- (Delta t=1.0)

The current stability guard requires `dt * max(Du, Dv) <= 0.25` and `dt <= 1.0`. The implementation is a reference finite-difference scheme; it does not claim universal stability or cross-platform bitwise identity.

## Initial falsifiable tests

1. Uniform U=1, V=0 state remains a fixed point.
2. Identical patch, parameters and step count produce identical state and reports.
3. All simulated values and observables remain finite and bounded.
4. A seeded 32×32 patch in an 80×80 field, evolved for 3,000 steps at default parameters, produces at least two four-connected V domains above a pre-registered threshold of 0.1. This tests developed multi-domain pattern formation rather than a merely nonuniform remnant of the initial patch.
5. Invalid dimensions, overflow, parameter ranges, timestep, patch bounds and thresholds fail explicitly.

These are implementation tests, not evidence of a natural phenomenon or qualified solver accuracy.

## How this contributes to OLI

The generator can support matched counterfactuals:
- no perturbation vs localized perturbation;
- different feed/kill conditions;
- different diffusion coefficients;
- short vs long observation windows;
- full vs partial spatial sampling;
- continuous drive vs drive reduction/removal.

OLI can ask whether an apparent pattern is explained by the declared reaction–diffusion dynamics, whether the pattern is externally maintained, and what additional observation would distinguish it from a competing mechanism.

The generator must be registered as `SIMULATION` evidence. The simulator's own outputs cannot be entered as direct laboratory measurements.

## Future qualification work

- Compare the finite-difference solver with a separately implemented numerical reference and a convergence study.
- Freeze the seeded pattern case: 80×80 grid, 32×32 patch at (24,24), U center 0.5, V center 0.25, U/V noise amplitudes 0.2, xorshift64 seed 42, 3,000 fixed steps, and V threshold 0.1 for four-neighbour connected components.
- Freeze parameter sweeps, boundary conditions, grid spacing, timestep, seed patch, and output digests in benchmark manifests.
- Add reaction–diffusion pattern families to OLI's 12-family abiotic look-alike tournament.
- Add perturbation-response benchmark cases with pre-registered passive-relaxation baselines.
- Preserve whole-parameter-regime holdouts so a detector cannot memorize one feed/kill band.

## Claim ceiling

This contribution establishes a deterministic numerical generator and structural tests, once tests pass. It does not establish physical validity against laboratory observations, demonstrate life-like self-maintenance, or support agency, intelligence, or consciousness claims.
