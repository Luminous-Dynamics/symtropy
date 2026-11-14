# CAVERN REPRESENTATIVENESS SHIFT COUNTEREXAMPLE — v0.1

**Owner:** #4041 CAVERN-EXPERIMENT-VOI-001  
**Related:** #3171 SIM-REL-029; #4057 ROBUST-EXPERIMENT-001  
**Evidence class:** `SYNTHETIC_ONLY`

## Purpose

This is a deterministic negative control for adaptive measurement selection under sampling-distribution shift. It demonstrates why maximizing expected information on the observed/training mix is not enough to guarantee lower decision loss on a shifted deployment mix.

Research anchor: Tang, Sloman & Kaski, *Representative, Informative, and De-Amplifying: Requirements for Robust Bayesian Active Learning under Model Misspecification*, AISTATS 2026, PMLR 300:3016–3024. https://proceedings.mlr.press/v300/tang26d.html

This fixture is **not** a physical CEA sensor model, not a defect prevalence estimate, and not an implementation or reproduction of R-IDeA.

## Frozen toy decision problem

Two strata:
- routine: prior defect probability 0.02;
- thermal-stress: prior defect probability 0.35.

Decision costs:
- false acceptance: 3;
- false rejection: 1.

Thus the Bayes action rejects when posterior defect probability exceeds 0.25.

Two binary observations:
- M1 is optimized for routine conditions: sensitivity/specificity 0.99/0.99 in routine, 0.55/0.95 in thermal stress.
- M2 is robust across strata: 0.85/0.85 in both.

All measurement costs are set equal to zero for this isolated decision-rule counterexample. A real campaign must include its complete cost vector.

Training mix: 95% routine, 5% thermal stress.  
Sealed deployment mix: 50% routine, 50% thermal stress.

The acquisition policy sees only the training mix. The deployment mix is disclosed only to the evaluator.

## Exact results

| Quantity | M1 | M2 |
|---|---:|---:|
| Expected mutual information, training mix (bits) | **0.1177875** | 0.0501611 |
| Expected decision loss, training mix | **0.03513** | 0.06975 |
| Expected decision loss, shifted deployment mix | 0.25770 | **0.15750** |

No-measurement decision loss is 0.08950 under the training mix and 0.35500 under the shifted mix.

M1 wins both training mutual information and training decision loss. M2 wins deployment-shift decision loss by 38.9% relative to M1:
`(0.25770 - 0.15750) / 0.25770 = 0.3892`.

Per-stratum expected decision losses:
- routine: M1 0.0104; M2 0.0600;
- thermal stress: M1 0.5050; M2 0.2550.

These are finite arithmetic results under the exact synthetic assumptions in the JSON fixture. They are not statistical estimates from real observations.

## Benchmark assertions

A future executable baseline should:
1. Recompute expected mutual information from the declared priors and likelihoods.
2. Recompute posterior decisions and expected loss independently for each observation outcome and stratum.
3. Recompute the training and shifted mixture results using the exact frozen weights.
4. Confirm that training maximum-mutual-information and training minimum-loss choose M1.
5. Confirm that shifted deployment minimum-loss chooses M2.
6. Fail if the sealed deployment weights are exposed to the acquisition policy.
7. Preserve the two strata and per-stratum outcomes; no pooled score may conceal the thermal-stress failure.

Compare maximum mutual information, fixed sampling, one-step EVSI on the training model, exact adaptive #4027 where its assumptions apply, robust finite-model #4057 where a probability distribution over models is not justified, and representative/de-amplifying acquisition when its assumptions and implementation are available.

If the target distribution is unknown, do not invent weights. Evaluate the admitted environments separately using a pre-registered robust criterion, or return NEEDS_EVIDENCE / HUMAN_REVIEW.

## Limitations

No physical likelihood, sensor calibration, crop/facility domain, measurement cost or independent model-set admission is established. No operational measurement recommendation is authorized. This fixture exists to test the benchmark's treatment of representativeness and distribution shift only.
