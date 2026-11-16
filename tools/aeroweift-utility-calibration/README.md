# AeroWeft Utility-Estimate Calibration Oracle

Synthetic-first Rust evaluator for whether Track V's conservative utility bounds remain calibrated on independent, sealed challenge groups and required operating strata.

It checks exact candidate/receipt/policy/partition scope, one outcome per independent group, per-stratum lower-bound coverage for each utility dimension, and a one-sided Wilson upper confidence bound for the uninformative-event rate. It does not fit an estimator, compute Bayesian EIG, authorize tests, authenticate source data or establish physical model truth.

The evaluator uses only challenge-sealed outcomes. Calibration/development/confirmatory observations cannot be silently relabelled as challenge evidence. The Wilson interval is an explicitly named approximate score interval over supplied independent Bernoulli groups; independence and group construction remain domain-owner assumptions and are not verified by the crate.

Positive output is `CALIBRATED_FOR_DECLARED_CHALLENGE_SET_ONLY`, not general aviation reliability or safety assurance. It composes Track V #5394, Track T #5362, Track U #5371, Symthaea Sense #7735/#7756, Symtropy #3588/#3171, GEO-OBS #1760 and GLASS-OOD-VOI #2778.
