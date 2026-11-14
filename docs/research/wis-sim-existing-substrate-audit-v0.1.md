# WIS-SIM existing-substrate audit v0.1

**Status:** source inspection, not an execution report. This audit identifies reusable components and boundaries before any new scenario runner is written.  
**Related:** [WIS-SIM-001](https://github.com/Luminous-Dynamics/symtropy/issues/1731), [WIS-SIM-002](https://github.com/Luminous-Dynamics/symtropy/issues/1798), [draft scenario pack PR #1793](https://github.com/Luminous-Dynamics/symtropy/pull/1793).

## Findings

### 1. Replay exists, but it is physics replay

[`crates/core/symtropy-physics/src/replay.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/core/symtropy-physics/src/replay.rs) exposes a `ReplayTape<D>` made from time steps and ordered `WorldCommand<D>` values (forces, impulses, velocity updates, wake), plus bitwise rigid-body/world snapshots.

**Reuse:** draw from the replay-identity principle and stable exact-state comparison where that physics backend is actually the subject.

**Do not infer:** a physics replay tape is not a generic governance/evidence event log. It has no native types for claims, source authority, consent, minority objections, revisions, group-level outcomes or service correction.

### 2. Ecology substrates are real reuse candidates, with a narrow claim ceiling

- [`symtropy-lifesim-core`](https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/domains/symtropy-lifesim-core) owns a CPU-first dense field grid with moisture, toxin, nutrient, biomass, disease and related channels.
- [`symtropy-colony`](https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/domains/symtropy-colony) models a stigmergic foraging loop, food reserve, trail/pheromone dynamics and environmental danger.
- [`symtropy-mycelium`](https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/domains/symtropy-mycelium) reuses the field substrate for growth, nutrient transport and toxin avoidance.
- These crates expose useful state and metrics for ecological resource/shock scenarios. Their output is not evidence about human trust, civic deliberation, spirituality, consciousness or real community outcomes.

### 3. The existing Village “governance” headless example is not an independent oracle

In [`crates/apps/symtropy-mycelix-village/src/headless.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/apps/symtropy-mycelix-village/src/headless.rs), the demo calls the two synthetic agents “Virtuous” and “Wasteful.” Both receive high `stewardship_care` and `epistemic_integrity`; the loop assigns hard-coded power consumption based on body handle 1 and directly subtracts a penalty from that agent's care field. The validator then checks that the same care value has decreased after tick 500.

This is circular for qualification: the intervention encodes the expected direction and the check observes the field being mutated by that intervention. It may illustrate plumbing, but it cannot establish a causal relationship between waste, cooperation, ethics, or care, and should not be used as a baseline policy benchmark.

A second semantic hazard is in [`CognitiveBrain::phi()`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/bridges/symthaea-bevy-brain/src/lib.rs): its implementation returns `self.profile.epistemic_integrity`. For this bridge, `phi()` is a direct field alias, not an independent validated integrated-information measure. Scenario telemetry must not relabel it as consciousness, virtue or wisdom.

### 4. City-scale plugin is a placeholder, not an existing social model

The public [Village library](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/apps/symtropy-mycelix-village/src/lib.rs) marks `CityScalePlugin<S>` as an intentional no-op and says the ecology stack is not yet wired to it. WIS-SIM must not imply that a full city-scale social simulator already exists.

## Scenario-to-substrate decision

| WIS-SIM family | Existing substrate that may be reused | Missing contract / caution |
|---|---|---|
| WSS-001 commons allocation | LifeSim/Colony resource state as a *domain-specific analogy* | A separate neutral allocation policy and fairness observables; ecological foraging does not itself define civic fairness |
| WSS-002 contested evidence | Existing deterministic replay identity patterns | Typed claims, provenance, conflict and uncertainty semantics |
| WSS-003 minority objection | None identified as an executable policy model in this source audit | Objection lifecycle and group-level outcome receipt |
| WSS-004 repair after conflict | Replay/versioning principles | Immutable decision history, evidence-triggered reconsideration, repair-cost contract |
| WSS-005 adversarial authority | Scenario inputs can declare contradictory sources | A policy adapter with explicit source authority, ground truth and uncertainty, independent from expected outcomes |
| WSS-006 distributional regression | LifeSim exposes field-level/aggregate measurements | Explicit group outcome vectors and missingness state; aggregate-only score is insufficient |
| WSS-007 resource shock | LifeSim/Colony/Mycelium for narrowly ecological shock cases | Fixed event schedule, conservation/recovery assertions and output receipt; results cannot generalize to civic behaviour |

Status in this table is based on source inspection only; no run or fitness test was executed.

## Recommended architecture

1. **Keep the WIS-SIM corpus and policy semantics outside domain engines.** They declare experiment identity, policy matrix, seeds, ground-truth fixtures, typed metrics, invariants and claim ceilings.
2. **Use adapters, not a new universal simulator.** Choose a domain backend only when the scenario actually describes that domain. A symbolic claim/objection scenario should use a small typed policy model, not get forced into rigid-body physics.
3. **Separate policy, ground truth, and evaluator.** The scenario supplies exogenous conditions; the candidate policy produces actions and observations; a separate evaluator checks the resulting report. The evaluator must not mutate the output to make its invariant true.
4. **Make assertions executable.** Prose invariants are documentation, not a pass/fail oracle. The next runner contract needs typed assertion kinds, metric paths, scopes, comparison operators, severity, and explicit semantics for unknown/missing measurements.
5. **Bind every result.** Capture fixture and schema hashes, scenario+variant, policy version, seed, engine/backend revision, exact toolchain, declared environment, report hash, and per-invariant status. Same-input replay identity belongs alongside—not above—model validity.
6. **Return vectors, not a virtue score.** Report per-policy task outcome, resource use, uncertainty changes, objection retention, revisions, recovery cost, subgroup outcomes and missingness independently.
7. **Preserve evidence layers.** Input-shape validation, deterministic runtime behaviour, invariant pass/fail, validity of modelling assumptions, and external/human evidence are separate gates.

## Immediate qualification sequence

- Complete structural verifier execution on the exact PR head.
- Add a strict run-report schema and typed assertion contract before calling any scenario simulated.
- Refactor the existing headless demo as a separate WIS-SIM-002 task; do not silently reuse its current labels/penalty logic as the policy benchmark.
- Run each declared policy against the exact same scenario/variant inputs; report missing runs explicitly.
- Add mutation controls for dropped objections, source-status privilege, unknown-to-verified promotion, changed ground truth, subgroup hiding, changed seed, and altered report/corpus hashes.
- Keep scenario output status at `NOT_RUN` until those commands emit exact-head receipts.

**Claim ceiling:** This audit is based on source review. It does not establish that the existing domain models compile, are deterministic under a declared profile, reproduce ecology, or represent people. No WIS-SIM policy result or human-outcome claim is made.
