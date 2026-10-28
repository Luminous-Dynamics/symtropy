# Glass–Silicon Selection-Aware Qualification v0.1

**Status:** synthetic executable audit; exact-head runtime qualification pending  
**Benchmark ID:** `glass-selection-aware-qualification-synthetic-v0-1`  
**Claim ceiling:** simultaneous exact-binomial interval and decision-gate mechanics only. No real-material, production-yield, independence, or industrial-performance claim is validated.

## The failure mode this closes

The companion audit `audit-qualification-risk` uses per-candidate 95% confidence intervals. Those are appropriate for a single predeclared candidate under the stated Bernoulli assumptions, but not a blanket 95% guarantee for a search process that evaluates many candidates and then reports the winner.

If a campaign evaluates a family of candidates and qualifies any candidate whose interval clears the threshold, the opportunity for at least one false qualification rises with the number of candidates unless multiplicity is controlled. The family here is therefore frozen in the public task before the evaluator-only outcomes are opened.

This matters especially in optimization: the output is deliberately selected because it looks best. Selection is part of the method and must be reflected in the evidence rule, not treated as if the winning candidate had been preselected.

## Frozen contract

The audit has its own task, sealed oracle, and report schemas:

- `docs/tech/glass-selection-aware-qualification-task-v0.1.schema.json`
- `docs/tech/glass-selection-aware-qualification-oracle-v0.1.schema.json`
- `docs/tech/glass-selection-aware-qualification-report-v0.1.schema.json`

Synthetic fixtures:

- `tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-task.synthetic.json`
- `tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-oracle.synthetic.json`

The task freezes the target success probability at 0.95 and family-wise confidence at 0.95. The candidate family contains between 1 and 100 candidates, with at most 1,000 Bernoulli trials per candidate. The task and oracle are separate byte artifacts, bound in the report by SHA-256.

## Statistical rule: Bonferroni simultaneous exact intervals

For each candidate with \(x_i\) successes in \(n_i\) Bernoulli trials, construct a two-sided exact Clopper–Pearson interval. Let \(m\) be the *entire preregistered candidate family*, and let the family-wise error budget be \(\alpha=0.05\). Allocate each candidate and each tail:

\[
\alpha_{\text{tail}} = \frac{\alpha}{2m}
\]

For \(m=5\), each tail has alpha 0.005. The union bound guarantees at least 95% simultaneous coverage of all candidate intervals without requiring independence between candidates, provided each individual binomial model is valid. The candidate trials themselves still need to be independent and representative within each candidate's sampling model.

Decision rule against \(p_{\min}=0.95\):

- **QUALIFIED:** the simultaneous interval's lower endpoint is at least \(p_{\min}\).
- **NOT_QUALIFIED:** the simultaneous interval's upper endpoint is below \(p_{\min}\).
- **UNRESOLVED:** otherwise, including candidates with zero trials.

For five candidates, the all-success boundary becomes stricter: 104/104 passes clear the simultaneous lower-bound requirement; 103/103 do not. This is not an industrial sample-size prescription. It is a known-answer for this fixed synthetic family and confidence contract.

Bonferroni is intentionally the first implementation because the contract is easy to audit and valid under arbitrary dependence between candidate-level tests. A later separately versioned comparison may implement Holm step-down testing for greater power when only decisions, rather than simultaneous interval estimates, are needed. That should not silently replace the frozen v0.1 result.

## Synthetic known answers

| Candidate | Evidence | Expected disposition |
|---|---:|---|
| `lot-alpha` | 104/104 passes | QUALIFIED |
| `lot-beta` | 103/103 passes | UNRESOLVED |
| `lot-gamma` | 85/100 passes | NOT_QUALIFIED |
| `lot-delta` | 76/80 passes (95% point rate) | UNRESOLVED |
| `lot-epsilon` | 0 trials | UNRESOLVED |

Tests cover the adjusted all-success boundary, comparison against the unadjusted 72/72 case, all three dispositions, zero evidence, a binomial family-wise false-qualification bound, strict input schema, invalid IDs, missing outcome records, and rejection of a forged report during deterministic replay.

## Limits and requirements for real process evidence

This gate covers multiplicity across the candidate family declared in one task. It does not automatically cover:

- Adding candidates after looking at previous results, repeated peeking, or repeated campaigns without an alpha-spending or held-out confirmation plan.
- Multiple outputs or constraints beyond the single Bernoulli qualification event.
- Correlated trials within a candidate, lot-to-lot variation, tool/operator clusters, process drift, censoring, or measurement-system error.
- Selection-biased data or a production population different from the sampled population.

Freeze the family and analysis before inspecting outcomes. If candidates must be added adaptively, allocate error prospectively (for example, with a validated sequential alpha-spending design) or use a genuinely independent confirmation set. If the underlying sampling units are correlated, the Bernoulli IID interval is not enough; use a model that accounts for those clusters and predeclare it.

The report's deterministic replay verifies reproducibility against the same exact bytes. It is not an independent software implementation, not a malicious-code sandbox, and not proof that the physical measurements are valid.

## Reproduce

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-familywise

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-familywise -- \\
  tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-oracle.synthetic.json > glass-selection-aware-report.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-familywise -- \\
  --replay glass-selection-aware-report.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-selection-aware-qualification-oracle.synthetic.json
```

## Research grounding

- The US FDA's [Multiple Endpoints in Clinical Trials guidance](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/multiple-endpoints-clinical-trials) describes why looking across multiple endpoints creates an increased chance of false conclusions and why multiplicity control should be prespecified. The setting differs from manufacturing quality, but the statistical principle transfers to a family of candidate qualification claims.
- NIST Technical Note 2119, [Estimating Instrument Performance: with Confidence Intervals and Confidence Bounds](https://doi.org/10.6028/NIST.TN.2119), covers binomial success probabilities and confidence bounds for performance decisions.
- Mainini et al., [Analytical benchmark problems and methodological framework for the assessment and comparison of multifidelity optimization methods](https://doi.org/10.1007/s11831-025-10392-8), argue for standardized, varied optimization benchmarks and statistics across initializations.
- Sabanza-Gil et al., [Best practices for multi-fidelity Bayesian optimization in materials and molecular research](https://www.nature.com/articles/s43588-025-00822-9), show that multi-fidelity performance depends on the informativeness and cost of lower-fidelity sources.
- NASA's [2026 framework and acquisition-strategy benchmark](https://ntrs.nasa.gov/citations/20260001102) offers a current baseline reference across several BO frameworks, task types and randomized initializations.

These sources motivate the protocol; no source result has been reproduced in this code and no glass/silicon SOTA claim is made.

**Current conclusion:** this adds a defensible selection-aware evidence gate to the benchmark stack. It makes false qualification harder to hide; it does not improve the physical optimizer by itself.
