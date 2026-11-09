# LOCO-SAFETY-VECTOR-001 — Multi-outcome locomotion safety and proxy-shift audit

**Status:** protocol and synthetic adversarial fixtures. No physical safety or controller-superiority result.

Issue: [#4324](https://github.com/Luminous-Dynamics/symtropy/issues/4324).

## Objective

Prevent a locomotion policy from appearing safer merely by reducing one selected proxy while increasing an omitted adverse outcome or losing measurement coverage.

This is the measurement/benchmark-contract layer. It composes with #3357, #3353, #3475, #4004/#4017/#4230, #4010/#4231, #4058/#4061, #4119, #4044/#4179/#4206, #4304/#4308 and #4310/#4313. It does not replace raw event detection, viability or confidence-interval aggregation.

## Motivation: a current SOTA result exposes the proxy problem

Ovinnikov et al., *Predictive Safety Curricula for Robust Legged Locomotion* (arXiv:2609.37070, 29 Sep 2026), use a distributional safety critic to prioritize terrain contexts and replayable environment events. The paper's ANYmal-D hardware evaluation reports 63% lower shank-collision incidence than learning-progress sampling across three matched training seeds, with 100 crossings per seed and method. The production stair-climbing evaluation reports 33 baseline shank collisions versus zero PSC shank collisions over 50 ascent–descent pairs per method at comparable observed speed.

The authors' project page also reports more foot scuffs under PSC because foot scuffs were not included in its safety cost. That is not grounds to dismiss the result; it is a concrete warning that the measured target and cost taxonomy influence which failures are optimized.

Sources:
- https://arxiv.org/abs/2609.37070
- https://ovinnikov.com/project/quadruped-locomotion/

The paradigm shift is from one scalar safety proxy to a **typed, exposure-normalized adverse-outcome vector with explicit measurement coverage, unknowns, and censoring**.

## V0 taxonomy

Required adverse categories should include:
- fall / unrecovered loss of balance;
- recoverable loss of balance / emergency recovery action;
- foot slip / stance slip / unintended foot motion;
- foot scuff / toe or foot collision;
- shank collision;
- thigh, hip or body collision;
- support collapse / contact loss;
- impact impulse / peak contact force exceedance;
- actuator saturation / thermal derating;
- task failure, route abandonment or emergency stop;
- Unknown / unclassified adverse contact;
- sensor-health, occlusion and measurement-missingness indicators.

A category has stable ID, taxonomy version, adverse flag, event definition, body/object scope, severity unit and required measurement policy. Threshold changes require a new taxonomy or classifier digest and an explicit reviewed migration.

One incident may produce distinct typed outcomes (e.g. a shank strike and an impact threshold exceedance). A canonical event ID identifies one physical event per category. Multiple sensor/channel records must be fused upstream; they must not silently inflate event counts. Keep the physical incident ID to connect different categories from the same incident.

## Matched trial and exposure contract

Every baseline/candidate pair binds:
- robot morphology/configuration and operating domain;
- controller/policy and training-curriculum identities;
- environment, terrain, route, command and perturbation strata;
- taxonomy version/digest and detector/classifier digest;
- sensor set, calibration and time-base health/coverage;
- exposure value and stable exposure-unit ID;
- per-category observed exposure and objective coverage;
- canonical adverse event records with incident ID, category, time and severity.

The matched stratum digest, taxonomy digest, detector digest and exposure unit must agree before a numeric comparison is qualified.

Exposure units are not interchangeable. Crossings, distance, gait cycles, steps, episodes and robot-hours must remain separate unless an independently justified conversion with its uncertainty is supplied. Never treat 100 crossings as equivalent to 100 episodes.

For category k, report raw count N_k and exposure E_k; the exact rate is the rational pair N_k/E_k. The Rust reference compares rates by checked cross multiplication rather than floating-point rounding. Also preserve severity-load per exposure and maximum observed severity; a lower event frequency can coexist with more severe events.

## Proxy-shift and completeness semantics

The audit reports a vector, never a universal weighted safety score.

It flags:
- an adverse category observed but absent from the declared taxonomy;
- required adverse category not measured or only partially observed;
- adverse category omitted from the candidate or baseline safety objective;
- rising Unknown/unclassified events or unobserved exposure;
- incompatible exposure units, detector versions, taxonomy versions or matched strata;
- one adverse category regressing while a selected target category improves;
- event-rate improvement paired with worsening severity load or maximum severity;
- duplicate event identity or conflicting classification;
- zero exposure (rate undefined, never zero).

A complete all-outcome safety-improvement claim requires complete measurement coverage for every required adverse category in both policies, complete classification, matched exposure, matched taxonomy/detector and no adverse-category regression. If one category is unmeasured or objective-omitted, the result may still show observed per-category values but must suppress the complete claim.

This is intentionally stricter than optimizing a reward or a selected collision count. Safety constraints are not tradable against energy, speed or task reward.

## V0 executable scope

The dependency-free Rust auditor consumes a taxonomy, baseline/candidate policy summaries, canonical typed event records, exposure and per-category observed exposure, objective coverage and matched evidence identities.

It will:
- validate taxonomy/policy/event identities and units;
- reject duplicate event IDs and duplicate incident/category pairs;
- calculate per-category event counts and severity sums/maxima;
- compare event and severity-load rates exactly by integer cross multiplication;
- identify missing/unmeasured/Unknown outcomes;
- flag proxy shift when a target category improves while another adverse category regresses;
- suppress an overall improvement claim for incomplete coverage, missingness, taxonomy/detector mismatch or regressions.

It does not classify raw sensor streams, validate detection sensitivity or authenticate event labels. Those remain upstream responsibilities with independent labeled trials.

## Frozen known-answer/adversarial cases

Fixtures are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

1. **PSC-style proxy-shift analogue:** shank collision falls while foot scuff rises; suppress complete safety-improvement claim.
2. All required adverse categories improve under identical exposure and full measurement coverage.
3. Target category decreases while fall rate increases; severe regression stays visible.
4. Event rate falls but severity load and maximum severity rise.
5. Missing/Unknown category and unobserved exposure rise; observed zero is not assumed.
6. Distance and crossing denominators differ; reject direct comparison.
7. Category omitted from the candidate safety objective; objective coverage gap.
8. Taxonomy/detector digest changes; refuse silent cross-version comparison.
9. Same physical incident has distinct category outcomes; preserve both without counting repeated sensor channels twice.
10. Duplicate/conflicting canonical event identity; Invalid.
11. Zero exposure; rate undefined.
12. Event category absent from taxonomy; Invalid/undeclared.

## Matched SOTA / industry comparison

- **PSC vs learning-progress / PLR / standard terrain progression:** hold robot, task reward, terrain-command support, event replay pathway, interaction budget, matched seeds, test exposures and held-out hardware trials constant. Report shank collision and foot scuff separately, plus falls, slip, recovery, actuator and unknowns.
- **STEP-CBF+ vs nominal/risk-only/convex-MPC:** same terrain, route, task, body, scenario distribution and exposure; compare stable-time, dynamic margin, torque/friction utilization, falls and typed contacts separately. Do not infer a robust continuous viability certificate from reduced-order simulation.
- **Impact-aware robust convex MPC vs CMPC/ANCMPC:** keep its single-rigid-body modeling scope and report impact robustness separately from whole-body physical validation.
- **Other CBF/MPC/RL/learned-curriculum systems:** compare success, speed and energy only alongside the full adverse-outcome vector and detector coverage.
- **Symtropy ablations:** full evidence/viability gate; remove active probing; remove set-valued branches; remove recovery; remove the multi-outcome audit; and replace evidence-qualified admission with matched conventional controllers. Same action alphabet, scenario seeds and physical budget per arm.

## Scorecard

Keep independent:
- per-category counts, exposure and event rate;
- severity sum/load per exposure and max/tail severity;
- matched-stratum and taxonomy/detector identity;
- per-category observed exposure and coverage;
- Unknown/unclassified and missingness;
- recovery success and time;
- task completion, route speed and throughput;
- energy/CoT at declared system boundaries;
- actuator thermal/saturation and impact;
- compute, inference latency and deadline misses.

## Proposed targets (not achieved)

- 100% detection of injected proxy-shift, missingness and taxonomy-omission fixtures.
- Zero complete all-safety improvement claims when any adverse category regresses or is unobserved.
- Zero comparisons across incompatible exposure units or detector/taxonomy versions.
- Zero Unknown/missing exposure treated as event-free.
- Exact rate-direction agreement with an independent rational-arithmetic oracle.
- 100% preservation of raw counts, exposure and category-level severity summaries.
- Hardware target: non-inferior or improved results in every preregistered adverse category at matched exposure, with uncertainty intervals and adequate statistical power. No single-category trade hides a regression.

## Claim ceiling

V0 qualifies taxonomy and scorecard completeness over supplied already-classified event records. It does not validate sensor detection, classify raw physical events, authenticate labels or prove robot safety. Hardware superiority requires independent event labels, matched exposure, preregistered adverse strata and a separately qualified physical safety gate.
