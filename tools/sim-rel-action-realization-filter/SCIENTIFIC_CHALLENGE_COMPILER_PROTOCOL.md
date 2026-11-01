# SIM-REL-036: adaptive exact scientific decision challenge compiler

Status: deterministic recipe compiler for exact binary finite-channel tasks. It produces separate agent-facing and evaluator-facing artifacts. This initial slice does not yet implement automated recipe selection from observed agent weaknesses or embodied physical apparatus generation.

## 1. Scope

Static scientific-agent benchmarks can saturate, while generated tasks can be invalid or leak their answer if the generator, oracle and split policy are not separated.

SIM-REL-036 begins with the narrowest domain in which exact truth is available:
- finite hidden-state experiments;
- binary observation channels;
- exact rational probability grids;
- explicit prior and terminal loss table;
- classical worst-state Le Cam deficiency or prior/loss-span-weighted deficiency.

This gives us reproducible algorithmic challenge instances before extending the compiler into physical apparatus, hybrid dynamics and complete autonomous-science campaigns.

## 2. Versioned challenge recipe

A \`BinaryChannelChallengeRecipe\` records:
- caller-supplied opaque task ID;
- recipe ID/version;
- generator ID/version;
- family-group ID and evidence split;
- objective type;
- hidden-state count (2..13 for current exact solvers);
- exact probability atoms;
- raw prior-weight atoms;
- wrong-decision loss atoms;
- abstention-loss atoms.

The recipe is checked against a \`ChallengeSplitRegistry\` before generation. The registry can derive Calibration and SealedHoldout family-group IDs directly from a SIM-REL-034 ledger snapshot. Development groups are supplied by the benchmark registry because development may be adaptively expanded while the ledger is open.

All group IDs must be globally disjoint across splits. A recipe in an unregistered group is rejected.

## 3. Artifact separation

Generation returns three distinct typed artifacts:

### PublicScientificDecisionTask

Contains only the agent-facing finite problem:
- task/recipe identity;
- domain and objective;
- source and target finite observation channels;
- prior;
- complete terminal loss table.

It contains no generation seed, optimal objective, reference witness or evaluator receipt.

### SealedScientificDecisionOracle

Retains:
- exact optimum;
- optimal garbling witness;
- statewise total-variation residuals;
- statewise terminal-loss spans;
- prior/loss weights;
- exact-optimality flag.

### ChallengeGenerationReceipt

Retains the hidden generation seed, split/group identity, recipe and generator versions. It is for the benchmark evaluator and must not be sent as public task metadata.

Opaque IDs are not cryptographic hashes. The deterministic SplitMix64 generator is reproducible and explicitly non-cryptographic. Sealed seed custody and signed/canonical manifests remain external requirements.

## 4. Deterministic generation

For a fixed recipe version and seed:
1. Validate the recipe and registered split assignment.
2. Draw each hidden state's source/target first-output probability from the exact rational grid.
3. Draw raw prior weights and normalize them exactly; if a draw is all-zero, deterministically select a positive grid atom for one state.
4. Draw per-state wrong-decision losses and terminal abstention loss.
5. Construct exact stochastic channels and a complete terminal decision loss table.
6. Solve using the selected exact reference optimizer.
7. Independently replay the returned witness's rowwise TV residuals and the task-weighted objective.
8. Return public, sealed and generation-receipt artifacts separately.

Generation is deterministic, not cryptographic. The public artifact ID is supplied by the registry and is not derived from the seed.

## 5. Exact reference objectives

For \`ClassicalWorstStateDeficiency\`, use SIM-REL-032's exact binary Le Cam solver, which requires exact Fourier-Motzkin and active-set vertex agreement and a replayed residual equal to the optimum.

For \`PriorLossSpanWeightedDeficiency\`, use SIM-REL-033's exact arrangement-vertex solver and independently recompute:
\`sum_s prior(s) * loss_span(s) * TV_s\`.

The sealed verifier reruns the exact optimizer and compares the regenerated optimum and witness with the stored evaluator artifact. It also verifies stochastic-kernel feasibility and the complete statewise residual vector.

## 6. Reproducibility and split isolation

- \`verify_generated_challenge_from_recipe\` regenerates all artifacts from the exact recipe, registry and seed and requires typed artifact equality.
- Public task artifacts omit the seed and oracle fields.
- Calibration and SealedHoldout group IDs are sourced from the ledger snapshot; Development groups are separately registered.
- The challenge generator cannot silently reclassify an unregistered group.
- The generator does not consume benchmark outcomes or alter recipes in place. Adaptive recipe-selection policies will be added separately and must consume Development outcomes only.
- The generator must not read sealed outcomes to decide which new Development tasks to produce.

## 7. Conformance tests

- Same recipe/version/seed yields identical public, sealed and receipt artifacts.
- Different seeds yield different instances for the fixed test recipe.
- Oracle verification recomputes both minimax and weighted optima.
- Recipe replay checks seed/version/group assignment.
- Unregistered and cross-split group collisions reject.
- Invalid probability atoms and all-zero prior-weight grids reject.
- Public task debug representation contains no seed field.
- Mutated public priors or sealed optimum values fail evaluator replay.
- Calibration and SealedHoldout recipes are accepted only in their separately registered groups.

## 8. Benchmark targets

These are ambitions:
- 100% exact optimizer/reference agreement on supported recipe sizes.
- 100% reproducibility across supported generator versions.
- Zero invalid generated tasks escaping the independent verifier.
- Zero calibration/holdout leakage into development adaptation.
- Track unique hidden mechanism/channel configurations, recipe collision rate, solvability/rejection rate, verifier disagreement, task-generation cost and score decay over benchmark versions.
- Compare fixed human-curated task scores with generated recipe-family scores under matched model/tool/harness budgets.
- Report Discovery, held-out Learning, terminal Decision Loss, robust risk, calibration/abstention, Action Realization, apparatus Safety and Evidence Replay separately.

Benchmark results must specify the exact recipe, model, tools, budgets, evaluator and harness. This compiler is for exact finite algorithmic challenges; it is not evidence of physical lab capability.

## 9. Literature positioning

Adaptive task-recipe generation for scientific-agent evaluation is already studied by AutoSciBench. SIM-REL-036 does not claim to invent adaptive benchmarks. Its first contribution is a typed, split-aware compiler for exact finite scientific decision tasks, with separated public/oracle artifacts and reproducible exact ground truth.

References:
- AutoSciBench: https://arxiv.org/abs/2610.05140
- OpenScience benchmark: https://www.openscience.sh/benchmark
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- DOE autonomous scientific discovery selections, 8 October 2026: https://www.energy.gov/science/articles/doe-announces-four-national-laboratory-led-selections-advance-robotics-and

## 10. Claim boundary

This compiler generates exact finite binary-channel tasks under registered recipes and supported dimensions. It does not prove recipe-family coverage of real science, cryptographic secrecy of seeds, independent physical validation, an embodied apparatus capability, or SOTA superiority. Adaptive recipe selection and physical challenge generation are explicit follow-on work.
