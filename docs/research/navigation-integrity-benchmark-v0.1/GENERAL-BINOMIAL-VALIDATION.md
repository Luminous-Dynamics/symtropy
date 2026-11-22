# General binomial mission-evidence validation receipt

**Related PR:** [#6760](https://github.com/Luminous-Dynamics/symtropy/pull/6760)
**Exact head:** `b0ab9203a8a3c86ae2da9ca5f77f01e815056ef9`
**Workflow:** [run #165](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45007320523)
**Status:** passed — isolated software validation only

## Exact-head result

- Formatting/whitespace and Clippy: passed.
- Tests: 92 passed, 0 failed, across `nav-transition-validator` (31), `nav-observation-toy-fim` (49) and `miluv-ingest` (12).
- Student-t, adaptive PID, PL trace, zero-failure and mission-risk demos: passed.
- Generalization schema/split and dataset-admission CLIs: passed.
- Dataset registry: 6 candidates, 0 admitted.

## General one-sided binomial interval

The research crate now implements the exact one-sided Clopper–Pearson upper confidence limit for a binomial outcome, using the beta quantile `BetaQuantile(confidence, failures + 1, trials - failures)`. The zero-failure case uses the algebraically equivalent closed form.

A deterministic unit test substitutes the returned bound into the binomial lower-tail equation. The 1-failure / 10-trial / 95%-confidence demo returns `0.394163302437`.

## Zero-failure threshold

For a target risk of `1e-3` at 95% one-sided confidence, 2,995 independent same-scope missions with zero failures give an upper bound `0.000999744421`; 2,994 give `0.001000078170` and do not meet the target.

## Claim ceiling

These exact binomial limits assume independent, exchangeable mission-level Bernoulli trials with a fixed, predeclared scope. The code does not make high-rate samples, repeated windows or clustered route segments independent. The method does not validate mission representativeness, truth lineage, data quality, protection-level calibration or physical safety. No UrbanNav payload is admitted and no physical route result is reproduced.