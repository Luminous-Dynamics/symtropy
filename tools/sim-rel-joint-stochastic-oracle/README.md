# SIM-REL-013 joint stochastic experimental-design oracle

This dependency-free Rust crate is the bounded exact reference for SIM-REL-013.

## Research shift: one joint latent state, not three convenient marginals

The hidden state is the tuple (mechanism hypothesis, physical/process state, sensor/noise state). The kernel is supplied directly as a joint probability over next physical state, next sensor state, observation, and hazard conditioned on the current joint state and action.

The implementation does not factor this kernel into physical and sensor transitions, and it does not marginalize the belief over mechanism, specimen, or sensor state before an action. This preserves correlations that can determine whether a later experiment is informative.

Every kernel row is a complete finite distribution with exact rational mass one. The initial belief is a joint distribution over the full tuple and also sums exactly to one. A safe transition may change both physical and sensor state. A hazard ends that path. Reset and calibration semantics are ordinary actions whose distinct effects on specimen and sensor state are declared in the kernel and whose cost is charged.

## Exact decision objective

The finite-horizon dynamic program returns the exact Pareto frontier over expected cumulative experiment cost, campaign hazard probability, false-identification probability, and abstention probability.

Correct-identification probability is reported separately; the four terminal probabilities (hazard, false claim, abstention, correct claim) must sum exactly to one. A terminal claim is not assumed correct: the evaluator scores it against the latent mechanism. The caller chooses explicit maximum budgets for hazard, false identification, and abstention; the solver minimizes expected cost under those constraints. It does not hide the trade-off in a weighted score.

The returned policy branches only on public observations. It never receives the true hypothesis, physical state, sensor state, or model identity. Repeated actions are permitted and are evaluated under the current joint state distribution.

## Independent replay and qualification controls

The evaluator independently replays a policy from every initial joint-state possibility and the full kernel. The tests compare the memoized DP frontier against non-memoized exhaustive policy-tree enumeration followed by independent trajectory replay on a small stochastic domain.

Authored tests cover correlation preservation; calibration followed by measurement to resolve persistent sensor/physical confounding; repeated measurement that must not treat persistent sensor bias as IID evidence; exact stochastic probability mass and rational arithmetic; hazard cost accounting; malformed or incomplete kernels and resource caps; and agreement between the DP and an exhaustive tiny-domain policy oracle.

## Qualification

Run formatting, locked unit tests, and strict Clippy using the dedicated workflow. It pins Rust 1.96.0, verifies the exact checked-out SHA, and requires a clean tree.

## Scope limits

This oracle is intentionally small and exact. It does not implement particle filters, SMC-squared, approximate posterior inference, continuous states, model-family discovery, or physical safety certification. Its purpose is to make stochastic policy correctness auditable on finite cases before comparing scalable methods. Exactness is conditional on the supplied kernel, initial belief, costs, horizon, and policy class; it does not establish that these describe a real specimen or sensor.


## Locomotion domain adapter

The `locomotion` module compiles stable locomotion-domain identifiers into this crate's exact joint kernel. It keeps the latent tuple as mechanism hypothesis × physical state × sensor state, and does not factorize or prematurely marginalize these components.

### Separate biological and physical channels

Sensor-state metadata has distinct position-encoding, movement-encoding, vibration-sensitive and contact-feedback channels. The FeCO naming convention in the Drosophila literature (claw-/hook-/club-like) is retained as channel semantics, not collapsed into a single proprioceptive gain. Acquisition/instrument bias is a separate field from biological afferent gating.

Physical-state metadata independently carries gait phase, contact condition, load and physical carryover. The action vocabulary distinguishes active/passive probes, position perturbation, movement-afferent gating, vibration perturbation, descending-pathway modulation, contact/load perturbation, acquisition reference calibration and washout. These are typed action labels; the supplied transition probabilities remain explicit model inputs.

### Source and evidence contract

Each source manifest binds source kind, paper/source URI, dataset identity, release/materialization, SHA-256 pin, evidence class, and explicit unknown-endpoint inventory. A source digest is syntax-checked and compared to the declared pin; the acquisition/custody pipeline must still verify the actual bytes. Connectome snapshots require explicit release/materialization and an unknown-endpoint inventory. Unidentified endpoints remain first-class records with raw IDs and missingness reasons.

Mechanism hypotheses and every joint transition-likelihood row independently declare their source manifest IDs and claim ceiling. The compiled adapter preserves row-level provenance in canonical order alongside the numeric `JointKernel`; source references are not discarded after compilation. A missing transition-row source or a claim ceiling unsupported by its cited manifests fails closed. Connectome-derived anatomy can support an anatomical constraint but cannot satisfy a measured-neural or measured-behaviour claim. Measured trial receipts require a source whose evidence class matches the receipt and require animal/preparation identity. Randomized assignment and actual delivered protocol remain distinct; a delivery mismatch is retained as a failure record and cannot be scored as a valid intervention. The compiled adapter also carries a preregistered minimum delivery-fidelity threshold (0.90 in the synthetic fixture), and each trial receipt records numeric fidelity using exact rational arithmetic. A boolean "verified" flag cannot override a below-threshold delivery. Every receipt binds a SHA-256 digest of the complete pre-outcome history used for its conditional forecast. Batch validation rejects duplicate sample IDs but allows repeated samples from the same animal/session when sample IDs and conditioning-history digests are distinct; this is an identity/ordering check, not proof that the digest correctly represents the biological history. Compilation canonicalizes entities and transition rows by semantic IDs, so storage-order permutations produce identical compiled kernels. Typed intervention semantics are checked for every transition row: position, movement, vibration and contact/load actions cannot silently alter unrelated sensor channels; calibration preserves physical state; washout clears physical carryover without erasing sensor/acquisition state.

### Synthetic locomotion confounding fixture

The adapter tests include a deliberately synthetic case in which:
- a latent gait/contact state is confounded with a persistent acquisition offset;
- an immediate movement probe has zero mechanism EIG;
- a reference-calibration action changes acquisition state without changing physical gait state;
- a subsequent movement probe then distinguishes the latent mechanism hypotheses;
- movement, position and vibration perturbations each change only their declared channel;
- a load perturbation changes physical state without erasing acquisition drift.

This fixture exercises the joint-kernel representation and the limitation of one-step EIG. It is not a reproduction of FeCO physiology, a connectome model, or biological evidence.

### Integration

Compile a `LocomotionKernelSpec` into a `CompiledLocomotionKernel`, then use the existing `optimize_joint_kernel` and `baseline::rank_by_one_step_eig` APIs. The adapter does not implement a second optimizer, posterior estimator, or SOTA harness. The SIM-REL-014 benchmark remains responsible for independent replay, paired baselines, calibration, risk and regret reporting.

## Myopic EIG comparator

The baseline module ranks actions by one-step expected information gain about the mechanism, divided by action cost. It integrates the current joint physical/sensor belief through the same transition kernel, and treats a hazard event as an observed terminal outcome. Its score is floating-point and is deliberately separate from the exact rational optimizer.

The calibration fixture is a useful counterexample: neither the measurement nor calibration action has immediate mechanism information, but calibration changes sensor state so that a subsequent measurement becomes identifying. A one-step EIG policy cannot value that enabling sequence; the finite-horizon joint optimizer can.
