# Glass–Silicon Statistical Qualification Risk Gate v0.1

**Status:** synthetic executable audit; exact-head runtime qualification pending  
**Benchmark ID:** `glass-qualification-risk-synthetic-v0-1`  
**Claim ceiling:** exact binomial interval and decision-gate mechanics only. The fixture does not establish real-material performance, production yield, or the independence/representativeness of any plant observations.

## Why this is a distinct increment

The Pareto/fidelity benchmark in `GLASS_PARETO_FIDELITY_BENCHMARK_V0_1.md` asks which candidate evaluations to buy. This companion asks a different question: when is the evidence strong enough to call a candidate qualified?

A single confirmatory pass is inadequate when process outcomes vary. A measured 95/100 pass rate does not demonstrate, at 95% confidence, that the underlying pass probability meets a 95% requirement. This gate refuses to equate a point estimate with evidence.

## Frozen contract

The public task, evaluator-only outcomes, and report schemas are in:

- `docs/tech/glass-qualification-risk-task-v0.1.schema.json`
- `docs/tech/glass-qualification-risk-oracle-v0.1.schema.json`
- `docs/tech/glass-qualification-risk-report-v0.1.schema.json`

Fixture data:

- `tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-task.synthetic.json`
- `tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-oracle.synthetic.json`

v0.1 freezes the target success probability at 0.95 and confidence level at 0.95. Every candidate outcome is a sequence of Bernoulli pass/fail trials. Trial counts are capped at 1,000 and candidate count at 100. The tool does not accept a caller-selected confidence level after seeing the results.

## Statistical decision rule

For \(x\) passes in \(n\) trials, use the two-sided 95% exact Clopper–Pearson binomial confidence interval. Each tail has probability \(\alpha/2=0.025\). Let the interval be \([L,U]\), and let the required success probability be \(p_\text{min}=0.95\).

- **QUALIFIED:** \(L \ge p_\text{min}\).
- **NOT_QUALIFIED:** \(U < p_\text{min}\).
- **UNRESOLVED:** otherwise; more representative data or a different qualified statistical design is needed.
- **No trials:** \([0,1]\), always UNRESOLVED.

The lower endpoint is obtained by solving \(P_p[X\ge x]=0.025\) for \(X\sim\mathrm{Binomial}(n,p)\); the upper endpoint follows by symmetry using the number of failures. The Rust implementation uses a bounded bisection solver over the exact binomial tail, rather than a normal/Wilson approximation.

This is intentionally a **two-sided 95% interval**. It is more conservative for qualification than a one-sided 95% lower bound. For all-success samples, 72/72 clears the frozen 95%/95% gate (lower endpoint approximately 0.950056); 71/71 does not (approximately 0.949371). Passing 72 of 72 is not a universal production-quality recipe: it is the arithmetic of this narrowly defined independent Bernoulli fixture.

## Known-answer and mutation controls

The synthetic fixture contains five examples:

| Candidate | Evidence | Expected disposition |
|---|---:|---|
| `lot-alpha` | 72/72 passes | QUALIFIED |
| `lot-beta` | 71/71 passes | UNRESOLVED |
| `lot-gamma` | 85/100 passes | NOT_QUALIFIED |
| `lot-delta` | 76/80 passes (95% point rate) | UNRESOLVED |
| `lot-epsilon` | 0 trials | UNRESOLVED |

Tests cover exact endpoint boundary values, the distinction between an observed rate and its confidence interval, a binomial boundary control for false qualification, three different dispositions, empty evidence, forged decisions, digest mismatch, duplicate/unregistered candidate IDs, malformed trials, missing outcomes, and unknown properties.

The report commits to SHA-256 digests of the exact public task and sealed outcome bytes. `--replay` deterministically recomputes the report and rejects any changed decision, field, or digest. It is a deterministic receipt verifier, not a sandbox or a replacement for an independent statistical implementation.

## Important assumptions and failure modes

The exact interval covers Bernoulli sampling uncertainty only under the Bernoulli model; it does not correct for:

- correlated repeats from the same lot, tool, operator, or campaign;
- selection bias, cherry-picked trials, drift, changing operating conditions, or censoring;
- measurement-system error or an unvalidated definition of “pass”;
- multiple candidate selection without an appropriate simultaneous or post-selection adjustment;
- production distributions that differ from the sampled distribution.

Do not treat multiple cycles from one physical sample as independent replicates without a defensible sampling model. For clustered, sequential, censored, or time-varying process evidence, add a predeclared cluster-aware, sequential, or hierarchical analysis rather than reusing this IID gate. A real qualification program must establish the sampling unit, sampling frame, acceptance criteria, measurement-system capability, and independent confirmation before interpreting a bound as a plant guarantee.

## Run

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-risk

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-risk -- \\
  tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-oracle.synthetic.json > glass-qualification-risk-report.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-risk -- \\
  --replay glass-qualification-risk-report.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-qualification-risk-oracle.synthetic.json
```

## Promotion gates

1. Exact-head Rust tests, schema checks, generated report and deterministic replay must pass.
2. Add a preregistered simulation grid across true pass rates near 0.95, trial counts, lot effects, drift, and missing/censored outcomes. Report false-qualification and false-rejection rates separately.
3. Compare interval methods and decision policies on equal sampling budgets; include a one-sided exact bound only as a separately named comparator, never as a silent replacement.
4. For real use, define trials as independent representative production units or use a model that accounts for clustering. Validate calibration prospectively with a held-out campaign and independent metrology.
5. Only after prospective, independently reviewed validation may a claim be made about actual qualification risk. This PR validates no physical process.

## Research context

The 2025/2026 multi-fidelity benchmarking literature emphasizes standardized tasks, varied noise/landscapes, and statistics across repeated starts rather than a single deterministic outcome. See Mainini et al., [*Analytical benchmark problems and methodological framework for the assessment and comparison of multifidelity optimization methods*](https://doi.org/10.1007/s11831-025-10392-8), and the NASA 2026 [benchmarking report for BO frameworks and acquisition strategies](https://ntrs.nasa.gov/citations/20260001102). The exact-binomial interval originates with Clopper and Pearson, [*The use of confidence or fiducial limits illustrated in the case of the binomial*](https://doi.org/10.1093/biomet/26.4.404).

**Current conclusion:** this is a safer statistical gate and a synthetic known-answer test. It is not evidence that Symtropy can predict production yield or beat industrial optimization systems.
