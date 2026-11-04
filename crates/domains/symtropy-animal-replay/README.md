# symtropy-animal-replay

Evidence-bounded result receipts for animal reference-task evaluations.

## Responsibilities

- Reuse the capability-profile and rights/split manifest validators; this crate is not a second authority for species profiles or data rights.
- Bind one task and claim ceiling to the target, generalization split, code HEAD, config/evaluator/toolchain/platform, clean/dirty tree identity, seeds, metric definitions, uncertainty, sample counts, baseline references and threshold decision.
- Pin the required confidence level and versioned uncertainty method for score intervals and for paired-effect intervals; intervals whose confidence/method differs from the preregistered task are rejected.
- Bind each baseline to its own run ID, exact code HEAD, config digest, evaluator, toolchain, platform, revision, training-data scope and compute budget; a free-form model name alone is not a reproducible baseline.
- Require an exact SHA-256 digest for the paired-unit membership manifest and make the budget policy explicit: matched budget requires identical declared training-data scope and compute budget; a budget trade-off must be identified as such.
- Require explicit paired candidate-minus-baseline effects (independent unit, unit-manifest digest, confidence interval, minimum effect and non-inferiority margin). A baseline point score alone is never a claim that the candidate is superior.
- Classify baseline comparisons as outperforms-by-margin, non-inferior, regresses-beyond-margin, uncertain, not-assessed or incomplete. Higher-is-better and lower-is-better metrics are normalized explicitly.
- Bind every reported primary-metric stratum to its own baseline score and paired comparison. Taxon strata must be present in the manifest taxonomy, and the paired mean delta must agree with the reported candidate-minus-baseline means.
- Preserve a metric matrix by stable stratum (overall, taxon, life stage, site/study, occlusion or condition); the same metric may appear in multiple different strata, but duplicate metric/stratum keys are rejected.
- Mark a threshold as met only when the full reported confidence interval clears the threshold. If the interval overlaps it, return `uncertain`; if no interval is provided, the threshold is not assessed.
- Reject fabricated/non-finite scores, unit mismatches, duplicate metrics/baselines, unbound claims, known bad evidence, and synthetic evidence presented as empirical.
- Represent missing, abstained and failed metrics explicitly; never turn them into zero.
- Produce a canonical SHA-256 digest over a sorted receipt payload. Sorting removes input-vector order as a digest source; it does not hide differences in the actual payload.
- Mark synthetic test receipts as synthetic-oracle-only. An admissible synthetic receipt does not qualify biological accuracy.

## Important limits

The receipt builder does not fetch reference data, grant permissions, execute the simulator, select baselines, estimate scientific uncertainty, or prove causality. It validates the supplied result bundle and binds its declared evidence. It cannot establish that source metadata or a metric implementation is truthful unless those are independently qualified.

The main PR currently includes the synthetic capability-profile and observation-manifest oracles; the replay-result oracle extends those with **72 frozen expected-outcome cases** for hostile scorecard/receipt conditions. All fixture data and IDs are synthetic placeholders, not animal observations.

## Focused checks

```sh
cargo fmt --check --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml
cargo test --locked --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml
cargo clippy --locked --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml --all-targets -- -D warnings
```
