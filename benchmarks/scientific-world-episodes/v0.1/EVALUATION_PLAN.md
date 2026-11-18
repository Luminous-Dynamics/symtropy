# Scientific World Episodes — Evaluation Plan v0.1

**Snapshot:** 18 November 2026  
**Status:** preregistration proposal; no benchmark scores are claimed  
**Parent:** SCIENTIFIC-WORLD-EPISODE-001 / #5957  
**Implementation:** SIM-REL-015 / #5976, SIM-REL-016 / #6018, SIM-REL-017 / #6024

## 1. What we are evaluating

Do not call every capability “scientific discovery.” This programme has three separate tracks with different claim ceilings.

| Track | Agent receives | Hidden by the runner | What a pass establishes |
|---|---|---|---|
| **A. Closed-family identification** | Formal candidate family, action semantics, observation semantics, budget and evidence history. | Sampled candidate, latent physical/sensor realization and seed. | Correct intervention choice and compatible-candidate-set recovery within a declared finite family. |
| **B. Model adequacy / OOD detection** | The same declared candidate family and evidence interface. | A hidden owner generator that may lie outside the public family. | Ability to distinguish “one candidate currently fits” from “the declared family actually explains the world,” with false-positive accounting. |
| **C. Candidate-independent model discovery** | A problem description, observations/interventions, typed model DSL and resource limits—not a supplied candidate list. | Owner generator and sealed held-out experiments/counterexamples. | Discovery of a model program that generalizes to unseen interventions, ranges and horizons within the DSL. This is the future SIM-REL-018 track, not current capability. |

Track A and B are not ansatz-free discovery. Track C is not physical truth: it is still conditional on the DSL and synthetic owner generator. A separate physical qualification tier is required for claims about real systems.

## 2. Challenge families

Build a balanced suite rather than one confounding demonstration.

| Family | Required control | Scientific failure mode being tested |
|---|---|---|
| Structural identifiability | Two candidate models with identical output on all available interventions | False uniqueness under observational equivalence |
| Physical/sensor confounding | Different physical states masked by persistent sensor states | Treating instrument state as process state |
| Calibration enabling experiment | Calibration has little or no immediate mechanism information but enables a later discriminating measurement | Myopic EIG missing multi-step value |
| Persistent vs IID sensor bias | Repeated readings share a hidden sensor state | Counting correlated measurements as independent evidence |
| Measurement-map shift | Same underlying process with altered sensor mapping | Mistaking instrumentation drift for mechanism change |
| Hidden common cause | Correlated variables with and without an admissible intervention | Confounding vs causal effect |
| Stateful nonlinear dynamics | Action changes state and future response | Static regression that fails closed-loop prediction |
| Delayed observation | Event, acquisition and policy-availability times differ | Future-evidence leakage and incorrect temporal attribution |
| Hazard-constrained experiment | Information-rich action carries nonzero risk | Optimizing information without safety constraints |
| Out-of-catalogue generator | Hidden owner model is not in the public candidate subset | Model-family adequacy and OOD detection |
| Misleading near-neighbour | Related system shares one submechanism but changes another | Over-transfer and negative transfer |
| Unrelated control | No useful causal/mechanistic relation to the previous system | Spurious transfer from metadata or superficial similarity |

Every family needs at least one positive control, one negative control, and a known-answer evaluator fixture. For finite kernels, enumerate the complete state/action/observation support where tractable.

## 3. Primary metrics

No single pooled score. Report per family and per budget.

### Correctness and epistemic calibration
- Exact public candidate-version-space match.
- Candidate-set precision, recall and F1 from exact intersection / claimed / evaluator set counts.
- Generating mechanism retained by the public candidate family.
- False-unique rate and false-unique claim count.
- Epistemic-status correctness: identified, ambiguous, no-compatible-model, outside-candidate-family, abstain.
- OOD detection sensitivity, specificity, false-positive rate and missed rate.
- Safe abstention and unnecessary abstention separately.
- Calibration and Brier/log score only when the policy submits an explicit probability distribution; do not infer confidence from its prose.

### Experimental decision quality
- Cost to first independently verified identification.
- Cost to first supported model-adequacy/OOD claim.
- Applied interventions, rejected attempts, retries, hazards and incomplete attempts.
- Expected and empirical hazard probability.
- Counterfactual prediction error on sealed interventions.
- Action-ranking regret against exact finite-horizon optimal policy on small finite kernels.
- Expected value of sample information / decision-aware EVSI only when priors, costs, utility and candidate model probabilities are formally exposed.
- Stop/continue/calibrate/replicate/abstain/escalate decisions.

### Generalization and transfer
- Held-out intervention predictive log score or CRPS when probabilistic outputs are available.
- Horizon-stratified prediction error and interval coverage.
- Structural-family transfer accuracy and transfer-relation classification.
- Positive-transfer improvement against cold start at the same all-in budget.
- Negative-transfer rate and worst-family regression.
- Model revision count and number/size of counterexamples required before a valid held-out model.

### Resource and integrity
- Wall-clock, CPU/GPU time, peak memory, tokens, energy, tool calls, human review and total dollar cost.
- Exact binary/model/recipe/profile/evaluator hashes.
- Evidence-prefix integrity and no future-observation use.
- Runner-side attempt-counter reconciliation and denominator completeness.
- Hidden-generator, seed, split, metadata and evaluator-query leakage.
- Independent replay result and claim/evidence class.

## 4. Baseline arms

Use a fixed, declared set of comparators. Do not silently change the arm set when a new policy performs poorly.

1. **No intervention / abstain** — lower bound on experiment cost; epistemic calibration baseline.
2. **Fixed action order** — transparent deterministic experiment-order baseline.
3. **Seeded random** — reproducible exploration baseline; not cryptographic.
4. **Metadata-only negative control** — selects from public metadata without observations; should fail on randomized/hidden labels.
5. **One-step joint-state EIG per cost** — only after the public predictive model catalogue is available; it must use the same candidate prior and joint sensor/physical belief.
6. **Decision-aware EVSI** — only when terminal utility and risk budgets are declared.
7. **Exact finite-horizon joint-state optimizer** — diagnostic oracle on tractable kernels; not an agent result.
8. **Classical discovery methods** — SINDy, PySR, grammar/term search or other compatible baselines on the model classes where their assumptions hold.
9. **Frozen Symthaea incumbent and versioned challenger** — same model/tool budget and process boundary.
10. **Expert/human arm** — for a subset with pre-observation forecasts, action choices, decision time and domain review.
11. **Oracle-leak and evaluator-manipulation controls** — evaluator-side only; any positive result invalidates the campaign.

A baseline is only comparable if action set, observations, initial evidence/prior, risk limits, horizon, tool access and all-in resources are matched.

## 5. Experimental design

### Finite exact census
- Enumerate all declared candidate truths and all finite initial-state possibilities where feasible.
- Use exact rational likelihood propagation and exact frontiers.
- Report the whole finite population; do not attach sampling p-values to an exhaustive census.
- Preserve observational-equivalence classes. If several candidate programs make identical predictions on the tested intervention language, the evaluator must not reward a unique answer.

### Stochastic sampled campaigns
- Pair policy arms by episode seed and public task profile, but keep the hidden seed inside the runner.
- Use the same hidden generator draw and budget per arm.
- Use adaptive-budget comparisons only when action paths are allowed to differ; label them explicitly.
- Estimate variance in a pilot, then preregister sample size and minimum detectable effect before confirmatory runs. A small initial seed batch is debugging evidence only.
- Report paired effect sizes and uncertainty intervals, with family-stratified results and worst-case regressions.

### OOD campaigns
- Include both in-family and out-of-family generators under the same public profile.
- Include overlap prefixes where an OOD world initially produces observations compatible with a declared candidate.
- Include later discriminating actions that can make all declared candidate likelihoods zero.
- Score early false OOD claims, missed OOD cases, and cost to first supported mismatch independently.
- Never encode the hidden generator identity, OOD flag, seed or scoring outcome in public IDs, ordering or metadata.

### Leakage and reliability controls
- Randomize opaque episode IDs independently of mechanism identity.
- Permute public candidate order between paired episodes.
- Verify the policy cannot access provider objects, candidate-generation code, hidden kernel, runner memory or evaluator APIs.
- Force unknown action IDs, over-budget requests, terminal requests, malformed frames, oversized frames, blank stdout and child nonzero exits.
- Reconcile the runner's independent request counter with retained receipts; a provider's in-memory counter cannot prove that the upstream caller did not omit an attempt.
- Any incomplete trace or hidden-data leak is a hard failure, not a score penalty.

## 6. Targets — ambitions, not achieved results

| Stage | Target | Gate |
|---|---|---|
| Protocol integrity | Zero hidden-generator/seed/evaluator leaks; exact request/attempt denominator reconciliation | Adversarial process and access tests |
| Closed-family identification | Zero false-unique claims on exhaustive non-identifiability controls; exact candidate set on fully identifying cases | Independent rational evaluator and full finite census |
| OOD/model adequacy | Pre-registered false-positive ceiling plus high recall on supported out-of-family controls; report cost-to-detect | Paired hidden in/out-family worlds, overlap prefixes and divergence interventions |
| Experiment selection | At least 15–20% lower paired cost-to-verified-identification or action regret than strongest matched simple baseline, with no worse hazard/false-claim/abstention outcomes | Confirmatory paired campaign and uncertainty interval excluding zero |
| Model usefulness | Improved sealed interventional and long-horizon prediction, not only open-loop fit | Held-out interventions and independent evaluator |
| Transfer | Positive transfer on at least one held-out related family, no material negative transfer on unrelated controls, lower all-in cost than cold start | Hidden relation labels and leave-one-family/domain-out evaluation |
| Candidate-independent discovery | Better held-out structural recovery than supplied-structure parameter fitting and compatible classical baselines | Typed DSL, sealed counterexamples, independent program evaluator |
| Physical qualification | Independently reproduced result on a real apparatus with calibrated measurement and safety review | Physical evidence, rights/safety qualification and domain-expert signoff |

## 7. External benchmark mapping

- **Unsatz:** closest target for candidate-independent ansatz discovery, hidden states, distorted sensors, budgeted experiments and held-out interventions/longer runs. The current Track A/B candidate family is not equivalent; Track C must be implemented before an Unsatz-style reproduction.
- **FrontierPhysics:** end-to-end theoretical/computational/experimental research and expert-reviewed manuscript output. Use as a research-workflow qualification, not a finite mechanism-recovery leaderboard.
- **Scientist World:** improvement over reproduced paper methods in executable paper-derived environments. Use to test research-method improvement and trace quality.
- **FIRE-Bench:** evidence-backed rediscovery of verifiable findings from ML papers. Use to test rediscovery, not novel hidden-mechanism synthesis.
- **SCI-AGENT-EXAM / AstaBench / SciAgentArena:** broad scientific workflow, artifact and cost benchmarks. Compare overlapping task classes and resource usage, not exact candidate-set metrics.
- **Autonomous Science Bench:** real laboratory task resolution, cost and elapsed time. Use only after physical apparatus and safety qualifications.
- **RE-Bench:** research-engineering performance against human experts at matched time budgets.
- **World Model Benchmark Survey:** taxonomy check to keep prediction-level, closed-loop, physical plausibility and downstream utility claims separate.

## 8. Publication gate

A “better than SOTA” statement requires a pinned benchmark release, matched task construct, complete matched baselines, sealed holdout, paired effect estimate with uncertainty, per-family results, all-in cost, raw receipts and independent reproduction.

Otherwise publish the exact bounded finding. Example: “On this exhaustive synthetic stateful kernel census, the policy recovered the exact public candidate version space at X action-cost units, with Y false-unique claims and Z OOD false positives.” Never turn a narrow synthetic win into a general autonomous-science claim.
