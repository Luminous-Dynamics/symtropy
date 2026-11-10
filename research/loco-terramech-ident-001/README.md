# LOCO-TERRAMECH-IDENT-001 — History-aware foot–terrain identification and gait adaptation

**Status:** benchmark protocol and synthetic fixtures. No physical locomotion result.

Issue: [#4430](https://github.com/Luminous-Dynamics/symtropy/issues/4430).

## Paradigm

A footfall may change both the robot and the ground. On a yielding substrate, today's contact changes tomorrow's sinkage, shear response, bearing capacity and energy cost. A terrain label or instantaneous friction scalar is therefore insufficient when the selected physical process is history-dependent.

This benchmark studies whether qualified foot–terrain evidence and Terrain-owned constitutive memory improve held-out prediction and safe gait adaptation across rigid/granular transitions.

## 2026 direct comparator

Guo et al., *Terramechanics-Aware Adaptive Quadrupedal Locomotion via Foot-Terrain Interaction Model*, IEEE Robotics and Automation Letters 11(11), 2026, DOI [10.1109/LRA.2026.3732903](https://doi.org/10.1109/LRA.2026.3732903). The paper combines a virtual-ground plastic-yield model (nonlinear sinkage, shear saturation, progressive bearing-capacity loss) with gait adaptation from contact-history estimates of friction/deformability. It reports Isaac Sim, MPM-based Newton Physics and Unitree Go2 experiments, including loose-sand footprints, sand-to-tile transition and natural sandy-beach trials. It is our closest comparator; Symtropy has not reproduced or surpassed it.

Additional baselines:
- IRCMPC (IEEE RA-L 2026, DOI [10.1109/LRA.2026.3692086](https://doi.org/10.1109/LRA.2026.3692086)): bounded terrain-impact uncertainty in SRB dynamics, robust min–max MPC, Go2 hardware. Strong impact baseline, not a persistent soil constitutive model.
- STEP-CBF+ (Scientific Reports 2026, [DOI 10.1038/s41598-026-72507-w](https://doi.org/10.1038/s41598-026-72507-w)): reduced-order foothold/posture screening, terrain-risk memory, friction/torque feasibility and energy-aware selection; about 13.6 ms at 49 candidates in its declared simulation benchmark. It is not a whole-body invariance proof or hardware qualification.
- Jia et al. (2026 contact survey, [DOI 10.1007/s10462-026-11644-0](https://doi.org/10.1007/s10462-026-11644-0)): D1 interface pressure/shear/geometry, D2 near-foot wrench/vibro-acoustic, D3 proprioceptive inference; controller-facing F1 contact modes, F2 wrench/CoP, F3 uncertainty-qualified feasibility.
- RAIBO2 (Nature 2026, [DOI 10.1038/s41586-026-11102-5](https://doi.org/10.1038/s41586-026-11102-5)): one-charge marathon and reported CoT 0.25. Energy-system reference only; not directly comparable across different robot, boundary or substrate.

## Ownership and non-duplication

| Responsibility | Existing authority | This benchmark consumes |
|---|---|---|
| Surface facts vs body-relative interaction | LENV #1241 | Qualified surface, body, foot, load and action profiles |
| Process-specific hysteresis/recovery state | LENV #1248 | Future-bearing state semantics, not an event log |
| Shared contact cause and owner-specific ground/ecology response | LENV #1266 | Qualified contact cause and response profile identities |
| Canonical ground disturbance/constitutive state | Terrain #1294 | Terrain-owned state, revision, profile and settlement receipts |
| Material/contact law profile | Physics #473 | Material identity, friction/contact profile and applicability |
| Independent contact-law assurance | Physics #3183 | Contact-law oracle and solver receipts |
| Generic calibration/identifiability | Physics #3784 | Sensitivity, rank, parameter and experimental design semantics |
| Graded/contact information and active probing | Locomotion #4143/#4230/#4004 | Time-qualified D1/D2/D3 evidence and action-conditioned information |
| Robust viability/recovery and certificate validity | Locomotion #4058/#4061/#4083/#4119 | Admitted physical probes/actions and recovery results |
| Viability-conditioned decision value | Locomotion #4310/#4313 | Post-probe decision value over admitted continuation sets |
| Task/degraded-loop benchmark | #3357/#3475/#4327 | Matched locomotion performance and held-out tests |
| Adverse-outcome vector | #4324/#4326 | Falls, slip, scuff, collisions, impacts, Unknown/missingness |
| Energy/actuator | #4044/#4179/#4206 | Declared electrical/mechanical and thermal boundaries |

This workstream owns experiment identity, model tournament, identifiability reporting, held-out validation and the combined locomotion scorecard. It must not add a second ground-history state store inside the locomotion stack; mutate Terrain from reward, policy output or estimated soil state; make a collision callback canonical environmental-effect authority; promote estimated friction or soil parameters directly into movement authority; or replace upstream robust viability, certificate-lifecycle or contact-law owners.

## Research hypotheses and falsifiers

1. Two substrates with the same nominal label and similar first-contact force can diverge after repeated loading.
2. The order and magnitude of normal load, shear, dwell and recovery can change future response for a genuinely path-dependent material.
3. An elastic reference must recover without permanent disturbance; a rigid reference must remain NoEffect within declared tolerances.
4. A model calibrated on one specimen/location can fail on another despite matching material labels.
5. A foot/body/action-specific interaction estimate must differ when contact area, load distribution, compliance or approach profile changes.
6. Contact-history adaptation can improve transfer but must still pass independent physical admission and recovery gates.

## Scenario matrix

Freeze sample preparation and exact sequence manifests for:
- rigid reference / hard rock;
- reversible elastic compliant reference;
- dry granular substrate;
- moisture-conditioned granular substrate only where moisture measurement and source are qualified;
- repeated normal loading and dwell/recovery sequences;
- repeated normal-plus-shear sequences;
- rigid-to-granular and granular-to-rigid transitions at multiple gait phases and approach speeds;
- confounded substrate parameter sets that look alike at first contact but diverge after a designed intervention;
- sensor-family ablation/delay/corruption (D1, D2 and D3);
- actuator derating combined with substrate transition;
- out-of-domain substrate, which must produce Unknown/NeedsRicherModel/abstention;
- untouched control patch to detect uncaused ground-state drift.

Hold out whole physical samples, prepared batches or field locations. Do not randomly split adjacent timesteps from the same physical sequence across calibration and test.

## Data and evidence contract

Each contact/experiment record binds robot morphology, foot geometry/configuration, mass/load profile, material/ground-history model profile, specimen/location/preparation batch, moisture/temperature, action/gait phase, approach speed, commanded and delivered joint motion, D1/D2/D3 source/calibration/time identities, force/pressure/footprint metrology, solver/engine versions, timestep/convergence, exposure, initial ground revision, outcome and calibration/validation/held-out role.

A missing force sample is not zero force. A video-rate kinematic trace is not a physics-step wrench. Streams may be joined only through verified time-base receipts.

## Model tournament

Same raw evidence, training/calibration budget, interaction budget, robot and held-out sequences for:
1. rigid Coulomb contact / blind RL;
2. elastic compliant contact without plastic history;
3. instantaneous empirical force/sinkage and shear curves without history state;
4. version-pinned Bekker/Wong-style pressure-sinkage and shear model only where its assumptions/applicability are declared;
5. Guo et al. virtual-ground plastic-yield plus contact-history gait adaptation;
6. MPM-based Newton Physics as an independent high-fidelity numerical comparator, not presumed physical ground truth;
7. Symtropy using Terrain-owned history state + LENV body-relative interaction + qualified contact evidence;
8. model-adequacy/abstention baseline which returns Unknown when the selected profile is out of domain.

### Learned reduced dynamics comparator: NeDM Go2 CRM Study 4

A second direct baseline is the Go2 CRM granular-terrain study attached to Zhang & Negrut, *Learning the Right Abstraction: Neural Reduced Dynamics for Complex Robot Control* (arXiv:2608.19375, 19 Aug 2026).

- Public dataset card: https://huggingface.co/datasets/ksha23/nedm-study4-go2-crm
- Reproduction repository: https://github.com/uwsbel/NeDM/tree/kyle/quadruped-pipeline
- Reproduction code pin inspected for this protocol: branch `kyle/quadruped-pipeline`, commit `18c9316ad831d5dc0196ad94a23224b6aa19df9d` (02 Oct 2026).
- The dataset card identifies CC-BY-4.0 for the corpora, surrogates, fine-tuned policies and scoring records. The base policy is Apache-2.0 and the Go2 model BSD-3-Clause. The GitHub repository metadata currently has no declared repository license; do not vendor or copy its implementation without a separate rights review. Access to the Hugging Face files requires accepting the dataset access agreement. The dataset's revision SHA must be pinned at retrieval; the published MANIFEST.tsv supplies per-file SHA-256, sizes, source run and machine.

**Published experimental baseline:** 1,150 CRM episodes (920 train / 230 validation, split by episode), 2,041,752 rows at 100 Hz, about 5.7 h. The base policy walks Chrono's CRM/SPH granular soil. Its NN reduced model uses a 36-D control-interface state and 12-D action to predict 10 ms state deltas; transformer 6 layers × 8 heads × 256 width, context 128 rows (1.28 s), approximately 4.85M parameters. Training is one-step followed by 1 s rollout training. PPO then fine-tunes the existing policy inside the frozen surrogate using 1,024 branches of 2 s, initialized from base-policy training states. Every candidate is scored again in full Chrono.

The recipe at PPO iteration 1,000 uses ten independently trained surrogate seeds and full-Chrono paired scoring on 40 held-out command paths (37 usable; three are too wide for the finite bed and are skipped in every arm). The dataset card reports mean tracking-error reductions of 52% forward, 67% yaw and 28% lateral. These are per-study tracking metrics, not reductions in falls, physical risk or energy. The matched rigid-ground tests report no performance regression. The detailed reproduction README describes the metric as per-episode mean absolute velocity error after the first 0.5 s, with paired percent change calculated against the base policy.

**Critical limits that must survive comparison:**
- This is a local, one-round offline improvement to one existing policy; it is not a replacement for Chrono, an online controller, or a general terrain-identification result. Direct fine-tuning in Chrono was not run, so do not claim matched wall-clock savings against a full-Chrono PPO experiment.
- Training/branch starts come from states visited by the base policy in the training corpus. The authors report only 0.0–0.1% policy visitation outside the corpus. This is local occupancy, not demonstrated distribution-free transfer.
- The model is explicitly accurate near the corpus. The guard is an early warning, not a certificate; one collapse occurred in roughly 60 runs. Harder 240–300 N push tests exceed the corpus's 140 N push range; disturbance training inside the surrogate did not solve that coverage gap.
- The 36-D control-interface closure was the strongest state preset in its channel ablation. Adding four foot-normal forces, four foot sinkages, both, or full 3-D foot forces (40/44/48-D) did not improve average transfer in this particular study. Treat this as a serious negative control against the assumption that adding contact channels always improves a learned model—not as proof that contact sensing is unhelpful. Test whether channel value depends on prediction horizon, physical target, sensor health and the decision being supported.
- CRM trajectories are high-variance. The reproduction team's own evaluation note withdrew an earlier machine-effect claim after widening spawn variation; it reports CRM tracking SD around 5.7 percentage points at ±1 m spawn variation, versus about 0.1 on rigid ground. Use broad preregistered episode variation, paired per-seed differences and uncertainty intervals; a single CRM rollout is diagnostic only.
- The original 36-D model uses a particular CRM discretization. Its soil notes state the Go2 foot spans about 2.2 particles while the HMMWV tire spans about 11.8; Go2's contact length scale is therefore near/below the soil smoothing-kernel scale. Require particle-spacing / smoothing-length / foot-geometry convergence before treating its numerical sinkage as quantitatively validated terramechanics.

For Symtropy, reproduce the **frozen control-interface baseline first**, then test set-valued uncertainty, history-bearing ground-state inputs, contact-force/sinkage channels, D1/D2/D3 degradation, longer rollout horizons and held-out substrate/actuator shifts as explicit ablations. Do not improve the model by silently giving it more privileged state than a baseline receives. Preserve the evidence boundary: the NeDM dataset is simulation/Chrono data, not physical Go2 beach-surface ground truth.

Controller tournament:
- blind RL/fixed gait;
- contact-history gait adaptation without active probing;
- EIG/Fisher-only selector after identical physical admission (negative control);
- fixed-state VOI followed by robust continuation gate;
- viability-conditioned VOI (#4313);
- IRCMPC impact-aware robust MPC;
- STEP-CBF+ reduced-order screen;
- Symtropy full stack and ablations.

No method may use hidden specimen identity, evaluator labels, future observations or privileged ground state at test time.

## Identification and uncertainty requirements

- Declare each parameter's meaning, unit, prior/range and model applicability before fitting.
- Report scaled sensitivity, effective rank, singular values and confounded parameter directions.
- If observations cannot distinguish parameter sets, report an equivalence set or abstain; do not output falsely precise friction/cohesion/deformability point estimates.
- Separate parameter fitting, model-family selection, uncertainty calibration and final validation.
- Calibrate predictive intervals on independent physical episodes/specimens. Repeated control ticks from one rollout are not independent calibration samples.
- Select active experiments only after every possible probe prefix and post-probe continuation passes #4058/#4061/#4083/#4119.
- Retain joint sensor/parameter/contact dependence; do not multiply independent marginals without qualified factorization.
- Test future-contact prediction after repeated loading and transition—not only the contact used to fit parameters.

## Metrics (never collapse into one score)

**Constitutive prediction:** force–sinkage curve error/coverage; shear-force–displacement curve error/coverage; residual footprint geometry error; bearing-capacity loss; recovery/relaxation prediction; plastic vs elastic classification; held-out specimen transfer.

**Locomotion:** slip distance, stable-step fraction, contact-mode transition error, body-state estimate, rigid/soft adaptation time, task completion, sustained speed, distance/time-to-failure, recovery success/time.

**Adverse outcomes:** falls, stance slip, foot scuff, shank/body collision, impacts, emergency recovery, contact loss, actuator derating and Unknown/missingness by #4326.

**Resources:** probe energy/time, electrical energy at declared boundary, mechanical work, substrate dissipation, CoT only for comparable system boundaries, actuator torque/slew/thermal utilization, latency, deadline misses and memory.

**Assurance:** physical-branch completeness, robust viability/recovery disposition, evidence freshness/identity, model-domain status and abstention.

## Frozen external baseline manifest and known-answer fixtures

The external comparator inventory is machine-readable in [references_v0_1.json](references_v0_1.json). It pins the NeDM code branch to commit `18c9316ad831d5dc0196ad94a23224b6aa19df9d`, records dataset access/licensing conditions and its known limits, and explicitly leaves the Hugging Face dataset revision SHA as **unresolved until authorized acquisition**. Do not treat an unpinned data snapshot as a reproducible raw-data result.

Machine-readable synthetic falsifiers are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json). The 20 cases cover:
- same first-contact output / different repeated-load future response;
- rigid NoEffect and elastic recovery negative controls;
- plastic sinkage with shear-model misspecification;
- same impulse / different material response;
- changed contact area/load profile;
- non-identifiable parameter equivalence;
- correlated sensor channels and missing D2 force;
- stale/mismatched Terrain settlement receipt;
- high-information probe with no recovery;
- zero-mass physically possible dead-end branch;
- held-out specimen/batch leakage;
- adaptation that improves task success but worsens foot scuff/slip;
- NeDM surrogate's local base-policy occupancy and unsupported high-push extrapolation;
- study-specific contact-channel ablation (extra force/sinkage dimensions did not help its transfer);
- CRM replicate variance and single-run overconfidence;
- particle/foot length-scale mismatch;
- data-access, dataset-revision and implementation-license boundaries.

These are synthetic semantic fixtures, not empirical physical observations. Literature/data-card results remain external claims and are not represented as Symtropy measurements.

## Proposed qualification targets (not achieved)

- 100% exact fixture and identity validation.
- 100% detection of stateless/path-dependent confounding fixtures.
- Zero canonical ground-state change without an independently qualified contact cause and Terrain-owner settlement.
- Zero point-identification claim for non-identifiable parameter directions.
- Held-out predictive coverage at preregistered levels with specimen-clustered uncertainty intervals.
- Zero probe/decision-value bypass of robust physical admission.
- No aggregate safety or energy claim can hide a regression in any required adverse outcome.
- Stretch: lower held-out constitutive prediction error and faster safe gait adaptation than the best matched baseline under equal probe, energy, compute and physical-risk budgets.

## Claim ceiling

This benchmark qualifies model identifiability and prediction against supplied observations. It does not prove a universal soil law, treat MPM as physical truth, prove whole-body viability, or demonstrate deployment superiority. Those require independently prepared/characterized substrates and held-out physical robot trials.


## Executable finite Rust V0 reference

The standalone dependency-free Rust crate at `research/loco-terramech-ident-001/` validates the **experiment evidence manifest**, not terrain mechanics.

It consumes:
- whole-specimen assignments to calibration/validation/test;
- preparation-batch and independent collection-cluster identities;
- declared D1/D2/D3 sensor sources, model/calibration digests and native time-base mappings;
- per-contact explicit channel coverage (Present or Missing with a reason);
- per-channel observations with raw/corrected/arrival times, uncertainty, units and source identity;
- ordered contact events and Terrain-owned before/after revision/settlement receipts.

It verifies:
- specimen, preparation batch and independent cluster do not cross splits;
- multiple specimens may share a preparation batch/cluster within one split;
- sequence/event/observation identity uniqueness;
- source/calibration/mapping/unit identity and time-window/uncertainty qualification;
- raw and corrected timestamps increase within each source, while different channels need not be synchronized;
- every required channel is explicitly Present or Missing for every contact event;
- unrequested channels cannot be silently added to a result;
- NoEffect, Unknown and OutOfDomain do not advance the Terrain revision;
- Settled effects carry unique receipts and strictly advance the revision chain;
- Unknown/missingness counts remain explicit in the validation report.

The 17 Rust tests cover split leakage, same-split specimen sharing, identity and timing failures, missing-channel semantics, ground revision continuity, settlement receipt reuse and the no-silent-mutation boundary.

Qualification commands:

    cargo fmt --manifest-path research/loco-terramech-ident-001/Cargo.toml -- --check
    cargo test --manifest-path research/loco-terramech-ident-001/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-terramech-ident-001/Cargo.toml --locked --all-targets -- -D warnings

The exact-head workflow is authoritative for Rust 1.96. A green run validates only the supplied manifest/evidence relationships. It does not authenticate external receipts, classify raw contact signals, estimate soil parameters, or validate the physical response model.
