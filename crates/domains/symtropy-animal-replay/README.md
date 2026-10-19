# symtropy-animal-replay

Evidence-bounded result receipts for animal reference-task evaluations.

## Responsibilities

- Reuse the capability-profile and rights/split manifest validators; this crate is not a second authority for species profiles or data rights.
- Bind one task and claim ceiling to the target, generalization split, code HEAD, config/evaluator/toolchain/platform, seeds, metric definitions, uncertainty, sample counts, baseline references and threshold decision.
- Reject fabricated/non-finite scores, unit mismatches, duplicate metrics/baselines, unbound claims, known bad evidence, and synthetic evidence presented as empirical.
- Represent missing, abstained and failed metrics explicitly; never turn them into zero.
- Produce a canonical SHA-256 digest over a sorted receipt payload. Sorting removes input-vector order as a digest source; it does not hide differences in the actual payload.
- Mark synthetic test receipts as synthetic-oracle-only. An admissible synthetic receipt does not qualify biological accuracy.

## Important limits

The receipt builder does not fetch reference data, grant permissions, execute the simulator, select baselines, estimate scientific uncertainty, or prove causality. It validates the supplied result bundle and binds its declared evidence. It cannot establish that source metadata or a metric implementation is truthful unless those are independently qualified.

The main PR currently includes the synthetic capability-profile and observation-manifest oracles; the replay-result oracle extends those with hostile scorecard/receipt cases. All fixture data and IDs are synthetic placeholders, not animal observations.

## Focused checks

```sh
cargo fmt --check --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml
cargo test --locked --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml
cargo clippy --locked --manifest-path crates/domains/symtropy-animal-replay/Cargo.toml --all-targets -- -D warnings
```
