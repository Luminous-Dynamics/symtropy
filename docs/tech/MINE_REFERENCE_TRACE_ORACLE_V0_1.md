# MINE-ACP-002 — pinned reference trace oracle v0.1

## Purpose

This utility checks whether a small Rust translation matches the published online-conformal reference implementation step by step. It is intentionally separate from MINE-ACP-001's synthetic benchmark: a benchmark can be internally deterministic and still compare the wrong algorithm.

The Rust crate implements the trace equations for ACI, clipped ACI, COP, OGD, scale-free OGD, and decaying-step OGD. A test-only Python oracle imports the authors' source from the exact pinned repository commit and emits expected traces at CI time. The Python code is verification-only; it is not a Symtropy runtime dependency.

## Pinned reference and license

- Repository: [creator-xi/Conformal-Optimistic-Prediction](https://github.com/creator-xi/Conformal-Optimistic-Prediction)
- Exact commit: [a41c0dcadc10979d8521ab9b5b8f3d460807d883](https://github.com/creator-xi/Conformal-Optimistic-Prediction/commit/a41c0dcadc10979d8521ab9b5b8f3d460807d883)
- Relevant file: `core/methods.py`
- License: MIT; attribution and complete license are recorded in [THIRD_PARTY_ADAPTIVE_CONFORMAL_ATTRIBUTION.md](THIRD_PARTY_ADAPTIVE_CONFORMAL_ATTRIBUTION.md).
- Dependency versions used by the reference workflow are pinned: NumPy 1.26.4, pandas 2.2.3, statsmodels 0.14.4, and tqdm 4.67.1. The test imports the upstream `core.methods`; it does not run the full paper reproduction suite or fetch its experimental datasets.

## Frozen test inputs

The versioned fixture `fixtures/reference-trace-inputs-v0.1.json` contains six small, deterministic cases:

- ACI and clipped ACI around the `alpha_t <= 1/(t+1)` infinite-quantile condition and the finite higher-quantile update.
- COP's score-range-proportional learning rate and empirical-CDF correction.
- OGD, scale-free OGD and decaying-step OGD on a trace containing both hits and misses.

The fixture commits the input scores and algorithm parameters, not an asserted hand-written copy of the reference output. The independent Python reference generates the expected output each CI run; both expected and Rust traces are retained with the input fixture for 30 days. This avoids treating Rust-generated output as its own oracle. A reviewed static expected-output fixture can be committed after the first successful, independently inspected run.

## Equality contract

- Case order, method identity, trace lengths, finite/infinite quantile sentinel, coverage bits and alpha-trace presence must match exactly.
- Finite numeric trace values use absolute tolerance (10^{-10}); no metric optimization or benchmark score is used in this test.
- On mismatch, the comparator reports the first case/step, output field, expected value, actual value, and error.
- The CI checkout verifies both the exact Symtropy SHA and upstream commit SHA before running.
- Test-only Python dependency installation is isolated in a temporary virtual environment.
- The artifact records reference output, Rust output and input fixture separately.

## Important boundary

This verifies trace compatibility only for the six frozen parameterized cases. It does not prove equivalence for every parameter, numerical corner case, delayed-label policy, or ahead horizon. Subsequent cases must cover quantile ties, all-equal scores, `alpha_t) boundary behavior, additional window sizes, ahead greater than one, and edge conditions. A reference-compatible interval algorithm still does not confer model credibility or operational authority; the independent admission gate remains separate.

## Research rationale

COP's ICLR 2026 paper proposes a distribution-informed update to reduce overly conservative online prediction sets while reporting coverage and regret guarantees under its stated setup ([paper](https://proceedings.iclr.cc/paper_files/paper/2026/hash/5f8241131e6fd428aa49914da76b8ad0-Abstract-Conference.html)). A 2026 full-scale wastewater pH forecasting study reports that static and covariate-weighted intervals undercovered shifted operating regimes while online adaptation recovered nearer nominal coverage at a width cost ([PMLR study](https://proceedings.mlr.press/v329/lee26a.html)). Those results motivate the comparator set; they are not field evidence for this benchmark or for a mine-site deployment.

The next phase should add larger adversarial trace suites, then evaluate the verified algorithms on temporally held-out, independently reviewed real telemetry. Report coverage, interval width, recovery time and post-recovery persistence separately; do not hide them in a composite score.
