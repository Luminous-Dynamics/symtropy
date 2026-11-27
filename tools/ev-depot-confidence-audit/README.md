# EV Depot Confidence Audit v0.1

Standalone Rust calculation of **exact one-sided Clopper–Pearson lower confidence limits** for binary service-target pass rates across a set of scenario resamples. It separates an empirical capacity frontier from a capacity level that statistically demonstrates a target reliability threshold at a declared confidence level.

## Why this matters

If 30 out of 30 independent scenario trials pass, the empirical pass rate is 100%, but the exact one-sided 95% lower confidence bound is only approximately 90.5%. A 95%-reliability target therefore is not statistically demonstrated at 95% confidence by 30/30 trials. With 59/59 successes the bound is just above 95%. This prevents a modest resampling campaign from being misreported as a statistical reliability guarantee.

The confidence statement is conditional on the resampling distribution and the independence/representativeness of scenario trials. It does not establish real-world reliability, dataset representativeness, simulator validity, or physical feasibility.

## Run

```sh
cargo fmt --manifest-path tools/ev-depot-confidence-audit/Cargo.toml --all -- --check
cargo test --manifest-path tools/ev-depot-confidence-audit/Cargo.toml --all-targets

cargo run --quiet --manifest-path tools/ev-depot-confidence-audit/Cargo.toml -- \
  tools/ev-depot-confidence-audit/fixtures/robust-cases-synthetic-v0_1.json \
  --confidence 0.95 --target 0.95 confidence-report.json
```

The input schema is the case bundle emitted by the EV depot robustness workflow. Each case contains `vocation`, `fleet_size`, `seed`, and a `report.capacity_frontiers[]` array with `policy` and capacity `points[]`; each point requires `fraction_of_declared_base_capacity`, `installed_site_power_budget_kw`, and `meets_service_target`.

For every vocation, fleet size, policy and sampled capacity the tool reports trials, successes/failures, observed pass rate, one-sided exact lower bound, target/confidence, and how many additional independent all-pass trials would be needed to demonstrate the target if every added scenario passes. It also computes separate minimum sampled capacities meeting the observed pass-rate target and statistically demonstrating it.

The tool rejects empty or malformed cases, nonfinite or out-of-range capacity fractions, and duplicate seeds for the same condition/capacity. Capacity levels are quantized to basis points to make floating-point condition keys stable.

## Statistical method

For (k) passes of (n) trials and confidence (1-\alpha), the lower confidence bound is the solution (p_L) to:

[
\Pr_{p_L}(X \ge k)=\alpha,\qquad X\sim\operatorname{Binomial}(n,p_L).
]

This is the one-sided exact Clopper–Pearson lower confidence limit (\operatorname{Beta}^{-1}(\alpha;k,n-k+1)) for (k>0); it is zero when (k=0). The crate computes the inverse by bisection over the binomial upper tail; the all-success case uses (p_L=\alpha^{1/n}).

## Expected use in depot planning

Pair this tool with identical workloads and a fixed target such as 95% of scenarios meeting both 95% deadline completion and 95% energy service. Report both the empirical frontier and the confidence-qualified frontier. With 30 resamples and a 95%-in-95% requirement, the confidence-qualified frontier should remain unclaimed even if all 30 tests pass. Increase resampling count or change the decision target explicitly; do not quietly lower the gate.
