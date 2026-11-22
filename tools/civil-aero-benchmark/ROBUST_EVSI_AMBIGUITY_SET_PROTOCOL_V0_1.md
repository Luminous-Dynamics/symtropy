# Robust EVSI / finite ambiguity-set protocol v0.1

## Purpose

The one-model EVSI engine evaluates one caller-specified prior and likelihood matrix. This extension evaluates a finite set of compatible decision models and selects an experiment by the worst-case net EVSI over that set (maximin). It prevents an experiment that looks excellent under one model but has negative net value under another declared model from being selected solely on the nominal model.

This is not distributionally robust optimization over an unbounded ambiguity set. It is robust only to the finite ensemble supplied by the caller. The ensemble is not automatically representative, calibrated or exhaustive.

## Compatibility contract

Every member must share the same ordered state IDs, decision IDs and utilities, candidate experiment IDs, outcome IDs and experiment costs. Members may differ in their prior probability vectors and experiment likelihood matrices. Each member is independently validated by the ordinary EVSI engine. Duplicate member IDs, missing experiments, incompatible decision tasks, or differing experiment cost/outcome schemas fail closed.

## Decision rule

For each experiment, evaluate net EVSI independently for every model member. Compute the minimum, arithmetic mean and maximum across the finite set. The mean and maximum are diagnostics only. Select the affordable experiment with the greatest strictly positive worst-case net EVSI; ties use lexicographic experiment ID. If every affordable experiment has non-positive worst-case net EVSI, abstain.

Budget, experiment cost and task utilities are common across members. No prior averaging or posterior model weighting is performed. The result is labelled finite_ambiguity_set_maximin_not_probability_calibrated.

## Known-answer adversarial example

Two equiprobable states have two decisions with utilities (10,0) and (0,10). Two model members share this task and the same experiment set:
- Member A says fragile_perfect_or_blind is perfect information (gross EVSI 5).
- Member B says the same experiment is uninformative (gross EVSI 0).
- robust_noisy is a 75%-accurate binary signal under both members (gross EVSI 2.5).
Costs are 1.0 and 0.4 respectively. Net worst-case values are -1.0 for the fragile experiment and +2.1 for robust_noisy. A nominal selector on member A chooses the fragile experiment; member B chooses robust_noisy. The maximin selector chooses robust_noisy.

Rust tests cover this disagreement, schema mismatch, duplicate member IDs, abstention and deterministic output. An independent standard-library Python oracle recomputes the closed-form known answer.

## Application to aerodynamic experimentation

The ambiguity members should represent explicit competing assumptions, such as turbulence/transition model variants, uncertain boundary conditions, experimental measurement intervals converted into a conservative set, mesh/model discrepancy envelopes, or prior alternatives for a suspected flow-regime transition. Keep parameter uncertainty, model-form uncertainty and measurement uncertainty distinct. Do not turn scenario counts into probabilities. The next physical/CFD experiment should be selected only after its total cost includes all solver runs, mesh generation, failed attempts, instrumentation and independent review.

A robust EVSI pass means only that one experiment has positive worst-case decision value across the supplied finite set. It does not prove that the true physical model lies in the ensemble.
