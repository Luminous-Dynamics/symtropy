# AeroWeft Model Credibility Envelope

A dependency-free Rust structural evaluator for model-validation packages. It checks exact model/applicability identity, calibration-versus-confirmatory group separation, declared evidence modalities, held-out metric provenance, per-stratum coverage, sample-count floors and predeclared metric gates.

It does **not** run an aircraft simulator, compute residuals from flight data, authenticate datasets, verify sensor calibration, establish physical truth, or certify a model. Positive output is `ADMISSIBLE_FOR_DECLARED_DOMAIN_REVIEW_ONLY`.

The crate is deliberately synthetic-first. Real flight/bench/HIL evidence needs the separate Mycelix provenance/custody and owner-specific physical-validation path.
