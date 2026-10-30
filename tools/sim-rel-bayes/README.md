# SIM-REL-003: MDL-prior Bayesian experimental design

This isolated, dependency-free Rust crate extends the bounded grammar prototype with a principled, cost-aware experiment selector. It is a research comparison against the SIM-REL-002 baselines, not a claim that this heuristic already wins.

## Research change

SIM-REL-002's active selector maximizes the size of the smaller candidate partition. That treats all semantic functions as equally probable and all experiments as equally costly. SIM-REL-003 instead uses a grammar-derived minimum-description-length (MDL) prior and selects the experiment with the greatest expected reduction in posterior entropy per unit cost.

### Declared probabilistic grammar

At each expression node:

- with probability 1/2, stop and choose one of five terminals (0, 1, x, y, z) uniformly;
- with probability 1/2, choose one of four operators (NOT, AND, OR, XOR) uniformly, then recursively encode its child expression(s).

The resulting shortest-derivation code length is:

- terminal: L = log2(10) bits;
- operator node: L = 3 + sum(L(children)) bits.

The grammar is subcritical: expected child count is 0.875 per node, so it defines a proper terminating recursive prior. The implementation assigns each **semantic truth-table class** the code length of its shortest enumerated derivation. Its unnormalized weight is 2^(-βL) and the weights are normalized over the finite, depth-capped semantic candidate set. The CLI exposes `--mdl-beta` in [0,4], with default β=1. At β=0, every semantic function has equal prior mass and MDL-EIG must reduce exactly to uniform-prior EIG; increasing β concentrates more prior weight on shorter descriptions. The workflow predeclares a β∈{0,1,2} sensitivity sweep and retains separate exact-head receipts.

This is a shortest-program/MDL prior. It is **not** the exact marginal probability obtained by summing the probabilities of every syntactic derivation of the same semantic function. That choice is deliberate: syntactic alias multiplicity must not give a semantic function an accidental extra vote. It is also not a calibrated posterior claim; sensitivity to grammar, code lengths and priors remains an open research question.

### Experimental design objective

Given the current surviving functions, the deterministic simulator induces an outcome distribution for each unqueried action. Its expected information gain is the expected posterior entropy reduction:

EIG(a) = H(H | D) - sum_y P(y | D,a) H(H | D,y,a)

For a deterministic observation model this equals the entropy of the binary outcome Y_a. The selector maximizes:

EIG(a) / cost(a)

Costs are positive integer units supplied in the agent-facing task fixture. The experiment chooser sees the posterior candidates, available actions and costs; evaluator truth is not a selector input. The runner consults truth only to simulate the response to a chosen action and score the episode.

A separate post-fit falsification step challenges a provisional singleton: the MDL selector chooses an unqueried action with the highest prior-weighted probability of contradicting that candidate per unit cost. A successful challenge is not proof of the full rule; exact identification still requires all eight input assignments. If the observation eliminates all hypotheses, the status is misspecified.

## Compared policies

All five policies share the same generated grammar, task targets, initial observations and intervention budgets:

1. maximum-disagreement / maximum-minority partition (SIM-REL-002 selector);
2. uniform-prior expected information gain per cost (isolates cost-awareness);
3. MDL-prior expected information gain per cost;
4. fixed ascending action order;
5. seeded-random action order (32 repetitions per task).

Report interventions and cumulative cost separately. The primary cost metric is mean/median/p90 cumulative cost; query count is a secondary diagnostic. Also report semantic candidate counts, in-grammar target retention, ambiguous/provisional/exact/misspecified statuses, and post-fit falsification outcomes.

## Model class and fixture

Three binary inputs; eight intervention assignments; constants 0 and 1; NOT, AND, OR, XOR; syntax-tree depth capped at two; 109 unique semantic truth tables. The 18 task functions are inherited from the public SIM-REL-002 smoke fixture and now have unequal action costs. Evaluator target outputs are stored separately from task descriptions.

**This is the same small, hand-authored public pilot, not a new sealed held-out corpus.** Its cost profiles permit a cost-aware smoke test; they do not establish efficacy or generalization. Do not use the published 'heldout' labels from this fixture as an independent confirmatory set.

### Exhaustive finite-domain corpus

The branch also adds `problems-exhaustive-boolean8-v1.tsv` and its separate evaluator truth fixture. These enumerate **all 256 possible Boolean functions** over the eight assignments of three input bits. The 256 target functions are deterministically permuted before opaque IDs and task costs are assigned; action costs are generated independently from the target function. The diagnostic splits contain 154 train (66 in-grammar / 88 out-of-grammar), 51 validation (22 / 29), and 51 heldout (21 / 30).

This is exhaustive over the finite three-input truth-table target space, not exhaustive over larger input spaces or physical systems. The split labels are diagnostic partitions, not a generalization test: all target functions are included in the same benchmark and the entire finite function space is now publicly known. It provides a complete finite-domain census of target retention, ambiguity, and misspecification under the declared candidate grammar.

The exact-head workflow runs both the 18-case pilot and the 256-function exhaustive corpus at β={0,1,2}, generating six retained JSON receipts. The exhaustive set's primary scientific value is complete finite-domain accounting, not a claimed industry benchmark win.

## Run

From the repository root, choose an exact reviewed Git SHA:

~~~bash
cargo fmt --manifest-path tools/sim-rel-bayes/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-bayes/Cargo.toml --bin sim-rel-bayes
EXPECTED_HEAD="<reviewed-40-hex-commit>"
cargo run --locked --manifest-path tools/sim-rel-bayes/Cargo.toml --bin sim-rel-bayes -- \
  --problems tools/sim-rel-bayes/fixtures/problems-v0.tsv \
  --truth tools/sim-rel-bayes/fixtures/evaluator-truth-v0.tsv \
  --expected-head "$EXPECTED_HEAD" \
  --max-depth 2 \
  --mdl-beta 1 \
  --out /tmp/sim-rel-003-receipt.json
~~~

The runner requires an exact HEAD and a clean checkout. Write outputs outside the repository. The dedicated workflow runs formatting, tests, strict Clippy, then a predeclared β={0,1,2} sweep. Each exact-head receipt is validated and uploaded for inspection.

## Limits / next gates

- Observations are deterministic. No noise likelihood, continuous-valued outcomes, parameter posterior or calibration metric is implemented yet. Do not present this as a noise-robust Bayesian design method.
- The action cost model is synthetic and task-specified, not calibrated against real experiment time, energy or money.
- This is not a direct comparison against PySR, Operon, SR4MDL, ESR, AI Feynman or physical-system discovery agents; those solve different tasks. It only compares selectors under a shared synthetic grammar.

Relevant field comparisons:
- [SR4MDL (ICLR 2025)](https://proceedings.iclr.cc/paper_files/paper/2025/hash/a402493de088886740b5939f666a6e56-Abstract-Conference.html) uses an MDLformer-guided search objective for formula recovery. Its paper reports roughly 50 recovered formulas across 133 problems and tests 122 unseen black-box problems. This is a formula-recovery result, not an adaptive intervention-cost result.
- [Exhaustive symbolic regression with MDL (Royal Society A, published 2026)](https://doi.org/10.1098/rsta.2024.0584) argues for exhaustive search and information-theoretic model selection to address search failure and ambiguous model selection. That motivates keeping our candidate generation and MDL prior explicit and inspectable.
- [GOLLuM, Nature Machine Intelligence (28 August 2026)](https://www.nature.com/articles/s42256-026-01283-z) integrates a GP objective with language-model representations for calibrated experimental optimization. It reports 23 tasks, 20 seeds, a 50-experiment budget, 36.3% top-5% coverage, and median 41% fewer iterations to reach the traditional GP baseline. This is closer to the eventual physical-experiment-design comparison, but still differs from exact Boolean rule identification.
- [2026 symbolic-regression survey](https://link.springer.com/article/10.1007/s11831-026-10681-w) reviews 300+ methods and highlights convergence around physics-informed constraints, multi-objective optimization, semantic awareness and hybrid methods, while noting scaling and overfitting limits.
- [Sequential Bayesian design and optimal stopping (Cheng & Huan, revised June 2026)](https://arxiv.org/abs/2509.21734) treats choosing the next experiment and deciding when to stop as coupled finite-horizon decisions. Our future oracle/abstention benchmark should compare terminal stopping value, not simply force full identification.
- [Bayesian Experimental Design via Score Matching (UAI 2026)](https://proceedings.mlr.press/v337/phillips26a.html) addresses the computational intractability of EIG policy optimization and is a relevant scale-up reference once the exact finite-domain policy is established.
- [GOLLuM code and data](https://github.com/schwallergroup/gollum) are public; it is a useful external implementation for a later physical-domain comparison.
- The 18 public tasks are too small for performance conclusions. Before efficacy claims, build at least 100 independently generated worlds, three difficulty bands, held-out complete rule/composition families, noise and alias-equivalence controls, sealed targets, prior-sensitivity analysis, and paired uncertainty intervals.
- Proposed target: at least 15% lower median cost than the strongest baseline at equivalent target retention, falsification, identification and abstention quality, with a paired uncertainty interval excluding zero and replication on another sealed rule family. This is a target, not a result.
- Transfer to spring–mass and typed graph-rewrite systems only after the synthetic selector passes integrity, identifiability and prior-sensitivity gates.
