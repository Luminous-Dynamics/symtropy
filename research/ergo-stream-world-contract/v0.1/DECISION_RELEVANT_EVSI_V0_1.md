# Decision-relevant expected value of sample information v0.1

**Implementation:** `src/decision_value.rs`  
**Protocol:** finite, evaluator-owned prior/loss/likelihood table
**Machine-readable fixture:** `examples/decision_relevant_evsi_v0.example.json`
**Schema:** `schemas/decision_value_problem_v0.schema.json`  
**Related:** Symtropy #7193 · Symtropy #7230 · Symthaea #13418 / PR #13407 · Symthaea #5476 / #5478 / #8721

## Research claim

More information is not necessarily a better decision.

A candidate experiment can reveal which hypothesis is true while leaving the optimal action unchanged. Another experiment may reveal fewer bits about the hypothesis but change the best action enough to reduce expected loss.

This module provides a small exact finite oracle to separate:
- mutual information about the latent hypothesis;
- expected value of sample information (EVSI), measured as reduction in minimum expected decision loss;
- net EVSI after a declared experiment cost.

It is a reference evaluator and baseline, **not** a research scheduler, learned policy, causal inference system or action authority.

## Mathematical contract

Let (H) be a finite hypothesis, (A) a finite decision action, (O) an experiment outcome, (L(H,A)) a non-negative loss, and (C(e)) the experiment cost in the same declared loss units.

Prior Bayes risk:

[
R_0 = min_{ain A} mathbb{E}[L(H,a)]
]

Expected Bayes risk after observing the experiment:

[
R_e = sum_{oin O} P(o)min_{ain A}mathbb{E}[L(H,a)mid o]
]

Expected value of sample information:

[
mathrm{EVSI}(e)=R_0-R_e
]

Net decision value:

[
mathrm{NetEVSI}(e)=R_0-R_e-C(e)
]

Mutual information is a separate quantity:

[
I(H;O)=sum_{h,o}P(h,o)log_2rac{P(omid h)}{P(o)}
]

It quantifies reduction in uncertainty about (H), not whether an action improves.

The code uses integer prior weights, integer likelihood weights and integer losses. EVSI and net EVSI are represented as exact integer numerators over a common denominator. Mutual information is reported only as a floating-point diagnostic and is not used in the exact decision calculation.

## Known-answer counterexample

The reference task has three hypotheses and two actions:

| Hypothesis | Prior weight | Loss of action A0 | Loss of action A1 |
|---|---:|---:|---:|
| H0 | 4 | 0 | 10 |
| H1 | 4 | 0 | 10 |
| H2 | 2 | 10 | 0 |

Before any experiment, A0 is optimal. Its expected loss is 2 units.

Two experiments have the same declared cost (1 loss unit):

### Experiment 100 — information about a decision-irrelevant distinction

- H0 deterministically produces O0.
- H1 deterministically produces O1.
- H2 produces O0 or O1 with equal likelihood.

This reveals whether H0 or H1 is active, but those two hypotheses prefer the same action. The optimal action remains A0 for both possible outcomes.

- Mutual information: 0.8 bits.
- EVSI: 0.
- Cost: 1.
- Net EVSI: −1.

### Experiment 101 — less information, but decision-relevant

- H0 and H1 produce O0.
- H2 produces O1.

The O1 result changes the optimal action to A1.

- Mutual information: approximately 0.721928 bits.
- EVSI: 2.
- Cost: 1.
- Net EVSI: +1.

Therefore, a greedy information-gain policy prefers experiment 100, while a decision-value policy prefers experiment 101. This is the central known-answer test: **more information can be less useful**.

The exact numerators in the code use a common denominator of 20:
- Experiment 100: EVSI numerator 0; net numerator −20.
- Experiment 101: EVSI numerator 40; net numerator +20.

## Implementation boundary

The evaluator accepts:
- positive prior weights for a finite set of hypothesis IDs;
- a finite action set;
- a complete non-negative loss table over every hypothesis/action pair;
- a finite outcome set;
- a complete integer likelihood table, normalized to one declared scale for each hypothesis;
- an experiment cost in the same units as the loss table.

It validates duplicate/unknown IDs, complete loss and likelihood tables, row normalization, zero-probability outcomes and bounded exact arithmetic. Tie-breaking is deterministic by input action order.

An outcome that is impossible under every live hypothesis is represented with no selected action. It is not converted into zero information or a false posterior.

## Required ablations and extensions

The first implementation is an exact one-step known-answer oracle. Its tests cover decision-irrelevant versus decision-relevant information, exact EVSI, cost dominance, invalid normalization, incomplete loss tables, impossible outcomes, duplicate IDs/pairs and unknown or out-of-range references. The machine-readable fixture has the same priors, losses and likelihoods as the Rust test. The next research layer should compare:
1. entropy / mutual-information greedy;
2. hypothesis-disagreement variance from Symthaea `experiment_selection.rs`;
3. hierarchical model/parameter/state information gain where priors and likelihoods are admitted;
4. exact net EVSI with experiment cost;
5. static threshold and matched-cost random baselines;
6. calibrated prospective value-of-computation predictions from #5476;
7. an oracle action-value policy on synthetic worlds only.

Later versions should add risk constraints, deadlines, resource vectors and distribution shift. Do not bolt those onto the exact finite oracle until each quantity has a defined unit and evaluator-owned truth.

## Integration boundary

- Symthaea #11060 remains the blind episode runner and seven acquisition baselines.
- Symthaea #11171 remains the production research-policy adapter.
- Symthaea #5476/#5478/#8721 own prospective cognitive value forecasts, causal metareasoning ablations and resource allocation.
- Symthaea #13418 / PR #13407 supplies a separate EGB-specific two-stage decision-value known-answer contract.
- Symtropy #7173 supplies structural/dynamics transfer witnesses.
- Symtropy #5990 owns hidden episode truth.

This module must remain an evaluation baseline/subroutine. It does not select or execute real experiments.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
