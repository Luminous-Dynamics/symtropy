# LOCO-COUPLED-FRONTIER-001 — Multidimensional uncertainty and robust viability frontiers

**Status:** protocol and finite synthetic known-answer fixtures. No physical robot safety or locomotion-superiority claim.

Issue: [#4304](https://github.com/Luminous-Dynamics/symtropy/issues/4304).

## Objective

Measure how combinations of contact, terrain, impact, actuator, payload and sensing uncertainty jointly change robust viability and guaranteed recovery.

This is the multidimensional companion to #4130/#4133, which intentionally measures axiswise reserve and forbids inferring a coupled-axis margin from separate one-dimensional sweeps. It consumes outputs from #4061/#4083; it does not compute a competing viability kernel or create another physical-model authority.

## Research hypothesis

For a declared nested family of uncertainty sets U(θ), robust viability generally forms a **sampled downward-closed region** in multidimensional severity space, not necessarily a rectangular box or single scalar margin.

Potential interactions include friction lower-bound degradation × terrain slope; impact impulse × actuator derating; payload/CoM uncertainty × actuator slew/thermal derating; sensing delay × support-phase uncertainty; and external force/moment disturbance × foot-contact uncertainty.

A robot can pass every one-axis sweep at its maximum tested severity and still lose viability under a combined condition. This is the central falsification target.

## Scope and ownership

Consumes, never replaces:
- transition/abstraction digest from #4083;
- robust viable/recovery oracle results from #4061;
- independent result/certificate lifecycle evidence from #4119;
- axiswise reserve samples from #4133;
- energy/actuator identity and accounting from #4044/#4179/#4206;
- episode risk allocation/execution reconciliation from #4245.

Owns only the finite experiment/family schema, canonical vector ordering, sampling coverage, nesting/monotonicity audits, antichains and interaction metrics.

## Parameterization and nesting

Prefer **uncertainty-set severity coordinates** whose increase explicitly widens a declared disturbance/parameter set:
- friction lower-bound degradation / friction interval width;
- contact-impact impulse bound;
- actuator torque/slew derating bound;
- payload or center-of-mass uncertainty radius;
- sensing/actuation delay bound;
- terrain height/slope/roughness uncertainty radius;
- external force/moment disturbance bound.

Each axis includes stable ID, unit, integer levels, uncertainty-set constructor digest, order direction and nesting declaration. Non-nested or non-monotone raw parameters must be stratified/categorical, not forced into a coordinatewise order.

Freeze base state, morphology, model, abstraction, controller/action alphabet, horizon, task, solver, sensor/actuator identity and operating mode within each comparable family. Any change creates a new family identity.

## Mathematical contract

For severity vectors θ in N^d, a family declares whether θ ⪯ θ' implies U(θ) ⊆ U(θ'). For a valid nested family, robust viability should be downward closed:

    θ ⪯ θ' and V(θ') = true  =>  V(θ) = true

Guaranteed recovery R(θ) is checked separately. Do not collapse viability and recovery into one status.

Statuses:
- **Viable:** complete qualified upstream oracle reports the declared robust viability property.
- **Nonviable:** complete upstream oracle returns a counterexample/witness.
- **Unknown:** timeout, resource exhaustion, incomplete graph, unsupported condition or unqualified result.
- **Invalid:** malformed coordinates, duplicate/conflicting point, identity mismatch or malformed receipt.

Unknown is not Nonviable and must never enter the unsafe antichain. It blocks frontier claims through that point.

For a finite sampled grid:
- maximal-sampled-safe points have no strictly larger sampled safe point;
- minimal-sampled-unsafe points have no strictly smaller sampled nonviable point;
- compute antichains separately for viability and recovery;
- report every sample, identities, missing cells, Unknown and Invalid records;
- a comparable pair with lower-severity Nonviable and higher-severity Viable suppresses family qualification and must retain both witnesses;
- never interpolate a continuous threshold from sparse samples.

A finite frontier is a sampled, model-conditional boundary, not proof about every continuous parameter combination or a physical robot.

## Experiment design

Start with two and three dimensions:
1. Reproduce existing one-axis reserve ladders #4133.
2. Build complete small integer grids for friction × impact, friction × slope, actuator derating × payload, and delay × impact.
3. Add selected three-axis slices after pairwise behavior is characterized.
4. Compare complete grids, factorial designs, Latin-hypercube/space-filling designs and adaptive boundary refinement.
5. Freeze held-out points/adverse corners before the sampling policy sees oracle results.
6. Compare uniform, space-filling and safety-critic-prioritized query allocation.
7. Learned predictors (including Predictive Safety Curricula-style safety critics) may prioritize expensive oracle queries, but cannot label points or certify the frontier.

A key negative control is the **separable-margin assumption**: combine one-axis thresholds as if they formed a rectangular safe box. Measure and expose cases where that box contains sampled nonviable combinations.

## Frozen known-answer corpus

The machine-readable cases are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

- **A — 2D staircase:** V(i,j) iff i+j ≤ 2; maximal-safe points (0,2), (1,1), (2,0), minimal-unsafe points (1,2), (2,1). Every axis endpoint is safe but the joint high-high point is not.
- **B — 2D rectangular control:** independent caps produce a rectangular down-set, distinguishing separable from interacting behavior.
- **C — separate recovery frontier:** viability and guaranteed-recovery predicates have different antichains.
- **D — monotonicity contradiction:** a lower-severity point is Nonviable while a comparable higher-severity point is Viable. Suppress qualification and retain witnesses.
- **E — Unknown and coverage hole:** Unknown samples and missing grid cells are not filled or classified.
- **F — 3D antichain:** exact brute-force antichain comparison on a small three-axis lattice.

## Qualification scorecard

Report independently:
- exact known-answer agreement with brute-force finite-grid antichains;
- coordinatewise nesting/monotonicity violations and implicated axis pairs;
- maximal-sampled-safe and minimal-sampled-unsafe antichains;
- viability and recovery frontiers separately;
- tested-grid coverage, holes, Unknown/Invalid fraction;
- interaction penalty versus the separable-margin negative control;
- normalized frontier displacement only within matched unit-compatible family identities;
- safe-volume lower bound only for a completely enumerated grid with declared cell weights;
- oracle calls, compute, memory and resource exhaustion;
- false-safe/false-unsafe witnesses from upstream oracles;
- held-out predictor quality, clearly distinguished from oracle qualification.

Do not collapse robustness, recovery, calibration, task reward, energy and compute into one weighted score.

## Proposed targets (not achieved)

- 100% agreement with independent brute-force antichains on frozen finite fixtures.
- Zero accepted monotonicity contradictions.
- Zero Unknown/Invalid samples treated as Viable or Nonviable.
- Zero cross-family identity substitutions.
- Canonical output invariance under coordinate/sample permutation.
- At least one frozen interaction case where one-axis maxima pass but the combined severity is Nonviable.
- No rectangular separable-margin claim without evidence from the tested domain.
- Publish coverage and missingness before claiming frontier superiority.

## External comparators

- **#4133 axiswise reserve auditor:** one-axis threshold brackets; baseline, not a coupled frontier.
- **STEP-CBF+ (Scientific Reports, 22 Sep 2026):** combines terrain risk, support prediction, barrier screening, energy, torque, friction and posture in reduced-order randomized simulation. Combined-terrain results show trade-offs and non-uniform superiority; averages do not establish robust coupled uncertainty surfaces. https://www.nature.com/articles/s41598-026-72507-w
- **Impact-aware robust convex MPC (IEEE RA-L 2026):** bounded terrain-impact uncertainty with Unitree Go2 simulation/hardware comparisons; compare within its single-rigid-body model scope. https://doi.org/10.1109/LRA.2026.3692086
- **Ashtekar & Dutta (Mechanism and Machine Theory, 15 Oct 2026):** force-tilt and moment-tilt stability margins on 3D terrain, including external forces/moments and changing centroidal angular momentum. This is an instantaneous geometric comparator; our proposed frontier concerns multi-axis hybrid future viability and recovery. https://doi.org/10.1016/j.mechmachtheory.2026.106551
- **Predictive Safety Curricula (Ovinnikov et al., arXiv:2609.37070, 29 Sep 2026):** a distributional safety critic reallocates training experience toward contexts predicted to have high future safety cost. Its controlled ANYmal-D hardware comparison reports shank collisions falling from 29.0 to 10.7 per 100 crossings (63% relative reduction) across three matched training seeds, with 100 one-way crossings per seed/method; production stair-climbing trials report 33 baseline collisions and zero PSC collisions across 50 ascent–descent pairs per policy at comparable observed speed. This is an important **data-allocation** baseline, not a robust viability certificate. The authors' project notes that the safety-cost definition excludes foot scuffs, which increased under PSC; this is a concrete warning that the chosen failure taxonomy changes the apparent result. Use the method to prioritize oracle queries, then qualify each point with the independent #4061/#4083 oracle. https://arxiv.org/abs/2609.37070

## Claim ceiling

This protocol qualifies sampled finite-grid structure for a declared nested family only. It does not prove parameterization nesting, physical abstraction soundness or the correct classification of every continuous point between samples. Hardware claims require validated physical bounds, independent model/abstraction qualification and matched trials.
