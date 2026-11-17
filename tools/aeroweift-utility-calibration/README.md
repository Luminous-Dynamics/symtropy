# AeroWeft Utility-Estimate Calibration Oracle

Synthetic-first Rust evaluator for whether Track V's conservative utility bounds remain calibrated on independent, sealed challenge groups and required operating strata.

It checks exact candidate/receipt/policy/partition scope, one outcome per independent group, per-stratum lower-bound coverage for each utility dimension, and a one-sided Wilson upper confidence bound for the uninformative-event rate. It does not fit an estimator, compute Bayesian EIG, authorize tests, authenticate source data or establish physical model truth.

The evaluator uses only challenge-sealed outcomes. Calibration/development/confirmatory observations cannot be silently relabelled as challenge evidence. The Wilson interval is an explicitly named approximate score interval over supplied independent Bernoulli groups; independence and group construction remain domain-owner assumptions and are not verified by the crate.

Positive output is `CALIBRATED_FOR_DECLARED_CHALLENGE_SET_ONLY`, not general aviation reliability or safety assurance. It composes Track V #5394, Track T #5362, Track U #5371, Symthaea Sense #7735/#7756, Symtropy #3588/#3171, GEO-OBS #1760 and GLASS-OOD-VOI #2778.


## Held-out decision regret

The evaluator also scores policy-selected alternatives against the best feasible alternative under the same held-out case and the same six-dimensional resource budget. It reports mean and maximum regret, oracle and selected utility, abstention count, and infeasible/over-budget selections separately. An infeasible or over-budget selection receives zero utility and is counted as a false-feasible selection; it cannot be rescued by a favorable average.

Cases are sorted by stable identity and equal-utility oracle ties use a stable candidate-ID tie-break, so input permutation does not alter the report. This is a finite benchmark score, not a statistical proof of population-wide decision superiority. The crate does not yet implement propensity-weighted off-policy evaluation for adaptively selected challenge populations; use a prospectively independent challenge design or narrow the claim to the observed challenge population.
