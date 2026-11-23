# Action-BED reproduction against the pinned TestBED implementation

**Status:** upstream implementation and recipe inspected; reproduction not yet executed  
**Tracking:** [#7072](https://github.com/Luminous-Dynamics/symtropy/issues/7072) · [benchmark programme #7062](https://github.com/Luminous-Dynamics/symtropy/issues/7062)  
**Branch:** `research/cmni-actionbed-testbed-baseline-v1`

## 1. Why this is now executable

The Action-BED paper has an official public implementation in the authors' [TestBED repository](https://github.com/testbed-benchmark/testbed). TestBED provides a modular PyTorch framework, training recipes and tests. The current upstream revision was inspected and pinned to commit:

`016fd46fd38186f2d7e7e4fed35d38165ace905a`

The pinned commit is dated 5 November 2026 and has a verified GitHub commit signature. The repository declares Apache-2.0. We do not vendor its implementation into production CMNI; it remains an isolated external research baseline.

The Action-BED paper's current arXiv version is v2, revised 6 November 2026: [Rossa, Phillips and Rainforth, Action-BED](https://arxiv.org/abs/2606.23662). The TestBED software/benchmark paper is [Kerrigan et al., arXiv:2611.06633](https://arxiv.org/abs/2611.06633), submitted 4 November 2026.

## 2. Exact upstream files inspected

All blob IDs below are from the pinned upstream commit, not mutable `main`:

| File | Git blob |
|---|---|
| `testbed/methods/actionbed.py` | `8a9cb776ff23e2320f85ddf8751e960283fd884b` |
| `testbed/recipes/experiments/temporal_discounting/actionbed.yaml` | `4d11ca4e4fdff1fa9f98ba890e86b96c511e3fa8` |
| `testbed/recipes/method/actionbed.yaml` | `592982799fabe56e84b326a8d7d6c4c93d724fd4` |
| `testbed/recipes/task/temporal_discounting.yaml` | `bf576efeb611c7057b4bdbbeed0932286091e6f6` |
| `testbed/recipes/final_eval/default.yaml` | `8ae8edefc7684280323a4eee4c4120b344308a49` |
| `testbed/recipes/net/attentive_policy.yaml` | `f402b2cb261f49e8a635babac67c0fad88ace6a3` |

The machine-readable pinned record is `research/cmni/benchmarks/actionbed-testbed-temporal-discounting-v1.json`.

## 3. Recipe semantics (do not mislabel the objective)

The official temporal-discounting recipe uses a horizon of 20, discrete Bernoulli outcomes, and the standard hyperbolic-discounting prior. The outcome temperature is zero.

**The training objective is downstream `LogMSELoss`, not EIG.** The pinned TestBED implementation defines this element-wise as `mse + log(mse + epsilon)` (with the recipe's default input/target transforms disabled). This is the paper's log-augmented MSE loss family; preserve it as a distinct variant from raw MSE, CMSE, and true log-MSE rather than treating all “MSE” recipes as interchangeable. The outcomes are discrete, so the recipe configures REINFORCE with a leave-one-out baseline. The Action-BED implementation explains that the trajectory score term is needed to propagate gradient to the design policy for non-reparameterizable outcomes. Its design policy is deterministic (`stochastic_design_policy=false`); the learned downstream action policy is an MLP.

| Parameter | Pinned recipe |
|---|---:|
| Horizon | 20 |
| Action warm-up | 50,000 steps |
| Joint action/design training | 150,000 steps |
| Outer samples per step | 2,000 |
| Optimizer | Adam |
| Learning rate | 0.0005 |
| Betas | (0.8, 0.998) |
| Gradient clip | 1.0 |
| StepLR gamma | 0.95 |
| StepLR period | 2,500 steps |
| Loss | LogMSELoss |
| Objective | REINFORCE, leave-one-out |
| Action network | [60, 512, 256, 128, 2] |

The inherited final-evaluation recipe is enabled, with 2,048 outer samples and 1,000,000,000 contrastive samples (chunk size 200,000). This estimates EIG separately from the downstream loss. Do not report EIG as the optimized training objective.

## 4. Environment and execution protocol

TestBED documents Python >=3.10 and PyTorch >=2.5. Its README notes that the Hydra 1.3 CLI does not support Python 3.14, so use Python 3.13 or earlier. Pin the full environment (including exact Torch build, CUDA, Hydra, NumPy, and transitive dependencies) before an official reproduction. A `pip install` range is not a reproducible lock.

Official command:

```bash
python -m testbed.methods.actionbed --config-name experiments/temporal_discounting/actionbed
```

Run five seeds (0–4) in separate directories, preserving resolved configuration, checkpoints, logs, exact environment, hardware/resource telemetry and final-evaluation output. The TestBED paper's broader comparison uses five seeds and a 12-hour NVIDIA GH200 budget for its policy-based benchmark. That is the paper's shared suite protocol, not a claim that the 170,000-step task recipe itself has been run here. Record training time and final EIG evaluation time separately.

## 5. TestBED's temporal-discounting result is a loss-variant control, not one Action-BED score

The TestBED paper reports the following temporal-discounting results (mean ± standard error over five seeds) for the *refitted downstream predictor*:

| Action-BED training variant | MSE ↓ | CMSE ↓ |
|---|---:|---:|
| MSE | 1.264 ± 0.019 | −0.546 ± 0.029 |
| CMSE | 0.233 ± 0.004 | −4.773 ± 0.035 |
| MSE-L | 0.157 ± 0.004 | −4.053 ± 0.093 |
| CMSE-L | 0.290 ± 0.011 | −5.011 ± 0.040 |

These rows demonstrate why the training-loss choice must be pinned and matched: they are all Action-BED but have different downstream outcomes and metric trade-offs. The current upstream recipe's `LogMSELoss` computes element-wise `mse + log(mse + epsilon)`, which is the log-augmented MSE family; verify the camera-ready naming correspondence to the paper's MSE-L row before declaring a byte-for-byte reproduced variant. The paper also reports DAD MSE 0.290 ± 0.009, JointVarPost (m=1) MSE 0.287 ± 0.009, static MSE 0.613 ± 0.10, and random MSE 0.725 ± 0.009. On the paper's reported MSE metric, the pinned Action-BED MSE recipe (1.264) is not a strong standalone score; the MSE-L/CMSE variants are materially different. Do not select one variant post hoc—reproduce the full loss matrix.

Source: Kerrigan et al., TestBED, arXiv:2611.06633, temporal-discounting results table. https://arxiv.org/abs/2611.06633

## 6. What TestBED changes about our benchmarking strategy

TestBED's 2026 paper compares DAD, JointVarPost, RL-BOED, ALINE and Action-BED with static and random baselines across eight tasks. It reports no single policy method dominates all tasks and metrics; static designs and JointVarPost can be competitive. It also demonstrates that timestep embeddings can materially change some method results, and that EIG and downstream prediction loss can rank methods differently.

This motivates additional CMNI benchmark dimensions:

1. **Adaptive-over-static gain:** measure the value of learning a sequential policy over the best static design. A task with no measurable adaptive gain is not a strong test of adaptive-design progress.
2. **Task discriminability:** report the proportion of tasks where methods can be statistically distinguished, plus rank stability and confidence intervals. Do not call statistically indistinguishable results a win.
3. **Metric rank reversals:** report EIG and downstream decision-loss rankings independently, including cases where they disagree.
4. **Architecture controls:** time embedding on/off, history encoding, parameter count and action-network capacity must be controlled or ablated.
5. **Compute-performance curves:** compare performance against training time, gradient steps, joint samples, EIG-evaluation samples and peak memory. A fixed final budget hides the shape of learning curves.
6. **Exact small-instance regret:** test learned policies against CMNI's deterministic and randomized finite oracles on discretized controls.

The TestBED paper itself notes that task difficulty and method choices matter: static and random baselines are essential, and no aggregate should hide task-level variability.

## 6. CMNI extension after baseline reproduction

Only after the pinned paper recipe is reproduced should we add a hidden-source dependence variant. The baseline Action-BED design/action policy must not receive the hidden dependence-scenario ID. The CMNI variant should optimize or evaluate worst-case expected future loss over the same finite admitted scenarios.

For the shared task, report:
- expected future LogMSE loss;
- EIG upper bound as a separate diagnostic;
- worst-case loss and action regret across hidden dependence scenarios;
- clairvoyant scenario-aware loss only as an upper bound;
- non-anticipativity gap;
- seed variance and convergence curves;
- exact oracle regret on tractable instances;
- compute, wall time and memory.

The finite CMNI oracles are not substitutes for reproducing Action-BED. They provide an independent exact reference on small tasks and a robust hidden-scenario extension.

## 7. Current status

- Pinned TestBED source and recipe: inspected.
- Exact environment lock: not created.
- Training run: not executed.
- Five-seed paper reproduction: not executed.
- Independent review: pending.
- Physical validation: not applicable to this synthetic task.

No reproduction or superiority claim is made by this specification.
