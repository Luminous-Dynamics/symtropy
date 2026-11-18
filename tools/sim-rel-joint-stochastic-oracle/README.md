# SIM-REL-013 joint stochastic experimental-design oracle

This Rust crate is the bounded exact reference for SIM-REL-013. The oracle core remains deterministic and dependency-light; the interactive wire surface uses pinned `serde` and `serde_json` for typed JSONL serialization.

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


## Myopic EIG comparator

The baseline module ranks actions by one-step expected information gain about the mechanism, divided by action cost. It integrates the current joint physical/sensor belief through the same transition kernel, and treats a hazard event as an observed terminal outcome. Its score is floating-point and is deliberately separate from the exact rational optimizer.

The calibration fixture is a useful counterexample: neither the measurement nor calibration action has immediate mechanism information, but calibration changes sensor state so that a subsequent measurement becomes identifying. A one-step EIG policy cannot value that enabling sequence; the finite-horizon joint optimizer can.


## Interactive episode adapter v0.1

The additive `episode::EpisodeProvider` is a headless policy-facing adapter over this exact joint stochastic kernel. It owns the initial joint-state draw, current mechanism/physical/sensor tuple, simulation PRNG, budget, and append-only request attempts.

The public `EpisodeView` exposes an opaque episode identity; non-secret digests for the public contract, model, profile, world recipe, clock and time origin; current discrete step; the public action catalogue and costs; emitted observations; the caller's own attempt dispositions; accepted-attempt/cost counters; trace-completeness state; and terminal status. It does not expose the hypothesis identity, physical state, sensor state, full kernel, sampled initial-state realization, or PRNG seed. The manifest digests are public provenance bindings, not signatures or proof of origin.

Unknown actions, calls after a terminal event, cost-denied requests, attempt-budget-denied requests, and horizon-denied requests are retained as attempts and do not mutate the hidden world. The in-memory receipt log is capped at 65,536 retained requests. Further requests increment `dropped_attempts`, mark `trace_complete = false`, terminate the episode, and return an explicit error. An external runner must still reconcile its own independently maintained request counter; the provider cannot prove that the upstream caller did not omit an attempted request.

The provider now has a versioned serializable process protocol:
- `EpisodeRequest` (trusted runner → policy) contains protocol version and the public view only.
- `EpisodeResponse` (policy → trusted runner) contains protocol version and one `probe` or `finish` command.
- Schemas and examples are under `benchmarks/scientific-world-episodes/v0.1/wire/`.
- The intended Symthaea caller is the additive generic `ProcessAgent::exchange<EpisodeRequest, EpisodeResponse>` in [Symthaea #12758](https://github.com/Luminous-Dynamics/symthaea/pull/12758). That PR is transport-only and does not yet supply the integrated cross-repository episode runner/orchestrator.

The policy process must receive serialized EpisodeRequest/EpisodeView values only; it must never receive an EpisodeProvider instance, the exact joint kernel, the sampled hidden state, the PRNG seed, or EpisodeEvaluation.

The public profile contains the research objective, candidate IDs with structural-family labels/descriptions, action descriptions/targets, and categorical observation meanings/units. Public candidate IDs must be a non-empty subset of the hidden owner-kernel mechanism IDs; public actions and observations must match the owner catalogue exactly. This allows the owner to sample a hidden out-of-catalogue generator without revealing its identity. The request exposes task semantics but not the sampled candidate, latent physical/sensor state, transition probabilities or seed.

A policy may finish with an epistemic status: identified, ambiguous, no-compatible-model, outside-candidate-family, or abstain, together with its **claimed compatible candidate ID set** and a bounded rationale. The trusted provider independently propagates the exact rational joint belief across the recorded action/observation history and compares that candidate set with the exact version space. Evaluator-only output includes the exact candidate set, set-match, correct-candidate count, false-unique, correct-status, safe-abstention and unnecessary-abstention flags. That evaluation is **not** serialized to the policy.

**Claim ceiling:** this is a finite synthetic mechanism-adequacy benchmark with an explicit public candidate family and an evaluator-hidden owner generator. It measures intervention choice, exact candidate-set recovery and OOD/model-family detection. It does not score a free-form mechanism/model description or prove ansatz-free scientific discovery. The candidate family is supplied; do not call this a no-ansatz result. A future genuinely open-world track must generate candidate-independent mechanisms and score proposed model descriptions against sealed held-out interventions or independently measured physical observations.

The episode adapter adds 36 tests for a known-answer confounding world, exact-optimizer-to-provider policy replay, versioned request/response frames, public semantic visibility, candidate-subset validation, exact candidate-set/OOD scoring, false-unique and abstention controls, profile-digest tamper rejection, corrupted-trace rejection, terminal cutoff, receipt overflow, request chronology and sampler resolution. The public predictive catalogue adds 8 tests for exact u128 rational serialization, candidate-kernel reconstruction, task-prior separation, catalogue hash tamper detection, subset scoping, complete transition grids and profile/model order. The policy layer adds 12 tests for no-probe, fixed, seeded-random, metadata-only, public posterior update, one-step joint EIG, OOD empty-version-space detection and malformed-view rejection. The child frame reader adds 3 bounded-JSONL tests. The process integration suite adds 3 tests that launch the actual child executable and evaluate its policy through the trusted parent provider.

**Total: 62 authored tests** across episode, catalogue, policy, binary and process-integration targets. These tests have not yet passed an exact-head Rust run.

The first profile is synchronous and discrete: for each emitted observation, event, acquisition, and availability steps are equal. Each action receipt records its pre-action request step and post-transition execution step separately. It does not simulate delayed acquisition or late delivery. Add those semantics only with an owner model that represents the relevant clock and process evolution.

### Versioned policy wire protocol

The adapter defines `EPISODE_PROTOCOL_VERSION = "symtropy-joint-world-episode-v0"` and two serde frames:
- `EpisodeRequest`: trusted runner → policy; contains protocol version, public episode view, non-secret provenance digests, task description, declared candidate semantics, formal predictive candidate catalogue, actions, observations, attempt receipts and budget.
- `EpisodeResponse`: policy → trusted runner; contains one typed `probe` or `finish` command.

The public JSON Schemas and example frames live under `benchmarks/scientific-world-episodes/v0.1/wire/`. The request's public predictive catalogue includes:
- explicit **task priors** over the declared candidate family, separate from the hidden generator's sampling prior;
- candidate-specific conditional initial physical/sensor-state distributions;
- complete action-conditioned transition distributions, including categorical observations and terminal hazard outcomes;
- exact probabilities encoded as reduced decimal numerator/denominator strings, avoiding binary floating-point loss for u128 values;
- a canonical SHA-256 over the model catalogue, plus a manifest profile digest binding candidate priors, descriptions, action/observation semantics, intervention costs and the catalogue digest.

The public catalogue includes only declared candidate models. The sampled generator ID, hidden initial state, hidden generator models outside the public candidate subset, owner kernel as a whole, PRNG seed and evaluator output remain private. The trusted provider validates the public catalogue against its owner kernel before creating an episode; the child policy independently validates profile/catalogue hashes and reconstructs its own candidate-only kernel from the public frames.

A Symthaea process runner can call `ProcessAgent::exchange::<EpisodeRequest, EpisodeResponse>`; the trusted world runner applies the response through `EpisodeProvider::apply_response`, creates the next request and repeats. The process layer must use a 16 MiB frame limit for this protocol (the generic bridge's default is 1 MiB); the catalogue itself is capped at 8 MiB to leave room for the rest of the episode view. The process boundary is not an OS sandbox.

A finish command reports one of `identified`, `ambiguous`, `no_compatible_model`, `outside_candidate_family`, or `abstain`, plus the policy's claimed compatible candidate IDs and rationale. The trusted provider independently propagates the exact rational joint belief over the recorded action/observation history and scores exact candidate-set recovery, false-unique claims, OOD detection, false positives/misses and abstention. That score is never serialized to the policy.

**Claim ceiling:** SIM-REL-017 gives a policy enough formal information to compute Bayesian posterior updates and one-step joint EIG over the declared model family. It is still a **finite candidate-family model-discrimination benchmark** with a hidden owner generator and OOD controls. It does not accept free-form candidate-independent model programs, and it is not an Unsatz-equivalent ansatz-discovery result. That work is tracked separately under SIM-REL-018.

### Executable baseline policies

The crate includes a bounded process-ready executable, `episode_baseline_agent`, which reads one EpisodeRequest JSONL frame (maximum 16 MiB) and emits one EpisodeResponse frame. It supports five explicit arms:

- `no_intervention`: no experiments; report the public prior candidate set and abstain.
- `fixed_order`: try each affordable public action once in catalogue order, then report the candidate set supported by the actual observations.
- `seeded_random`: reproducibly choose an unattempted affordable action from the public catalogue; seed is an agent-side control, not the hidden world seed.
- `one_step_eig`: reconstruct the public candidate-only joint kernel, update the exact joint posterior using emitted observations, and choose the affordable action with the highest one-step mechanism EIG per unit cost. It may repeat actions when the current posterior makes a repeated intervention informative.
- `metadata_only`: negative control that identifies the first public candidate without using observations.

Example build/run:

```sh
cargo build --locked --manifest-path tools/sim-rel-joint-stochastic-oracle/Cargo.toml --bin episode_baseline_agent
./target/debug/episode_baseline_agent --strategy fixed_order
./target/debug/episode_baseline_agent --strategy seeded_random --seed 23
./target/debug/episode_baseline_agent --strategy one_step_eig
```

The integration tests launch this executable as an actual child process, exchange typed JSONL frames, keep the provider/evaluator in the parent process, and evaluate after the child exits. These are intentionally weak transparent baselines; they do not claim to be model-based EIG policies or learned Symthaea agents.

**Remaining model-based gap:** the new EIG arm is myopic. It does not account for multi-step enabling actions (such as calibration that only makes a later measurement useful), terminal utility, intervention risk limits, model misspecification outside the declared generator family, or formal confidence calibration. The exact finite-horizon joint-state optimizer remains an evaluator-side diagnostic oracle. Next: compare the child one-step policy with the exact frontier on matched kernels; then implement decision-aware EVSI, explicit risk-constrained stopping and counterexample-driven model revision.

### Randomness and claim ceiling

The trajectory sampler uses the named SplitMix64 generator for reproducible runs. It is **not cryptographic randomness** and must not protect a sealed challenge seed from a process that can inspect/control the provider. A blind evaluation still needs an independently isolated runner and oracle custody.

The trajectory sampler converts exact rational weights to f64 and uses a 53-bit uniform draw. To avoid silently dropping positive events below the draw grid, it rejects any positive outcome or initial-state mass below 2^-52. This is a conservative representability guard, not an exact rational sampler; it excludes rare-event kernels from this adapter even though the exact oracle can still analyze them. The exact kernel, finite-horizon optimizer and independent policy replay remain the authorities for exact finite-domain probability calculations. The adapter does not turn a modelled transition into measured evidence, validate a biological/physical model, score a free-form scientific model claim, or control a real experiment.

### Run the adapter tests

```sh
cargo test --locked --manifest-path tools/sim-rel-joint-stochastic-oracle/Cargo.toml --all-targets
```
