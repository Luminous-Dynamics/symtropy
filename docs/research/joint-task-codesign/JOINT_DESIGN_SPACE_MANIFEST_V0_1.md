# Synthetic-joint task co-design: source-faithful design-space manifest v0.1

**Date:** 18 November 2026  
**Owner:** [JOINT-DOE-SEED-001 #6079](https://github.com/Luminous-Dynamics/symtropy/issues/6079)  
**Parent:** [JOINT-TASK-CODESIGN-001 #6064](https://github.com/Luminous-Dynamics/symtropy/issues/6064)  
**Architecture programme:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Status:** source-domain transcription and deterministic DOE seed; no optimizer or physical reproduction claimed.

## 1. Purpose and boundaries

This manifest makes the task-conditioned design space explicit before implementing an optimizer. It distinguishes optimizer decision variables, frozen task inputs, measured/model inputs owned elsewhere, derived quantities, hard constraints and observed responses. A published range is not a proof that every point inside it is manufacturable or feasible.

The initial quantitative domain is the J1 geared leg actuator from Huang, Dong & Zhao, *Task-Oriented Co-Design and Optimization of Geared Actuators for Robotic Applications* (19 September 2026), https://arxiv.org/abs/2609.22795. J2 and J3 are represented as different topology families, but unknown numeric bounds remain explicitly unknown.

Related owners consumed rather than duplicated: #4179 actuator maps; #4206 actuator dynamics; #3357 locomotion task benchmark; #4360 fault-aware morphology; #4516 hinge semantics/qualification; #4728 passivity and interaction ports; #4754 variable-compliance evaluation; #5991 morphology-as-computation attribution; #6078 self-sensing tendon qualification.

## 2. J1 source domain

The following values transcribe the paper's published motor/reducer design-space description. The source has seven slot–pole families (18S/16P, 18S/20P, 18S/22P, 24S/22P, 24S/26P, 24S/28P, 27S/30P), both package bounds (98 mm diameter and 50 mm axial length), and a fundamental winding-factor constraint k_w,1 > 0.9. It also reports seven slot–pole-specific surrogates, each trained from 1,000 valid Latin-hypercube samples over eight cross-sectional geometry dimensions, using standardized quadratic ridge regression, and checked against 200 independent validation cases. The 18S/20P outputs report R² values between 0.9681 and 0.9993; that range must not be generalized to every topology. Full equations, winding enumeration and tooth-count feasibility still require reconstruction before a source reproduction can be claimed.

| Variable | Published domain | Unit | Role |
|---|---:|---|---|
| Stator slot/pole family | 18S: 16/20/22 poles; 24S: 22/26/28 poles | categorical | decision |
| Outer stator diameter | 60–96 | mm | decision |
| Bore diameter | 36.1–75 | mm | decision |
| Tooth width | 2–5 | mm | decision |
| Slot height | 1–19.15 | mm | decision |
| Tooth-tip height | 0.5–2 | mm | decision |
| Slot opening | 0.5–2 | mm | decision |
| Magnet thickness | 1–3 | mm | decision |
| Magnet arc | 120–180 | electrical degrees | decision |
| Motor axial length | 1–30 | mm | decision |
| Winding wire | standard-wire index | index | decision; exact level list must be transcribed |
| Winding symmetry | symmetry divisor | integer/discrete | decision; exact admissible set must be reconstructed |
| Reducer topology | NGW, NW, 3K | categorical | decision |
| Target reduction ratio | 1–50, step 0.5 | ratio | decision; tooth combinations must be feasible |
| Rated current density | 6 | A/mm² | source comparison assumption |
| Actuator package diameter | at most 98 | mm | hard constraint |

**Interpretation:** the listed numeric ranges are source-reported bounds for one geared-actuator formulation. They are not universal motor, wrist, tendon or artificial-muscle limits. Wire indices, symmetry divisors, exact tooth-count sets and several coupled geometry rules are not replaced by guesses in this v0.1 manifest.

## 3. Frozen J1 task reference

| Input | Source value / boundary |
|---|---|
| Physical task | Two-DOF vertical-guide jumping-leg bench |
| Common bench load | 5.5 kg |
| DC bus | 48 V |
| Robot-level interpretation | Not whole-humanoid validation |
| Excluded dynamics | Whole-body balance and lateral dynamics |

Keep the paper's simulated walking/jump/stair torque profiles distinct from the physical vertical-guide bench. Record task traces, normalization, initial condition, loading and evaluator measurement boundary before reproducing a point.

## 4. Topology families are separate search spaces

### J1 — geared leg

Source-defined motor slot/pole family, motor geometry and winding, reducer family and ratio. Complete installed boundary must include motor, reducer, output bearings, housing, encoder/sensors, driver, cooling and cabling where included by the comparison contract. Do not infer missing mass components from nominal motor dimensions.

### J2 — wrist/ankle

At minimum preserve two distinct families: (a) serial pitch-yaw compliant transmission, informed by Tsai et al. (November 2026), DOI 10.1016/j.mechmachtheory.2026.106577; and (b) parallel 3-PRS/SS plus Yoshimura origami coupling and tendon-pneumatic actuation, informed by Hu et al. (15 October 2026), DOI 10.1016/j.mechmachtheory.2026.106545. Numeric ranges for link geometry, workspace, stiffness, pneumatic source, actuation and mass must be extracted from full source protocols or measurements; null is the correct value until then.

### J3 — tendon-coupled finger/gripper

Variables include tendon routing and moment arms, tendon construction, pretension, pulley/bearing friction, compliance, sensor modality, actuator location, differential/coupling topology and local reflex configuration. Numeric ranges remain source-required or measurement-required. Consume #6078 for tendon sensing and lifecycle qualification; do not infer that a tendon-length signal uniquely identifies tension.

## 5. Deterministic DOE seed

The JSON seed uses one-factor boundary/interior probes and categorical coverage. The rows are **coverage probes**, not jointly feasible actuator designs. All unassigned variables remain unresolved. The DOE seed must not be interpreted as a Pareto search or a physical candidate shortlist.

Required coverage:
- all six published slot/pole combinations;
- all three reducer families;
- low, midpoint and high probe for each numeric J1 design variable;
- representative reduction-ratio values at both bounds and midpoint, followed by exact feasible tooth-ratio enumeration;
- explicit out-of-domain and hard-constraint negative controls;
- separate J1, J2 and J3 domains and task manifests.

Before a candidate can be labelled feasible, reconstruct the source geometry and reducer equations, winding constraints, tooth-count feasibility, package boundary and actuator response model. An out-of-domain or unsupported candidate must be rejected or marked unqualified, not silently approximated.

## 6. Objective vector — no single weighted score

Keep distinct objectives and response fields:

- accepted task outcome, failed/abstained trials and worst held-out strata;
- rated and transient peak torque-speed envelopes;
- complete installed mass, volume and manufacturing variance;
- source electrical energy per accepted task, auxiliary energy and regeneration;
- thermal rise, duty and derating;
- reflected inertia, backdrive, friction, backlash and hysteresis;
- stiffness/damping matrix, modulation time, hold/change energy and repeatability;
- embedded-sensing error, uncertainty coverage, observability and OOD rejection;
- fatigue, creep, wear, service time, parts burden and requalification time.

Hard constraints include geometry and package feasibility, admissible reducer/winding combinations, required workspace, task torque-speed envelope, thermal validity, interaction limits and model applicability. Constraints are gates, not objectives that can be traded away.

## 7. Surrogate and evaluation protocol

- First validate the deterministic seed against dimensional/unit invariants and independent analytic cases.
- Preserve a source-faithful J1 reconstruction before extending the model to tendon, compliant or parallel mechanisms.
- Split by design groups and topology families; near-duplicate geometry points must not leak across train/test.
- Hold out at least one topology family and one task condition for transfer evaluation.
- Retain model residuals, interval coverage, invalid simulations and failures.
- Re-evaluate Pareto candidates using a higher-fidelity or independent model, then bench evidence when separately authorized.
- Report design-stage estimate, as-built mass, measured output and task outcome as different data products.

## 8. Entry gates and non-claims

M0 is source/domain transcription. M1 is schema, deterministic DOE replay and analytic known-answer correctness. M2 is source-faithful J1 model reconstruction. M3 is robust task-specific co-design and surrogate validation. Later physical stages require separately approved requirements, hazard analysis, procedures and independent instruments.

This manifest does not implement the optimizer, reproduce the published prototype, establish physical feasibility or authorize fabrication/energization, pressure-system tests, wearable trials or human contact. A copied source range is not an experimentally validated capability.

## 9. Source-method reconstruction details (arXiv:2609.22795v1)

The source paper's J1 search is more specific than a simple list of motor dimensions:

- **Seven discrete motor families:** 18S/16P, 18S/20P, 18S/22P, 24S/22P, 24S/26P, 24S/28P and 27S/30P. The initial v0.1 DOE omitted 27S/30P; v0.1.1 corrects the omission.
- **Eight cross-sectional geometry variables:** outer stator diameter, bore diameter, tooth width, slot height, tooth-tip height, slot opening, magnet thickness and magnet arc. Motor axial length and winding choices are subsequently applied analytically.
- **Per-family surrogate training:** 1,000 valid Latin-hypercube designs per family, standardized quadratic ridge regression, and 200 independent validation cases. The reported R² interval 0.9681–0.9993 is presented for the representative 18S/20P topology and eight outputs; it is not evidence of equal accuracy across all seven families.
- **Outer design search:** constrained NSGA-II, population 48, 30 generations and 1,488 evaluations including initialization per enumerated case.
- **Inner task-policy search:** bounded L-BFGS-B over twelve fifth-order Bézier coefficients, six for hip and six for knee.
- **Task objectives:** predicted slider rise and rated output torque density, kept as separate Pareto dimensions.
- **Hard constraints:** winding factor greater than 0.9, realizable motor geometry/winding, current/voltage operating limits, reducer assembly/clearance/tooth-root strength, package diameter ≤98 mm, package length ≤50 mm, contact constraints and valid task integration.

The current JSON now records these source facts, the task equations and the G1 simulated requirement envelopes. It still does not reproduce the electromagnetic surrogate, reducer sizing equations, NSGA-II run or L-BFGS-B policy optimization. Those remain explicit M2 reconstruction work.
