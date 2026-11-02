# Risk-weighted evidence assurance for aerospace decisions v0.1

## Purpose

This workstream turns a declared decision-risk context into a minimum evidence profile for a specific engineering claim. It is designed to sit above the run bundle, grid-family study, numerical admission, validation uncertainty and value-of-information modules.

The central principle is that the consequence of being wrong and the model's influence on the decision determine how much credibility evidence is required. This follows the risk-proportional direction in NIST Advanced Manufacturing Series 100-85 (published 5 October 2026), which links credibility effort to model influence and decision consequence and harmonizes simulation VVUQ with metrology practice.

References:
- https://doi.org/10.6028/NIST.AMS.100-85
- https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing

This module is a research policy scaffold. It is not a regulator-approved risk model, airworthiness method, or substitute for applicable certification requirements. The risk inputs are declared by the decision owner; the module does not infer them from a CFD run or assign probabilities.

## Risk axes

A risk context contains four explicit dimensions:

1. **Model influence**: how much the simulation or model output can change the decision.
2. **Decision consequence**: the consequence of a wrong or misleading decision, including an explicit safety-critical class.
3. **Domain exposure**: whether the claim is in-domain, near the validated boundary, or outside the current evidence envelope.
4. **Analysis regime**: steady, dual-time unsteady, time-accurate unsteady, or coupled transient.

The policy deliberately avoids collapsing these into a caller-selected weighted score. The AssuranceTier is a deterministic, monotone mapping from categorical inputs. Raising model influence, consequence or domain exposure cannot lower the required tier.

The current mapping is a proposed engineering policy, not a normative threshold. It should be reviewed with domain experts before being used to gate real aircraft release decisions.

## Assurance tiers and minimum evidence

Every tier requires:

- Exact problem identity and operating-condition declaration.
- Input artifact provenance.
- Protocol conformance.
- Replayable process receipt.
- Native solver termination reason.
- Declared convergence evidence.
- Conservation/consistency evidence.

The policy then raises the floor:

| Tier | Additional minimum gates |
|---|---|
| Screening | Baseline gates only; suitable for early exploration, not design release |
| Standard | Spatial grid convergence, sensitivity analysis, input uncertainty quantification |
| Enhanced | Independent model/solver comparison, experimental validation, blind domain-shift challenge |
| Maximum | Physical confirmation, independent reproduction, independent technical review |

Analysis-regime requirements are additive:

| Regime | Additional required gates |
|---|---|
| Steady | No temporal gate implied |
| Unsteady dual-time | Temporal resolution, per-time-step inner convergence, statistical-window convergence |
| Unsteady time-accurate | Temporal resolution and statistical-window convergence |
| Coupled transient | Temporal resolution and coupled-discipline consistency |

A gate not required by the declared regime is not silently marked as passed. For example, a steady case does not manufacture a temporal-convergence receipt. A dual-time unsteady run cannot substitute a window-mean pass for per-time-step inner convergence.

## Evidence observation contract

Each observation records:

- A typed gate identifier.
- Passed, Failed, or Unknown.
- An immutable source reference.
- SHA-256 of the exact evidence artifact for every passing gate.
- A rationale for failed or unknown evidence.

Admission requires exactly one passing observation for every required gate. Missing, duplicate, failed or unknown required gates prevent admission. A passing observation without a source reference and a 64-character SHA-256 is rejected.

The output means only:

    admitted_under_declared_risk_profile

It does not mean true, safe, certified, flight-worthy, or experimentally validated beyond the gates that were actually required and passed. The report preserves its required-gate list, passed-gate list, blockers and deterministic replay digest.

## Interaction with value of information

The existing VOI module computes one-step EVSI from caller-supplied priors, utilities and likelihood matrices. It explicitly does not calibrate those probabilities. The new risk-assurance layer does not alter that calculation.

The intended orchestration is:

1. Determine the decision's risk context and required evidence gates.
2. Evaluate which gates are already satisfied by replayable evidence.
3. Enumerate candidate actions: refine the grid, extend an unsteady time window, run an independent solver, measure a new operating point, characterize an input, or perform a physical experiment.
4. Estimate the expected decision value and total cost of each action using calibrated inputs where available.
5. Choose the best affordable action among actions that can advance the outstanding assurance plan.
6. Never allow positive EVSI, a better nominal objective, or an optimizer's feasibility claim to override an unmet mandatory gate.
7. Reassess the claim after the new evidence is independently replayed.

When probabilities are not calibrated, use robust bounds, sensitivity analyses or scenario-based worst-case regret rather than invented priors. An unknown likelihood is not a neutral likelihood.

## Paradigm shift

The traditional workflow often ends when the solver stops, the optimizer converges, or an objective improves. This programme should end when the decision is supported to the required level, or when the system explicitly abstains.

That changes the primary optimization target from minimizing drag, minimizing CFD calls, or maximizing nominal Pareto hypervolume to:

- Maximizing validated decision utility.
- Minimizing false-feasible admissions.
- Minimizing expected decision loss under uncertainty.
- Reducing the cost to reach a predeclared evidence tier.

The system should also retain the option to say that the next computation cannot resolve the dominant uncertainty, or that the design lies outside the evidence envelope and physical measurement is needed.

## Benchmark programme

Compare the following evidence-acquisition policies under identical total budgets:

1. Fixed checklist / fixed fidelity schedule.
2. Uniform or random experiment selection.
3. Maximum posterior-entropy selection.
4. One-step EVSI selection using the existing VOI module with independently calibrated likelihoods.
5. Conservative worst-case / minimax-regret selection.
6. Symthaea-guided selection subject to the risk-assurance hard gates.

The action set should include grid refinement, temporal-step refinement, longer statistical windows, solver/model comparisons, input-parameter characterization, new operating points and physical experiments.

Use at least three distinct benchmark families:

- AIAA DPW-8 OAT15A steady and unsteady cases, preserving their exact protocol lanes.
- AePW-4 buffet / aeroelastic / coupled transient cases where the public protocol and experimental references allow comparison.
- DLR Digital Hangar Simple Transonic Wing and DLR-F25 when the necessary models and data are legitimately available.
- ShapeBench's 103 tasks across eight aerodynamic shape categories for broad optimizer generalization, with its benchmark commit and task splits frozen.

References:
- https://www.aiaa-dpw.org/TestCases/testcases.html
- https://aiaa.org/courses/aiaa-aeroelastic-prediction-workshop-aepw-3/
- https://www.digital-hangar.de/collaboration/mdo-benchmark/
- https://arxiv.org/abs/2605.20763

### Primary metrics

- **False-assurance rate**: claims admitted at a tier that fail an independent audit or blind challenge.
- **Decision regret**: realized loss versus the best independently validated decision available under the benchmark.
- **Cost to tier**: measured resources required to satisfy each predeclared assurance tier.
- **Evidence efficiency**: reduction in expected decision loss per unit of measured resource cost.
- **Experiment value realization**: predicted versus realized decision improvement from the selected experiment.
- **Calibration**: coverage, interval width and subgroup/domain-shift calibration for nominal 90% and 95% intervals.
- **Robust feasible yield**: admitted candidates that remain feasible under blind geometry and operating-condition shifts.
- **Abstention quality**: rate at which the system abstains on genuinely unsupported claims versus needless abstention on in-domain cases.
- **Replay integrity**: exact artifact binding, deterministic offline replay and independent verifier agreement.
- **Resource completeness**: percentage of comparisons with measured wall time, CPU, memory, GPU and energy; unknown values remain unknown.

Report task-level paired outcomes, multiple seeds, confidence intervals, median, worst decile, every failed/aborted/retried evaluation, and all exclusions. Include the full human and compute cost where it materially affects the policy comparison.

### Pre-registered research targets

These are targets, not achieved performance:

| Measure | Initial target |
|---|---|
| Required-gate integrity | 100% of admitted claims have one source-bound pass for every required gate |
| Tamper detection | Zero altered or cross-claim evidence objects accepted |
| Risk monotonicity | Raising any risk axis never lowers the required assurance tier |
| False assurance | Zero false admissions on the locked adversarial suite |
| Generalization | At least three benchmark families, with geometry and operating-condition blind holdouts |
| Decision efficiency | At least 20% lower measured cost to reach the same validated decision quality; 30% stretch |
| Worst-case integrity | No increase in worst-decile regret or false-feasible rate versus the strongest baseline |
| Calibration | Nominal 90% and 95% intervals assessed with subgroup coverage and width reported |
| Abstention | Unsupported claims withheld; no hidden conversion of unknown evidence to pass |

The 20% cost target is not met by lowering fidelity, relaxing a required gate, dropping failed runs or changing the design constraints.

## Industry comparison

The proposed system should complement established tools rather than replace their strongest capabilities:

- **SU2 v8.6.0**: CFD, adjoints, native convergence and solver physics. Symtropy's opportunity is protocol-aware termination, independent evidence replay and decision-level assurance.
- **NASA FUN3D 14.3**: high-end unstructured CFD, adjoints and adaptation where access is authorized. Symtropy's opportunity is cross-solver evidence interoperability and matched-budget comparisons.
- **OpenMDAO 3.45.x**: coupled analysis and gradient-based MDO. Symtropy's opportunity is typed evidence dependencies, claim-specific admission, risk-proportional VVUQ and abstention.
- **OpenVSP/VSPAERO and NASTRAN**: geometry and structural-analysis disciplines. Symtropy's opportunity is preserving cross-discipline identity and evidence through aero-structural/mission decisions.
- **ShapeBench and DLR-F25**: external benchmark suites. Symtropy's opportunity is to demonstrate generalization and cost-to-decision improvements across heterogeneous families, rather than win one isolated case.

Do not claim superiority over these tools based on the existence of a Rust evidence layer. A credible claim requires matched protocols, strong baselines, equal full budgets, blind holdouts, independent reproduction and physical comparison where the claim requires it.

## Next integration steps

1. Bind AssuranceReport to the existing CFD numerical-admission and validation-uncertainty reports by exact source digest.
2. Add explicit requirement profiles for mission, structural, aeroelastic and manufacturing claims.
3. Add an action catalog that links each unmet gate to admissible next measurements without allowing optimizer-only evidence to satisfy physical gates.
4. Extend the one-step EVSI policy to sequential, budget-constrained evidence planning only after likelihood calibration and robustness tests are available.
5. Benchmark the assurance planner against fixed, entropy, EVSI and minimax-regret policies on blind DPW-8/AePW/DLR/ShapeBench families.
