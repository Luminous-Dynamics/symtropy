# Glass–Silicon Anytime-Valid Qualification v0.1

**Status:** synthetic executable audit; exact-head runtime qualification pending  
**Benchmark ID:** `glass-anytime-qualification-synthetic-v0-1`  
**Claim ceiling:** sequential testing and alpha-budget mechanics only. No real-material, plant-yield, independence, adaptive-ledger-integrity, or industrial-performance claim is validated.

## The paradigm shift

Manufacturing optimization does not always collect a fixed sample and analyze it once. Engineers inspect intermediate results, decide whether to continue testing, add new candidates, and stop when a candidate looks convincing or unpromising. Repeatedly checking ordinary fixed-sample 95% intervals can invalidate the intended error rate: optional stopping is part of the data-collection policy, not a harmless reporting detail.

This audit provides a deliberately conservative, transparent baseline for **anytime qualification**. It allows an append-only candidate-registration stream and repeated looks at each candidate. The next candidate and the time at which it receives another sample can be chosen adaptively; alpha is never reset when a new candidate arrives or an unresolved candidate receives another trial.

## Frozen contract

- Task: `tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-task.synthetic.json`
- Evaluator-only outcomes: `tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-oracle.synthetic.json`
- Task schema: `docs/tech/glass-anytime-qualification-task-v0.1.schema.json`
- Oracle schema: `docs/tech/glass-anytime-qualification-oracle-v0.1.schema.json`
- Report schema: `docs/tech/glass-anytime-qualification-report-v0.1.schema.json`

The public event stream is ordered and append-only. Every candidate must be registered before its first trial, registration IDs may not be reused, and no trials may be requested after a candidate crosses a terminal qualification or rejection boundary. A candidate that remains unresolved can receive additional trials later, interleaved with registrations or observations for other candidates.

The sealed oracle stores the evaluator's Bernoulli trial sequence for each candidate. The report reveals only outcomes actually requested by the public trace. Input and oracle bytes are bound by SHA-256. Replay recomputes the exact report and rejects changed decisions or evidence. For production use, the append-only event stream must be committed by a separately trusted ledger or timestamp/receipt mechanism; a static JSON replay alone cannot prove the event trace was originally recorded before hidden outcomes were known.

## Countable candidate-time alpha spending

Let global family-wise error budget be \(\alpha=0.05\), candidate registration index be \(i\ge1\), and the look number for that candidate be \(t\ge1\). Allocate each candidate, each look, and each of the two directional tests:

\[
\delta_{i,t} = \frac{\alpha}{2\,i(i+1)\,t(t+1)}
\]

Because \(\sum_{i=1}^{\infty}1/[i(i+1)]=1\), and the same telescoping identity holds over \(t\),

\[
\sum_{i=1}^{\infty}\sum_{t=1}^{\infty} 2\delta_{i,t}=\alpha.
\]

This is a simple alpha-spending construction. By the union bound, the probability of at least one false qualification or false rejection across all candidates and all looks is bounded by 0.05 under each candidate's valid independent Bernoulli sampling model. It does not assume independence between candidate-level decisions. It does require the within-candidate trials to obey the registered Bernoulli model and the outcome stream not to be manipulated or selectively omitted.

## Exact one-sided binomial tests

For \(x\) passes in \(t\) trials and required success probability \(p_0=0.95\), compute exact binomial tail probabilities under the boundary \(p=p_0\):

- Qualification tail: \(P_{p_0}(X\ge x)\). If this is at most \(\delta_{i,t}\), report **QUALIFIED** and stop sampling that candidate.
- Rejection tail: \(P_{p_0}(X\le x)\). If this is at most \(\delta_{i,t}\), report **NOT_QUALIFIED** and stop sampling that candidate.
- Otherwise report **UNRESOLVED** at this look; more trials may be scheduled.

For \(p\le p_0\), the upper-tail rejection probability is bounded by its value at \(p_0\). For \(p\ge p_0\), the analogous statement holds for the lower tail. The alpha allocations cover both error directions at every candidate-time pair, including repeated looks after an adaptive decision.

The Rust implementation accumulates binomial tails with log-sum-exp, preserving extremely small tail probabilities and avoiding the underflow that can turn a tiny nonzero probability into a misleading zero in the decision calculation. Both raw and log tail probabilities are recorded.

## Synthetic trace and known answers

The fixture is an event stream rather than a single fixed-size table:

| Candidate | Registration order | Observations used | Expected disposition |
|---|---:|---:|---|
| `lot-alpha` | 1 | 310/310 passes | QUALIFIED |
| `lot-beta` | 2 | 334/334 passes, interleaved with other work | QUALIFIED |
| `lot-gamma` | 3 | 173/204 passes | NOT_QUALIFIED |
| `lot-delta` | 4 | 76/80 passes | UNRESOLVED |
| `lot-epsilon` | 5 | No trials | UNRESOLVED |

The trace first observes 320 trials for `lot-beta`, leaves it unresolved, registers and evaluates later candidates, then returns for another 14 `lot-beta` trials. It therefore exercises a resumed candidate and interleaved event sequence. The oracle contains additional outcomes for `lot-alpha` after the qualification boundary; the task trace does not request them, and the audit must stop consuming that candidate after qualification.

Tests cover exact one-trial tail probabilities, the `lot-alpha` threshold immediately before and at its qualification boundary, total countable error allocation, interleaved registration/sampling, candidate/time budget monotonicity, pre-registration rejection, post-terminal rejection, and forged-report replay.

## What this is—and is not

This alpha-spending construction is an audit-friendly conservative baseline. It is not claimed to be the most statistically powerful sequential method. The \(1/[t(t+1)]\) allocation can require substantially more observations than mixture-martingale confidence sequences, e-processes, or well-validated online multiple-testing procedures.

The correct advancement test is not merely whether this baseline reaches a decision. Compare it against:
- Fixed-horizon exact Clopper–Pearson intervals (only when sampling/stopping was truly fixed in advance);
- Bonferroni simultaneous fixed-family intervals from `audit-qualification-familywise`;
- A validated beta-binomial mixture e-process or Bernoulli confidence sequence with an explicit anytime-valid guarantee;
- Prospective sequential alpha-spending / online family-wise-error procedures;
- The incumbent engineering qualification process.

Compare family-wise false qualification, false rejection, unresolved fraction, sample count to decision, total experiment cost, and robustness under drift, dependence and missing outcomes. Use paired randomized synthetic experiments and held-out physical campaigns. A method that reduces tests but exceeds the predeclared false-qualification bound is not a valid improvement.

## Limits

This audit does not fix dependent trial outcomes, sampling bias, nonstationarity, censoring, measurement-system errors, or a mismatch between the observed population and future production. Such conditions require separately specified methods (e.g. cluster-aware bounds, a predeclared change-point policy, survival/censoring methods, or a validated sequential model). If the candidate registry is reconstructed after observing hidden results, the mathematical alpha spend cannot prove the ledger's historical integrity.

The synthetic fixture demonstrates the algorithm and its receipts, not manufacturing performance.

## Reproduce

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-anytime

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-anytime -- \\
  tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-oracle.synthetic.json > glass-anytime-qualification-report.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-anytime -- \\
  --replay glass-anytime-qualification-report.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-anytime-qualification-oracle.synthetic.json
```

## Research grounding

- Howard, Ramdas, McAuliffe and Sekhon, [Time-uniform, nonparametric, nonasymptotic confidence sequences](https://doi.org/10.1214/20-AOS1991), *Annals of Statistics* (2021), develops intervals valid uniformly over an unbounded sequence of looks and arbitrary stopping times.
- Lu et al., [Feedback-Enhanced Online Multiple Testing with Applications to Conformal Selection](https://www.jmlr.org/papers/v27/25-2123.html), *JMLR* (2026), investigates online multiple testing with feedback and finite-sample error control; it is a comparator, not implemented here.
- Jankovic, Fischer and Brannath, [Asymptotic online FWER control for dependent test statistics](https://pmc.ncbi.nlm.nih.gov/articles/PMC13478442/) (2026), discusses alpha spending as an online family-wise-error baseline and motivates more powerful alternatives.
- Lee and del Rio Chanona, [Safe Bayesian Optimization in Process System Engineering](https://doi.org/10.69997/sct.139993) (2025), compares SafeOpt, GoOSE, trust-region GP and robust BO approaches and highlights practical limitations of existing safety methods in process systems.
- Stachl et al., [Bayesian experimental design in production engineering: a comprehensive performance and robustness study](https://doi.org/10.3389/fmtec.2025.1614335) (2025), evaluates 15,360 BO configurations across eight optimization problems with noisy and noiseless variants, supporting broad configuration and robustness testing rather than a single showcase run.

These references motivate the design and baselines. This code does not reproduce their findings or claim state-of-the-art performance.

**Current conclusion:** v0.1 is a transparent, time- and candidate-indexed error-budget baseline. The next scientific opportunity is to benchmark it against more efficient anytime-valid e-processes without sacrificing the stated family-wise guarantees.
