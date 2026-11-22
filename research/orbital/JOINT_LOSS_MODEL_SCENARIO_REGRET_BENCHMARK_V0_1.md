# Joint Loss-Model Scenario Regret Benchmark v0.1

**Implementation:** `tools/orbital-observation-surface-bench/src/joint_loss_scenarios.rs`  
**Composes with:** [#6835](https://github.com/Luminous-Dynamics/symtropy/pull/6835) finite compatible worlds and [#6918](https://github.com/Luminous-Dynamics/symtropy/pull/6918) exact point-loss regret  
**Research issue:** [#6922](https://github.com/Luminous-Dynamics/symtropy/issues/6922)  
**Claim ceiling:** exact finite synthetic joint-loss-scenario regret algebra only.

## 1. Research question

A robust decision evaluator must account for uncertainty in the physical/observation world and uncertainty in the consequence model. If action losses are correlated under a coherent causal or operational model, replacing the scenario set with independent per-action intervals can invent combinations that no joint scenario supports.

The evaluator accepts:
- an upstream compatible-world ID set and source receipt identity;
- a loss-scenario manifest, each row binding a scenario ID, physical world ID, loss-model ID, evidence status, receipt identity and a complete loss vector over every declared action;
- upstream action admissibility receipts and native cost dimensions.

It verifies SHA-256 digests for the canonical world-ID set and the complete joint scenario records. The scenario digest binds the loss values, statuses, evidence receipt references and action-to-loss assignments—not just scenario names.

The evaluator does not authenticate the opaque upstream receipts, construct the world set, calibrate loss, estimate scenario probabilities, or grant authority.

## 2. Two uncertainty geometries, reported separately

Let (W) be the compatible physical-world set, (S_w) the declared loss-model scenarios for world (w), (A_{adm}) the upstream-admissible actions, and (L_s(a)in[0,1]) the loss for action (a) in complete joint scenario (s=(w,m)).

### Joint-scenario regret

For the same coherent scenario (s), compare every action against the best currently admissible action in that same scenario:

[
R_{joint}(a)=max_{sin S}
left[L_s(a)-min_{bin A_{adm}}L_s(b)ight].
]

This retains correlation among the action losses. The evaluator reports the scenario attaining the worst regret and the number of joint scenarios in which each action is best.

### Rectangularized regret comparator

For a deliberately more conservative comparator, independently vary an action's loss scenario and its comparator action's loss scenario while holding the physical world fixed:

[
R_{rect}(a)=max_{win W}
left[
max_{min S_w}L_{w,m}(a)
-
min_{bin A_{adm},,m'in S_w}L_{w,m'}(b)
ight].
]

This is a rectangular relaxation: the action and its comparator need not attain their extrema in the same loss-model scenario. Consequently (R_{rect}(a)ge R_{joint}(a)) for every action under the declared finite set. The excess is reported explicitly; it is not described as additional empirical risk or additional information.

Worst-case absolute loss is separately reported as (max_{sin S} L_s(a)). Uniform optimality requires the same action to attain the minimum in every joint scenario. Neither minimax regret nor a Pareto frontier automatically selects an action.

### Resource-aware frontiers

Two non-scalar frontiers are emitted:
- joint-scenario frontier over worst-case absolute loss, joint regret, reference trials, exposure and elapsed time;
- rectangularized frontier over worst-case absolute loss, rectangular regret and the same native costs.

No weighted scalar score hides the trade-off. Hard safety, rights, admissibility and non-compensatory consequence constraints remain separate gates.

## 3. Evidence semantics: invalid loss models cannot shrink the uncertainty set

Every declared loss scenario is required to have evidence status `admissible` and a non-empty receipt identity before a decision frontier is emitted.

- Any `out_of_profile` scenario returns `OUT_OF_PROFILE`, with no action metrics.
- Any `indeterminate` or `inadmissible` scenario returns `LOSS_MODEL_INDETERMINATE`, with no action metrics.
- An incomplete loss vector, unknown world, duplicate scenario ID or digest mismatch fails validation.
- Every compatible physical world must have at least one declared loss scenario.
- Every joint scenario must provide a loss for every declared action, including actions later excluded by upstream admissibility. This makes omission visible instead of allowing the optimizer to benefit from it.
- Inadmissible and indeterminate actions are excluded before per-scenario best-action baselines, both regret calculations and either Pareto frontier.
- If no action is admissible, the result is `NO_ADMISSIBLE_ACTION`; there is no fallback.

The scenario digest binds the supplied scenario records. It does not prove the real consequence law belongs to the declared scenario family. Loss-model calibration and coverage need independent evidence, distinct from #6887's time-uniform physical-world survival guarantee.

## 4. Frozen challenge cases

The v0.1 corpus contains seven cases:

1. **Correlated common loss shift.** Two coherent scenarios move both action losses by the same amount. `alpha` has joint worst-case regret (0), while rectangularized regret is (1/2). This demonstrates exact conservatism from breaking the action-loss correlation.
2. **Action-rank reversal.** The best action changes between loss-model scenarios. The evaluator must preserve the reversal and report the worst joint regret.
3. **Unqualified loss model.** A model scenario marked indeterminate yields `LOSS_MODEL_INDETERMINATE`, with no numeric action frontier.
4. **Out-of-profile loss model.** A deployment scenario outside the admitted loss profile yields `OUT_OF_PROFILE`, with no numeric action frontier.
5. **Inadmissible low-loss action.** A zero-loss action is upstream-inadmissible and cannot influence any best-action baseline or frontier.
6. **All actions excluded.** No metric or fallback action is fabricated.\n7. **Multi-world strata.** Two physical worlds each contain multiple loss-model scenarios; the evaluator must take the per-world rectangular envelope and then the outer worst-world maximum, independent of input order.

Rust tests and the independent JavaScript `BigInt` oracle also exercise canonical manifest digests, complete vector coverage, and invariance to case, action, world and scenario order.

## 5. Current research and industry comparators

The novelty claim is not that robust regret or ambiguity sets are new. Several strong 2025–2026 methods already address nearby problems:

- **Yamao et al., UAI 2026 — Robust Decision-Focused Learning via Worst-Case Regret Minimization.** It treats measurement error in objective coefficients and deployment distribution shift, including uncertainty sets around observed coefficients and Wasserstein ambiguity around conditional empirical distributions. This is a direct robust-regret comparator. Test on matched coefficient uncertainty and held-out shift, while recognizing that its training-time/optimization setting differs from this finite receipt evaluator. https://proceedings.mlr.press/v337/yamao26a.html
- **Ohnemus et al., ICML 2026 — Loss-Aware Distributionally Robust Optimization via Trainable Optimal Transport Ambiguity Sets.** The loss informs the learned shape of an optimal-transport ambiguity set; data-driven bootstrapping enforces robustness. It is a strong comparator for whether explicit scenario manifests are too conservative or too expensive relative to learned loss-aware sets. https://proceedings.mlr.press/v306/ohnemus26a.html
- **Chen et al., ICML 2026 — Bulk-Calibrated Credal Ambiguity Sets.** This method separates a high-mass bulk from the remaining tail contribution and reports robust accuracy trade-offs under contamination and distribution shift. It is a useful contamination/tail comparator; the finite joint-scenario evaluator does not replace statistical calibration of a credal set. https://proceedings.mlr.press/v306/chen26e.html
- **Dellaporta, O’Hara & Damoulas, Entropy 2026 — DRO with Robust Bayesian Ambiguity Sets.** Their MMD ambiguity sets are centred on a robust posterior predictive distribution to address model misspecification, with probabilistic tolerance guarantees. Compare coverage, conservatism and out-of-sample regret on matched tasks. https://doi.org/10.3390/e28040430
- **NIST TN 2119, ASTM E2862-23 and NASA-STD-5009C** remain relevant where the actual problem is instrument detection/POD or fracture-critical NDE. They define domain-specific sampling and scope assumptions; none certifies this decision evaluator or validates a consequence model by itself.

The testable differentiator is the composition of observation-process identifiability, an upstream source-bound compatible-world set, coherent joint loss vectors, fail-closed loss-scenario admissibility, explicit joint-versus-rectangular regret comparison, and independently replayable receipts. Do not claim blanket SOTA superiority from the synthetic corpus.

## 6. Tournament protocol

For every matched task, freeze the same:
- physical compatible-world set and source receipt;
- action set and upstream admissibility receipts;
- loss evidence, loss-model manifest and scope;
- cost/exposure/time budgets;
- train/test and calibration/deployment split.

Compare:
1. #6918 point-loss minimax regret (one declared point scenario);
2. joint-scenario minimax regret;
3. rectangularized interval/minimax regret;
4. robust Bayesian ambiguity-set baseline;
5. Wasserstein / loss-aware OT-DRO baseline;
6. bulk-calibrated credal / contamination baseline;
7. exhaustive finite joint-scenario oracle on small domains.

Report:
- worst-case absolute loss, joint regret and rectangular regret;
- rectangular excess by action and world;
- action-rank reversal frequency across model scenarios;
- uniform-optimum existence/cardinality;
- joint and rectangular Pareto frontier sizes;
- action admissibility exclusions and no-action frequency;
- unqualified/OOD loss-scenario detection and false-in-profile rate;
- empirical loss-model coverage by operating stratum;
- calibration samples, reference trials, compute, wall time and operator cost;
- exact replay determinism and receipt verification overhead.

A weighted average must not conceal a safety-critical loss dimension or a non-compensatory constraint. Bayesian/expected-loss comparators require a declared prior; distributionally robust expected-loss comparators require a stated ambiguity set and calibration protocol.

## 7. Prospective gates (targets, not achievements)

- 100% exact Rust/JavaScript agreement for the frozen finite corpus.
- 100% scenario-manifest digest sensitivity to any loss, evidence status, receipt, world or action mapping mutation.
- Zero omitted declared scenarios or action losses.
- Zero numeric frontier when any required loss model is unqualified or out of profile.
- Zero inadmissible action in any optimization baseline.
- 100% exact proof that rectangularized regret is no smaller than joint-scenario regret for every emitted action.
- 100% input-order invariance and exact rational tie preservation.
- 100% zero authority/physical-qualification fields.
- Held-out loss-model coverage reported by stratum with exact intervals and preregistered multiplicity control.
- A prospective efficiency target of at least 20% fewer reference trials than the strongest matched baseline at the same predeclared decision-error and loss-model-coverage constraints; do not count synthetic fixtures as trials.

The 20% target is a research objective, not a current result. Report full risk–coverage–cost frontiers even if it is missed.

## 8. Next paradigm shifts

1. **Loss-set calibration separate from world-set calibration.** Construct and evaluate a held-out, scope-bound consequence-model envelope. Never inherit the e-process alpha/coverage guarantee from #6887.
2. **Adaptive consequence discovery.** Design safe probes that distinguish loss models when the physical world is already identified well enough; optimize information per reference cost without relaxing action admissibility.
3. **Multi-stage adaptive robust control.** Replace one-shot loss matrices with versioned policies and scenario trees, preserving non-anticipativity and no post-hoc scenario selection.
4. **Tail and contamination semantics.** Compare explicit finite joint scenarios with bulk/tail or credal ambiguity sets; declare boundedness assumptions instead of silently clipping catastrophic loss.
5. **Evidence transportability.** Bind hardware, software, calibration, operator procedure, operating envelope, consequence version and reference method. Material drift means requalification or indeterminacy.
6. **Independent consequence validation.** Use external incident/maintenance/reference outcomes and blinded holdouts; a loss function authored by the same model that chooses the action is not independent validation.

The ambition is not a system that always produces a choice. It is a system that preserves what is unknown, quantifies the price of conservative approximations, and refuses to convert an unqualified consequence model into a confident decision.
