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


## Model-indexed robust replay (SIM-REL-015)

The robust module evaluates one identical public-observation policy tree against a finite family of complete joint stochastic kernels. A kernel identity remains fixed for an entire campaign; the policy cannot inspect a model-world ID. Each world is replayed independently, with exact per-world expected cost, hazard, false-identification, abstention, and correct-identification probabilities.

The report exposes exact worst-case cost and risk summaries and checks explicit worst-case hazard, false-claim, and abstention budgets. The worlds must share hypothesis, action, observation, physical-state and sensor-state semantics, plus the same action-cost contract; each world may carry its own validated initial joint belief and transition kernel.

This is a robust evaluator, not a robust policy optimizer. It does not choose a minimax policy. Robustness is only over the supplied finite world family and does not establish that the true data-generating process is covered.


## Shared-policy robust optimizer (SIM-REL-016)

The robust optimizer performs finite-horizon dynamic programming over a vector of per-world joint beliefs. Each public observation updates every world under its own complete kernel. Worlds assigning zero probability to a branch are inactive on that branch, while their model identity remains fixed; no model switching or rectangularization occurs.

The optimizer preserves per-world objective vectors during Pareto pruning. This is required because observation probabilities differ across worlds: pruning only by current worst-case aggregates inside a subtree can discard a globally optimal shared policy. At the root, callers can minimize worst-case expected cost or nominal expected cost subject to worst-case hazard, false-identification and abstention budgets.

The implementation is bounded and exact for the supplied finite model family, prior beliefs, action costs, horizon, and policy class. It is not evidence that the real process belongs to that family.


## Reproducible baseline benchmark runner (SIM-REL-017)

The benchmark module constructs four concrete baseline families from a nominal joint kernel: fixed action order, seeded random order, static one-step EIG ranking, and receding-horizon greedy one-step EIG. It then replays every unchanged policy across all supplied model worlds and reports exact per-world/worst-case cost, hazard, false identification, abstention and correct identification.

The same explicit risk/false-claim/abstention budgets are applied to each baseline and to the exact robust shared-policy optimizer. Where a baseline is feasible and an exact robust optimum exists, the harness reports exact additional worst-case cost (regret). It records the random seed and horizon. The EIG score itself is floating point, while all replay outcomes and costs remain exact rational values.

The calibration-confounding fixture is a deliberate myopia trap: greedy one-step EIG follows the default first-action tie-break and repeatedly measures an uninformative combined signal, while the exact lookahead optimizer discovers the calibration-then-measure sequence. This isolates a real algorithmic distinction from broad claims about model quality or physical validity.


### Benchmark receipts and provenance

The benchmark runner records the exact hazard, false-identification and abstention budgets used, the seed and horizon, an optional caller-supplied 40- or 64-character Git source revision, and a canonical model-family fingerprint. The fingerprint is FNV-1a 64-bit, invariant to model-world order and outcome-row order, and sensitive to kernel changes. It is a reproducibility fingerprint only, not a cryptographic integrity or provenance proof; retain the Git commit and a separately signed or cryptographically hashed evidence receipt for assurance-sensitive use.


## Generated stateful-confounding challenge families (SIM-REL-017)

The challenge module generates a six-world finite family from an explicit seed and declared rational observation-noise rate: nominal, sensor drift, physical carryover, instrument reversal, shifted hazard, and damaging calibration. Every member shares the same hypothesis/action/observation vocabulary, state dimensions, initial prior, and action costs. Each generated kernel is complete and probability-normalized exactly before it is returned. These deterministic seeds are development fixtures, not a substitute for sealed holdout seeds managed outside the public source tree.


## Joint-state bootstrap particle filter (SIM-REL-018)

The particle module is a first approximate inference arm, not a nested particle filter or SMC-squared reproduction. Each particle retains a persistent model-world index and a joint mechanism/physical/sensor state. Public-observation updates condition on the corresponding world-specific joint kernel, sample the next physical and sensor states jointly, and use systematic resampling. Diagnostics expose ESS before and after resampling, resampling count, update count, and degeneracy. Approximate mechanism and model-world posterior summaries and a particle-estimated one-step EIG ranking are available. The exact oracle remains the reference for posterior and EIG approximation error on tractable kernels.


## Explicit state vocabularies (SIM-REL-019)

Every joint kernel declares an ordered physical-state vocabulary and an ordered sensor-state vocabulary, each with one unique non-empty identifier per state index. Robust multi-world replay and optimization require exact vocabulary equality, not merely equal state counts. Model-family fingerprints include both ordered vocabularies. These labels are caller-declared semantic contracts: validation catches missing, duplicate, reordered or mismatched identifiers, but cannot prove that two scientists used the same label to mean the same physical condition.


## Finite-horizon identifiability quotient (SIM-REL-020)

The identifiability module conditions the initial joint physical/sensor state distribution separately for each mechanism hypothesis and computes the maximum total variation between public observation-transcript distributions over all deterministic adaptive policies within a declared horizon. Its dynamic program retains the two unnormalized joint state distributions, recursively chooses shared public-history continuations, and accounts for observable hazard events. Every exact pairwise zero-TV result joins the corresponding hypotheses into an observational-equivalence class. Positive scores include a witness policy, and an independent transcript replay checks each dynamic-program result. TV*=0 means no admissible policy can distinguish the pair within the supplied horizon and model; TV*=1 means their transcript distributions can be perfectly separated in that finite model; intermediate values quantify partial distinguishability. This is conditional on the declared kernel, initial distributions, action set and horizon, not a global scientific equivalence claim.
