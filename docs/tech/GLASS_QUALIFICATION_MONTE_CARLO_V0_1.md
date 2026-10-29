# Glass–Silicon Paired Qualification Operating-Characteristics Benchmark v0.1

**Status:** executable synthetic benchmark; exact-head Rust execution is pending CI  
**Benchmark ID:** \`glass-qualification-paired-operating-characteristics-v0-1\`  
**Claim ceiling:** this estimates the operating characteristics of two sequential decision rules under idealized IID Bernoulli outcomes. It neither proves statistical validity (the proof is mathematical) nor validates production yield, physical material, metrology, dependence or industrial performance.

## Research question

The earlier paired fixture suggests that a uniform-mixture e-process can reach a decision sooner than a conservative time-indexed alpha-spending test on selected streams. That is only a hand-picked trace. This benchmark asks whether the sample-efficiency tradeoff persists across many random streams, including a boundary case where false terminal decisions matter most.

The comparison is deliberately paired: for each true success probability, family replication and candidate registration index, one deterministic SplitMix64 stream generates the outcomes. Both methods see that exact candidate stream and stop at their own first terminal decision. The first registered candidate has the largest alpha allocation; this is an explicit part of the present policy, not an order-invariance claim.

## Frozen v0.1 design

- True IID Bernoulli success probabilities: \(p\in\{0.90,0.95,0.98,0.99\}\).
- Candidate family size: 5, with fixed registration indices 1 through 5.
- Default Monte Carlo replications: 2,500 complete families per probability (12,500 candidate streams per scenario).
- Maximum observations per candidate: 400; unresolved candidates are counted at the cap.
- Default seed: \`20261029\`; PRNG: SplitMix64-v1 with deterministic seed derivation by scenario, replication and candidate.
- Qualification threshold: \(p_0=0.95\); familywise error budget \(\alpha=0.05\).
- Methods: (1) the reference time-indexed alpha-spending test, \(\alpha_{i,n}=\alpha/[2i(i+1)n(n+1)]\) per tail, and (2) the uniform-mixture e-process, with candidate budget \(\alpha_i=\alpha/[i(i+1)]\), divided equally between the qualification and rejection directions.
- Report: candidate-level qualification/rejection/unresolved rates; mean observations per candidate, charging unresolved attempts the full 400-trial cap; familywise false-decision rate; Wilson 95% Monte Carlo interval for that rate; and the paired reduction/increase in mean sample count.

The \(\alpha/[i(i+1)]\) allocation sums to \(\alpha\) over the countable candidate sequence. Each method's false-decision direction depends on the truth used in a scenario: at \(p< p_0\), a false qualification is counted; at \(p>p_0\), a false rejection is counted; at \(p=p_0\), either terminal decision is counted. The interval describes Monte Carlo sampling uncertainty around the finite simulation estimate. It is **not** a confidence sequence for production yield and does not establish or replace the mathematical familywise-control argument.

## Implementation and auditability

- Executable: \`tools/glass-silicon-contract-audit/src/bin/audit-qualification-monte-carlo.rs\`.
- Frozen output contract: \`docs/tech/glass-qualification-monte-carlo-report-v0.1.schema.json\`.
- Workflow: \`.github/workflows/glass-qualification-monte-carlo.yml\`.
- Precomputed threshold tables use log-binomial tails and log-space mixture e-values through the configured maximum. The simulator does not call an external statistics package or network service.
- Report configuration is SHA-256-bound. Seed, probability grid, family size, cap, method definitions and replication count are reported so results can be repeated.
- Rust unit tests cover a known SplitMix64 vector, Wilson intervals, threshold-boundary regressions (all-pass candidate 1: 310 vs 121; candidate 2: 334 vs 146), deterministic report replay, and candidate/family metric accounting.
- CI runs the fixed default configuration and stores the machine-readable JSON output as an artifact. CI checks structure and consistency, not that Monte Carlo rates fall under a hard threshold; finite random simulations should not be turned into a brittle proof test.

## Interpreting the result

A lower mean sample count is not sufficient by itself. The decision quality and unresolved rate must be evaluated alongside cost, and the familywise error estimate plus its interval must be considered at \(p=0.95\). The \(\{0.90,0.98,0.99\}\) scenarios test rejection/qualification behavior away from the boundary. At this version, all candidates have equal trial costs and a stationary, independent Bernoulli model; real lots may violate each of those assumptions.

A favorable result would justify only the next research increment: wider parameter grids, independent seed sets, family sizes \(1,2,5,10,25,50\), and a broader baseline set including fixed-horizon Clopper–Pearson qualification, the existing selection-aware simultaneous interval audit, and other confidence-sequence/e-process designs. Do not select a winner based on a single seed or tune the method on the held-out evaluation grid.

## Paradigm-shift roadmap

1. **From throughput to calibrated decisions.** Optimize expected total cost subject to predeclared false qualification and false rejection limits—not raw average sample count.
2. **From one ideal distribution to a stress matrix.** Test correlated trials, lot-to-lot random effects, serial drift, censored/missing observations, misclassified pass/fail readings, adaptive candidate scheduling, and changes of candidate specification. Every stress family needs a clear null, ground truth and a valid comparator; the IID e-process guarantee does not automatically transfer to them.
3. **From a single objective to constrained Pareto results.** Report sample cost, decision delay, unresolved fraction, type-I risk, and power together. A method that is efficient but miscalibrated is a failure, not a winner.
4. **From registration-index allocation to order-aware research.** The current \(\alpha/[i(i+1)]\) schedule is conservative and order-dependent. Compare it against procedures designed for online multiple testing without silently changing the registered candidate order. Include an order-permutation benchmark before claiming fairness across candidates.
5. **From synthetic to prospective evidence.** Only after simulation and a preregistered protocol should the benchmark touch production data. Capture independent lot IDs, calibrated instruments, process settings, failed runs, missingness, external timestamps/signatures and change-control history. An input hash binds a report to bytes; by itself it proves neither that the bytes were created before outcomes nor that a physical test happened.

## External research anchors

- Grünwald, de Heide and Koolen, [Safe Testing](https://doi.org/10.1093/jrsssb/qkae011), *JRSS B* (2024): e-values and optional continuation.
- Howard et al., [Time-uniform, nonparametric, nonasymptotic confidence sequences](https://doi.org/10.1214/20-AOS1991), *Annals of Statistics* (2021): time-uniform inference.
- Tavyrikov, Goeman and de Heide, [Carefree multiple testing with e-processes](https://doi.org/10.1214/26-EJS2546), *Electronic Journal of Statistics* (2026): highlights an important problem—many e-value multiple-testing rules depend on the order in which evidence arrives, motivating order-robust comparisons.
- Jankovic, Fischer and Brannath, [Asymptotic online FWER control for dependent test statistics](https://doi.org/10.1177/09622802261461776) (first published 2 July 2026): explicitly studies online familywise control when test statistics may be dependent.
- Leyendecker et al., [Bayesian experimental design in production engineering: a comprehensive performance and robustness study](https://doi.org/10.3389/fmtec.2025.1614335), published 8 January 2026: compares 15,360 optimization configurations across eight engineering-focused problems and noisy/noiseless variants. It is a production-engineering benchmark-design reference, not a direct comparator for sequential qualification.

These works motivate breadth of evaluation; their results are not reproduced by this benchmark.

## Reproduce

\`\`\`sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-monte-carlo

cargo run --release --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-monte-carlo -- \\
  --replications 2500 --seed 20261029 --max-trials 400 > glass-qualification-monte-carlo-report.json
\`\`\`

**Current limitation:** no local Rust compiler was available during authoring. The exact-head Rust build, test results and generated benchmark artifact must be read from CI; queued or pending is not a pass. Until then, this protocol and simulator are proposed benchmark code, not a verified result.


## Industry comparison target (not yet implemented)

The current pair of sequential Bernoulli methods is a statistical research baseline, not yet an industry-complete acceptance-sampling comparison. The next benchmark should include the current official standards as *separate, use-case-matched baselines*:

- [ISO 2859-1:2026](https://www.iso.org/standard/2859-1) defines AQL-indexed single, double and multiple sampling schemes for lot-by-lot inspection by attributes, including switching rules. It is published as the third edition in January 2026.
- [ISO 28591:2017](https://www.iso.org/standard/64623.html), confirmed current in 2024, specifies sequential sampling plans for discrete items and indexes plans by producer's-risk and consumer's-risk points.

These plans should not be declared beaten just because the e-process consumes fewer Bernoulli trials under an IID simulation. The comparator must match the standard's lot structure, sampling rules, AQL/quality levels, producer and consumer risks, acceptance/rejection outcomes, and switching policy. Compare operating-characteristic (OC) curves, average sample number (ASN), false acceptance / false rejection risk, inconclusive outcomes and total inspection cost. Record the exact standard edition and use only lawfully available standard-derived algorithms/tables; this benchmark currently neither implements nor certifies compliance with either ISO standard.
