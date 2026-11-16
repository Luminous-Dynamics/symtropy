# AeroWeft Utility Estimate Calibration Oracle

A synthetic-first calibration and held-out decision-regret evaluator for the scope-bound `UtilityEstimateReceipt` emitted by Track V.

This crate checks whether declared conservative lower utility bounds cover realized independent held-out outcomes, whether the upper bound on uninformative probability is calibrated, whether bounds are sufficiently sharp, and whether a named selector improves paired decision regret against frozen baselines under a matched budget manifest.

It rejects estimator/data/decision-boundary/ambiguity-set mismatches, calibration/adaptive/confirmatory group leakage, pseudoreplication, unknown or non-positive selection propensities, insufficient effective sample size and incomplete paired oracle campaigns. Weighted Wilson intervals use a Kish effective-sample-size approximation and are explicitly reported as such; they are not a universal guarantee under arbitrary dependence.

It does not implement Bayesian inference, EIG, active-learning acquisition, a sequential e-process, trust authentication, physical model validation, flight authorization or an aircraft controller. It composes Track V #5394, Track T #5362, Track U #5371, Mycelix trust custody, Symtropy #3588/#3171/#1760/#2778 and Symthaea Sense/SenseBench.

Positive output is limited to the named estimator, frozen policy, sampling design, held-out population, strata and benchmark assumptions. All test data is fictional.
