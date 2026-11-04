# ANIMA Sequential Intervention Design v0

Standalone, dependency-free Rust policy primitives for the sequential experiment-design layer tracked by Symtropy [#3588](https://github.com/Luminous-Dynamics/symtropy/issues/3588).

This crate does **not** implement a biological circuit, execute experiments, control hardware, or claim a biological/SOTA result. It consumes model-conditional likelihood tables supplied by separately versioned model and inference components. The finite categorical outcome model is an explicit V0 fixture/interface, not a claim that locomotor trajectories are categorical or conditionally independent.

## Paradigm shift

The one-shot selector asks which intervention is informative under today's posterior. This crate asks which *permitted* intervention should come next after sequential evidence, subject to a frozen policy, finite budget, causal-validity preconditions, and a stop/abstain/model-revision contract.

The policy does not control wet-lab hardware. Candidate costs must declare one shared comparable unit per decision manifest; mixed units are rejected. The interface is an offline decision oracle until protocol review, delivery validation, ethics approval where applicable, and a separately qualified execution system exist.

## Implemented surface

- Exact finite-outcome EIG: H(Model) - E[H(Model | outcome)]; unlike a Monte Carlo estimator, this categorical calculation has no integration error.
- Sequential Bayes update after an observed outcome, with assignment identity, delivered-protocol identity, replication-unit/session identity and washout checks.
- Greedy EIG, EIG-per-cost, robust maximin EIG-per-cost over an explicit posterior ambiguity set, deterministic random-feasible, and a fixed canonical-ID comparator.
- Hard exclusion for safety-rejected, infeasible, invalid-design, low-fidelity, unverified-delivery, interference-uncontrolled, carryover-uncleared and over-budget candidates.
- Four explicit outcomes: select, stop at a preregistered posterior threshold or experiment limit, abstain when no valid/informative option exists, or stop for model revision after a posterior-predictive failure.
- Candidate and state reproducibility fingerprints; candidate ordering and model maps are canonicalized, and rejected candidates are retained in receipts.
- Forty deterministic parameterized synthetic known-answer fixtures (eight regimes × five parameterizations), plus focused negative tests.

## Anytime-valid model-discrimination evidence

The `anytime` module adds a separate likelihood-ratio e-process for testing one pre-registered null model against a fixed prior mixture of alternatives. At each look it consumes conditional predictive probabilities for the observed outcome given the complete pre-outcome history and the intervention chosen using only past information. The mixture likelihood ratio is evaluated in log space and compared with the Ville boundary `1 / alpha`, so repeated peeking is covered under the stated conditional-model assumptions.

The caller must supply:
- a frozen prediction-manifest digest covering model versions, parameters, calibration, preprocessing and probability-generation code;
- a conditioning-state digest for the complete past used to generate that trial's conditional distribution;
- the exact assigned candidate and delivered protocol IDs;
- a valid candidate that passed assignment, safety, delivery-fidelity, interference and carryover gates;
- the complete outcome space, with each model's conditional probabilities summing to one.

An exact-zero likelihood under the null is rejected as a model-adequacy failure; it is not converted into infinite evidence. Every event records a unique sample ID, replication-unit ID and session ID in the receipt; repeated observations from the same unit/session are allowed only when the conditional forecast is based on the complete prior history, which is bound by the conditioning-state digest. Duplicate sample identities are rejected. Alternative weights are frozen at construction. Testing several null models requires a predeclared familywise alpha allocation across the separate e-processes.

This is **not** a generic guarantee for arbitrary adaptive experiments: likelihood-ratio validity fails if forecasts are not correct conditional distributions, if outcomes are selected after inspection, if the action uses future data, or if serial dependence / individual clustering is ignored. The digest fields bind the declared conditioning inputs but cannot prove their scientific correctness. The e-process is a software primitive; biological inference requires a calibrated conditional likelihood model and an externally validated protocol.

The motivating statistical frameworks are Howard et al. (2021), *Time-uniform, nonparametric, nonasymptotic confidence sequences*, Annals of Statistics, DOI [10.1214/20-AOS1991](https://doi.org/10.1214/20-AOS1991); Ramdas et al.'s 2025 ICML tutorial on game-theoretic statistics and sequential anytime-valid inference; and the adaptive-trial confidence-interval guidance of Robertson et al. (2025), DOI [10.1002/sim.70202](https://doi.org/10.1002/sim.70202). These are statistical method baselines, not locomotion-specific biological evidence.

## Important integrity limits

The V0 receipt fingerprint is FNV-1a 64-bit and is explicitly **non-cryptographic**. It detects accidental input drift but is not a cryptographic content address, signature, or tamper-evidence mechanism. Before any biological data, external protocol exchange or high-consequence use, replace it with an audited SHA-256/BLAKE3 implementation and a versioned canonical serialization. No privacy, security or provenance-integrity claim follows from the current fingerprint.

The caller must keep randomized assignment distinct from delivered stimulation. A mismatch causes the outcome to be rejected from the causal posterior update; it must still be retained as a delivery-failure record. Every experiment record carries an explicit evidence tier (synthetic fixture, in-silico generated, preprint measured, or peer-reviewed measured), and that tier is included in the decision-state fingerprint. The V0 single-event Bayesian updater rejects a replication unit or session already present in the prior history. This is deliberately conservative: repeated measurements from the same animal/preparation or session require a joint cluster/trajectory likelihood, which is not yet implemented. Do not feed repeated time samples through this updater as independent trials.

The categorical likelihood interface also does not yet represent serially correlated trajectories, continuous multivariate outcomes, parameter-posterior mixtures, delayed effects, within-session interference or time-varying carryover. Those remain owned by the separate full-covariance EIG, history-adjusted state-space and trajectory scorecard work. Integration must not duplicate those engines.

## Exact finite-horizon scenario-tree planner

The scenario_tree module adds an exact planner for the finite categorical V0 model. It expands every positive-probability outcome, updates the model posterior at each branch, applies candidate transition restrictions and the remaining cost budget, and maximizes expected discounted cumulative information gain. It returns the first action, immediate EIG, expected lookahead objective, horizon, expanded node count and residual budget.

The planner is an exact reference for the supplied finite model, not a global solution to the biological experiment-design problem. Each candidate reuses a static P(outcome | model, action) table at every future node, so the V0 tree assumes conditionally independent replicate trials. It cannot yet express serially dependent time series, within-animal repeated measures, history-dependent delivery, carryover dynamics or parameter-posterior predictive mixtures. Those require conditional likelihoods that change with the full observed history, and should be integrated with #3547 rather than approximated by reusing a static table. Complexity grows with candidates, outcomes and horizon. A hard node-expansion limit returns an error; it never silently prunes branches and calls the result exact. It currently has no learned policy warm-start, posterior network, continuous design optimizer, Monte Carlo fantasy branch reduction or anytime branch-and-bound optimality gap.

**External algorithmic baseline:** Guo et al. (ICML 2026), *Constrained Bayesian Experimental Design via Online Planning* (COPEx), combines offline amortized design-policy and posterior-network training with online multi-step scenario-tree planning, dynamic transition and budget constraints. Its public implementation uses Python/PyTorch and SLSQP-style optimization, with pre-trained ALINE models; its README describes horizon, fantasy branching, discounting, costs and transition restrictions. The published task suite includes active learning, location finding and CES, not Drosophila connectome experiments. COPEx is an important method baseline, not a drop-in fly-neuroscience comparator.

Our differentiation target is an exact, dependency-free Rust reference for small categorical mechanism-discovery problems, with causal validity, assignment/delivery identity, evidence tier, safety, intervention provenance and anytime-valid inference wired into the same evaluation loop. A future exact-to-approximate ladder would use exact enumeration for small trees, admissible bounds and explicit optimality gaps for medium trees, and learned/warm-started scenario-tree planning for larger trees. The latter two stages are not implemented yet.

## Benchmark protocol

Compare the policies on identical frozen model sets, priors, candidate manifests, outcome schemas, seeds and compute budgets:

1. static one-shot EIG ranking from the frozen initial posterior;
2. greedy sequential EIG;
3. cost-normalized greedy EIG;
4. robust maximin EIG-per-cost over prior/model ambiguity;
5. random feasible;
6. fixed canonical-ID round-robin (a smoke-test schedule, not a randomized factorial design);
7. exact finite-horizon scenario-tree lookahead on tractable categorical instances;
8. synthetic clairvoyant myopic upper bound (never a deployable policy or a finite-horizon optimality claim).

The paired-seed runner currently implements these policy surfaces; balanced factorial assignment and COPEx's learned warm-start/posterior network are external baselines/follow-on work, not already reproduced.

Report cumulative information gain, held-out intervention predictive log score, trials and cost to decision, regret versus the synthetic oracle, calibration, false confident selection, false stopping/discovery, abstention quality, rejected candidates, runtime and model-evaluation count. Report paired-seed confidence intervals and stratify by regime; do not collapse failures into mean EIG.

## Initial acceptance gates

- Zero unsafe/infeasible selections.
- Zero posterior updates after assignment/delivery mismatch or uncleared washout.
- Zero confident model choice when every candidate has identical predictive distributions.
- Correct stop on posterior-predictive model inadequacy.
- Candidate/outcome ordering does not alter deterministic adaptive decisions.
- No false precise posterior when the observed outcome has zero likelihood under all surviving models; request model revision instead.
- Random and fixed baselines remain selectable even when their chosen candidate has low EIG; the information threshold is an adaptive-policy gate, not a way to bias the baselines.

These are software acceptance properties, not biological performance targets.

## Commands

```bash
cargo fmt --manifest-path research/anima-sequential-design-v0/Cargo.toml -- --check
cargo test --manifest-path research/anima-sequential-design-v0/Cargo.toml --locked
cargo clippy --manifest-path research/anima-sequential-design-v0/Cargo.toml --locked --all-targets -- -D warnings
```
