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


## Factorized locomotion transition inference

The separate locomotion_factored module is an experimental dynamic-Bayesian-network representation for leg-local transitions and explicitly declared inter-leg coupling. It does not replace or silently modify the dense SIM-REL-013 JointKernel contract described above.

A factorized transition consists of typed input variables (current state and action), output variables (next state and observations), and one exact-rational conditional probability table per output variable. Each conditional factor carries its own source-manifest IDs and claim ceiling. The compiler canonicalizes semantic IDs, rejects missing or duplicate CPDs, incomplete parent assignments, invalid probability mass, unsupported evidence claims, and cyclic output dependencies.

The inference path performs exact variable elimination over sparse factor tables. Querying a subset of output variables eliminates unrequested variables without constructing the complete Cartesian output state. Dense enumeration is available only behind an explicit cardinality budget, checked before allocation. Intermediate factor tables have their own row budget.

The prototype tests compare factorized marginals against independently enumerated dense joint probabilities on tiny fixtures, demonstrate a planted left-to-right coupling that an independent-leg model misses, and reject malformed CPDs, cycles, missing source references, storage-order changes, and oversized dense materializations. Action variables have a separate role from context inputs: every action requires an intervention-scope contract naming its exact direct target factors and the complete downstream closure allowed by explicit coupling. A direct action-to-non-target-leg factor or an omitted/over-declared coupling descendant fails closed. A synthetic stress fixture with 30 binary outputs has a theoretical dense state cardinality of 1,073,741,824, yet a one-variable marginal is computed under a two-row intermediate-table budget; this measures inference architecture, not biological complexity.

**Scope boundary:** the first prototype is generic categorical inference with a two-leg synthetic coupling fixture and a 30-binary-output independent stress fixture. It is not yet a full six-leg biomechanical simulator, not yet connected to the policy optimizer, and not yet validated against measured fly data. The six-leg per-leg presynaptic state in the locomotion adapter motivates this representation; local joint angles, contact forces, leg-segment state, source-pinned connectome-derived coupling factors, and dense JointKernel conversion for tiny fixtures remain follow-on work. See LOCO-FACT-001 / issue #3677: https://github.com/Luminous-Dynamics/symtropy/issues/3677.

## Locomotion domain adapter

The `locomotion` module compiles stable locomotion-domain identifiers into this crate's exact joint kernel. It keeps the latent tuple as mechanism hypothesis × physical state × sensor state, and does not factorize or prematurely marginalize these components.

### Separate biological and physical channels

Sensor-state metadata has distinct position-encoding, movement-encoding, vibration-sensitive and contact-feedback channels. The FeCO naming convention in the Drosophila literature (claw-/hook-/club-like) is retained as channel semantics, not collapsed into a single proprioceptive gain. Acquisition/instrument bias is a separate field from biological afferent gating.

Physical-state metadata independently carries gait phase, contact condition, load and physical carryover. The action vocabulary distinguishes active/passive probes, position perturbation, movement-afferent gating, vibration perturbation, descending-pathway modulation, contact/load perturbation, acquisition reference calibration and washout. These are typed action labels; the supplied transition probabilities remain explicit model inputs.

### Source and evidence contract

Each source manifest binds source kind, paper/source URI, dataset identity, release/materialization, SHA-256 pin, evidence class, and explicit unknown-endpoint inventory. Measured neural/kinematic/behavioural sources require dataset and release identity; in-silico evidence must be attached to a simulation-code source; connectome evidence must be attached to a connectome source with explicit release/materialization and complete unknown-endpoint inventory. A source digest is syntax-checked and compared to the declared pin; the acquisition/custody pipeline must still verify the actual bytes. Unidentified endpoints remain first-class records with raw IDs and missingness reasons.

Mechanism hypotheses and every joint transition-likelihood row independently declare their source manifest IDs and claim ceiling. The compiled adapter preserves row-level provenance in canonical order alongside the numeric `JointKernel`; source references are not discarded after compilation. A missing transition-row source or a claim ceiling unsupported by its cited manifests fails closed. Connectome-derived anatomy can support an anatomical constraint but cannot satisfy a measured-neural or measured-behaviour claim. Measured trial receipts require a source whose evidence class matches the receipt and require animal/preparation identity. Randomized assignment and actual delivered protocol remain distinct; a delivery mismatch is retained as a failure record and cannot be scored as a valid intervention. The compiled adapter also carries a preregistered minimum delivery-fidelity threshold (0.90 in the synthetic fixture), and each trial receipt records numeric fidelity using exact rational arithmetic. A boolean "verified" flag cannot override a below-threshold delivery. Every receipt binds a SHA-256 digest of the complete pre-outcome history used for its conditional forecast. Batch validation rejects duplicate sample IDs but allows repeated samples from the same animal/session when sample IDs and conditioning-history digests are distinct; this is an identity/ordering check, not proof that the digest correctly represents the biological history. Compilation canonicalizes entities and transition rows by semantic IDs, so storage-order permutations produce identical compiled kernels. Typed intervention semantics are checked for every transition row: position, movement, vibration and contact/load actions cannot silently alter unrelated sensor channels; calibration preserves physical state; washout clears physical carryover without erasing sensor/acquisition state.

### Presynaptic modulation is not sensory-channel loss

The adapter keeps three concepts independent:

- **Afferent channel state:** whether position-, movement-, vibration-, or contact-feedback signals are intact, gated, attenuated, stale, or missing.
- **Presynaptic movement inhibition:** whether movement-encoding FeCO axons are disinhibited or presynaptically inhibited, represented independently for all six legs.
- **Acquisition state:** whether the measurement instrument is calibrated, offset, drifting, or noisy.

This distinction is grounded in Dallmann et al. (Nature, 2025): position-encoding claw axons remain active across behaviours, while movement-encoding hook axons are suppressed during walking and grooming. The authors identify GABAergic interneurons that provide presynaptic inhibition to hook axons and receive descending input; interneuron and descending activity correlates with self-generated rather than passive leg movements. DOI: [10.1038/s41586-025-09554-2](https://doi.org/10.1038/s41586-025-09554-2).

The synthetic state space has six independent presynaptic-inhibition entries. Its action matrix distinguishes active self-generated movement, passive movement, descending inhibition engagement, and descending inhibition release. The synthetic matrix includes front-left and front-right descending interventions. This expands the synthetic sensor-state enumeration to 2,048 states; the joint oracle's sensor-state cap is raised to 4,096, while its independent one-million-row kernel cap remains enforced. The physical fixture now has six whole-animal context/state archetypes: walking stance/swing, grooming supported/unloaded, and other supported/unloaded. The current fixture therefore compiles 294,912 joint transition rows. Active probes engage the target leg's presynaptic state during walking and grooming but not the generic "other" context; passive probes remain disinhibited. These changes preserve intact position, movement and vibration channel metadata. Descending actions must declare both a target leg and target inhibition state. The compiler rejects transition rows that alter non-target legs or unrelated sensory/acquisition variables. A movement-afferent channel gate preserves the entire six-leg presynaptic state. Sensor semantic IDs are derived from and validated against every typed field, including all six modulation entries.

**Claim boundary:** these are synthetic software fixtures informed by a published mechanism. They do not reproduce measured neural-response distributions, prove the complete descending-circuit dynamics, or establish a causal effect in a real fly. A measured-data claim still requires source-locked likelihoods and independent held-out biological validation.

## Synthetic locomotion confounding fixture

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
