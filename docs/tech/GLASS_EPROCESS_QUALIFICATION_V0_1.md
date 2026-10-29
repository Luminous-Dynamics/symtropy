# Glass–Silicon Anytime Mixture E-Process Qualification v0.1

**Status:** synthetic executable benchmark; exact-head runtime qualification pending  
**Benchmark ID:** `glass-anytime-eprocess-qualification-synthetic-v0-1`  
**Claim ceiling:** synthetic Bernoulli mixture-e-process mechanics only. No real-material, plant-yield, independence, adaptive-ledger-integrity or industrial-performance claim is validated.

## Why add this beside alpha spending?

The conservative audit `audit-qualification-anytime` splits error budget across both candidate index and every observation time. Its allocation is transparent and anytime-valid, but spending less alpha at later looks can make it slow to qualify a strong candidate.

This increment implements a nonnegative mixture likelihood-ratio **e-process**. E-processes exploit the fact that their expectation is bounded by one under the null. Ville's inequality then gives an anytime-valid crossing rule without separately shrinking the threshold at each observation time. We still allocate alpha across candidates, but do not spend it a second time over looks.

The two tools remain separate so the efficiency gain can be compared without changing the reference implementation or its frozen report format.

## Frozen contract and artifacts

- Executable: `tools/glass-silicon-contract-audit/src/bin/audit-qualification-eprocess.rs`
- Public event trace: `tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-task.synthetic.json`
- Evaluator-only outcomes: `tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-oracle.synthetic.json`
- Task schema: `docs/tech/glass-eprocess-qualification-task-v0.1.schema.json`
- Oracle schema: `docs/tech/glass-eprocess-qualification-oracle-v0.1.schema.json`
- Report schema: `docs/tech/glass-eprocess-qualification-report-v0.1.schema.json`

Every candidate must be registered before its first observation. Registration includes an explicit synthetic spec and SHA-256 of the canonical serde-JSON serialization of that spec. Candidate IDs cannot be re-used. Events have consecutive sequence numbers. A candidate stops receiving observations once either terminal decision is reached. The exact public trace and evaluator-only oracle bytes are included in the report digests, and deterministic replay rejects a changed report.

This is an offline evaluator contract. The digest proves that the report is bound to those bytes, not that a physical trial occurred or that a real-world append-only registry existed at the time. A deployed system needs independent timestamping, signed experiment receipts or another trusted ledger to prevent retrospective reconstruction.

## Statistical construction

Let \(p_0=0.95\) be the minimum required success probability. For each candidate registered at index \(i\), allocate

\[
\alpha_i = \frac{\alpha}{i(i+1)},\qquad
\delta_i = \frac{\alpha_i}{2},\qquad
\alpha=0.05.
\]

Since \(\sum_{i\ge1}1/[i(i+1)]=1\), the candidate budgets sum to \(\alpha\), and both directional tests together have total budget at most 0.05. The registration index is assigned by the public append-only event stream; new candidates do not replenish or reset old budgets.

For observed outcomes \(X_1,\dots,X_n\in\{0,1\}\), let \(S_n=\sum_j X_j=x\), and let \(L_n(q)=q^x(1-q)^{n-x}\) be the Bernoulli likelihood.

### Qualification e-process

Test \(H_0:p\le p_0\) against larger success probabilities by mixing likelihood ratios over \(q\sim\mathrm{Uniform}(p_0,1)\):

\[
E_n^+ = \frac{1}{1-p_0}\int_{p_0}^{1}\frac{L_n(q)}{L_n(p_0)}\,dq.
\]

For every fixed \(q>p_0\), its likelihood-ratio process is a nonnegative supermartingale under \(p\le p_0\); a mixture of these processes is also a nonnegative supermartingale. Ville's inequality gives \(P(\sup_n E_n^+\ge1/\delta_i)\le\delta_i\).

### Rejection e-process

Test \(H_0:p\ge p_0\) against smaller success probabilities using \(q\sim\mathrm{Uniform}(0,p_0)\):

\[
E_n^- = \frac{1}{p_0}\int_{0}^{p_0}\frac{L_n(q)}{L_n(p_0)}\,dq.
\]

This is the symmetric lower-tail construction and has the same crossing guarantee under \(p\ge p_0\).

The operational decision is:

- **QUALIFIED** when \(\log E_n^+\ge-\log\delta_i\).
- **NOT_QUALIFIED** when \(\log E_n^-\ge-\log\delta_i\).
- **UNRESOLVED** otherwise.

Each tail is computed in log space. The integral has a beta-function form; with integer successes/failures, its incomplete-beta terms reduce to binomial tails. The implementation uses stable log-sum-exp binomial tails and log-factorials, avoiding raw likelihood underflow. The test suite checks known values, the martingale identity under the null boundary through sample count 48, and supermartingale inequalities over composite-null probability grids on both sides of the boundary through sample count 32.

## Known-answer paired comparison

The evaluator uses the same per-candidate synthetic outcome sequences for both the earlier time-indexed alpha-spending audit and this mixture-e-process audit. Interleaving differs because each policy stops at its own decision boundary.

| Candidate | Outcome stream | Alpha-spending baseline | Mixture e-process |
|---|---|---:|---:|
| `lot-alpha` | All passes | 310 observations | 121 observations |
| `lot-beta` | All passes | 334 observations | 146 observations |
| `lot-gamma` | 173 passes / 204 total, errors evenly interleaved | 185 observations | 106 observations |
| `lot-delta` | 76 passes / 80 | 80 observations, unresolved | 80 observations, unresolved |
| `lot-epsilon` | No observations | 0, unresolved | 0, unresolved |
| **Total** | Same candidate-level outcome sequences | **909 observations** | **453 observations** |

On this deliberately synthetic trace, mixture e-processes reach the same known decisions with about 50.2% fewer observations than this conservative alpha-spending construction. That is a fixture-level illustration of the method's intended advantage, not an independent performance estimate, not a physical result, and not a claim that this mixture dominates all sequential methods. The all-pass crossing boundaries are tested explicitly: candidate index 1 crosses at 121 versus 310; index 2 crosses at 146 versus 334. For the same interleaved 173/204 candidate sequence, e-process rejection occurs after 106 observations while alpha spending rejects after 185.

The event trace has five registered candidates and 453 requested observations (458 total events). It preserves the candidate-specific streams while allowing candidate selection and scheduling to interleave.

## Assumptions and claim ceiling

The validity argument assumes each candidate's trials, after its registered spec is frozen, follow a Bernoulli model with a stable success probability. It does not handle correlated samples within a production lot, unmodeled drift, censoring, changing acceptance criteria, measurement-system error or candidate modifications after registration. Those need separately preregistered methods and evidence.

The alpha budget protects the candidate family under the registered candidate-index schedule. It does not magically validate outcomes selectively dropped from the oracle or prove that the trial ledger existed before outcomes were known. A real integration must use a trusted event log with externally verifiable receipts.

## Reproduce

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-eprocess

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-eprocess -- \\
  tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-oracle.synthetic.json > glass-eprocess-qualification-report.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-qualification-eprocess -- \\
  --replay glass-eprocess-qualification-report.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-eprocess-qualification-oracle.synthetic.json
```

## Research and baselines

- Grünwald, de Heide and Koolen, [Safe Testing](https://doi.org/10.1093/jrsssb/qkae011), *JRSS B* (2024), develops e-values and e-variables for optional continuation and composite hypotheses. This is a general foundation; the particular Bernoulli mixtures above are specified explicitly in this protocol.
- Howard et al., [Time-uniform, nonparametric, nonasymptotic confidence sequences](https://doi.org/10.1214/20-AOS1991), *Annals of Statistics* (2021), provides complementary confidence-sequence theory.
- Jankovic, Fischer and Brannath, [Asymptotic online FWER control for dependent test statistics](https://doi.org/10.1177/09622802261461776) (2026), discusses conservative alpha spending and more adaptive online family-wise error control.
- Leyendecker et al., [Bayesian experimental design in production engineering: a comprehensive performance and robustness study](https://doi.org/10.3389/fmtec.2025.1614335), *Frontiers in Manufacturing Technology* (published 8 January 2026), evaluates 15,360 optimization configurations across eight problems and noisy/noiseless variants, underscoring configuration sensitivity and the need for broad comparisons.

These are research anchors, not reproduced literature results. The next fair test is a preregistered simulation grid over true \(p\) values, candidate family sizes, orderings, missingness and stress conditions, comparing false-qualification rate, false-rejection rate, unresolved fraction, expected sample count and total cost. For a production claim, repeat that comparison prospectively using independent production lots and qualified metrology.

**Current conclusion:** this v0.1 mixture e-process is a more sample-efficient, anytime-valid baseline under its stated Bernoulli assumptions. It has not been compiled in this environment or demonstrated on physical material.
