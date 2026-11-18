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


## Myopic EIG comparator

The baseline module ranks actions by one-step expected information gain about the mechanism, divided by action cost. It integrates the current joint physical/sensor belief through the same transition kernel, and treats a hazard event as an observed terminal outcome. Its score is floating-point and is deliberately separate from the exact rational optimizer.

The calibration fixture is a useful counterexample: neither the measurement nor calibration action has immediate mechanism information, but calibration changes sensor state so that a subsequent measurement becomes identifying. A one-step EIG policy cannot value that enabling sequence; the finite-horizon joint optimizer can.


## Interactive episode adapter v0.1

The additive `episode::EpisodeProvider` is a headless policy-facing adapter over this exact joint stochastic kernel. It owns the initial joint-state draw, current mechanism/physical/sensor tuple, simulation PRNG, budget, and append-only request attempts.

The public `EpisodeView` exposes an opaque episode identity; non-secret digests for the public contract, model, profile, world recipe, clock and time origin; current discrete step; the public action catalogue and costs; emitted observations; the caller's own attempt dispositions; accepted-attempt/cost counters; trace-completeness state; and terminal status. It does not expose the hypothesis identity, physical state, sensor state, full kernel, sampled initial-state realization, or PRNG seed. The manifest digests are public provenance bindings, not signatures or proof of origin.

Unknown actions, calls after a terminal event, cost-denied requests, attempt-budget-denied requests, and horizon-denied requests are retained as attempts and do not mutate the hidden world. The in-memory receipt log is capped at 65,536 retained requests. Further requests increment `dropped_attempts`, mark `trace_complete = false`, terminate the episode, and return an explicit error. An external runner must still reconcile its own independently maintained request counter; the provider cannot prove that the upstream caller did not omit an attempted request.

This is currently a typed in-process Rust seam only: it does not yet serialize the generic public-packet JSON Schema, implement the parent/child JSONL process bridge, or connect a Symthaea policy. Those integrations belong to the existing provider and Symthaea #11171 owners. The policy process should receive an EpisodeView projection only; it must never receive an EpisodeProvider instance.

The 18 authored adapter tests include a known-answer confounding world: two private initial tuples produce the same first combined-signal observation; a sensor-calibration action leaves their public histories aligned; only a later measurement separates the physical states. A further cross-layer test obtains a finite-horizon policy from the exact joint optimizer and replays it through the interactive provider for both hidden initial states, checking that the calibration branch and later discriminating observations agree. They also cover public manifest validation, pre-action versus execution timestamps, receipt-cap overflow, rejected requests not consuming randomness, and sampler-resolution rejection. These are authored tests, not passing results until the workflow runs on the exact head.

The first profile is synchronous and discrete: for each emitted observation, event, acquisition, and availability steps are equal. Each action receipt records its pre-action request step and post-transition execution step separately. It does not simulate delayed acquisition or late delivery. Add those semantics only with an owner model that represents the relevant clock and process evolution.

### Randomness and claim ceiling

The trajectory sampler uses the named SplitMix64 generator for reproducible runs. It is **not cryptographic randomness** and must not protect a sealed challenge seed from a process that can inspect/control the provider. A blind evaluation still needs an independently isolated runner and oracle custody.

The trajectory sampler converts exact rational weights to f64 and uses a 53-bit uniform draw. To avoid silently dropping positive events below the draw grid, it rejects any positive outcome or initial-state mass below 2^-52. This is a conservative representability guard, not an exact rational sampler; it excludes rare-event kernels from this adapter even though the exact oracle can still analyze them. The exact kernel, finite-horizon optimizer and independent policy replay remain the authorities for exact finite-domain probability calculations. The adapter does not turn a modelled transition into measured evidence, validate a biological/physical model, or control a real experiment.

### Run the adapter tests

```sh
cargo test --locked --manifest-path tools/sim-rel-joint-stochastic-oracle/Cargo.toml --lib episode::tests
```
