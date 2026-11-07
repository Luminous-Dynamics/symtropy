# CIVDYN HumanStudy reproduction receipt v0.1

This crate is the first executable conformance layer for issue [#3786](https://github.com/Luminous-Dynamics/symtropy/issues/3786). It does **not** claim to reproduce HumanStudy-Bench yet.

## A source-version discovery that changes the benchmark plan

The HumanStudy-Bench paper describes an initial 12-study suite. The current upstream community repository at the pinned revision below contains 16 study directories. These are different protocol populations and must never be silently mixed.

| Track | Identity | Status |
|---|---|---|
| Published paper suite | 12-study paper snapshot | Paper scope is known; exact historical source/data revision still needs a separately verified pin |
| Current community edition | 16 studies, `study_001`–`study_016` | Pinned below; the first executable conformance profile |

Current community source:
- Repository: `HumanStudy-Hub/HumanStudy-Bench`
- Commit: `b34d214b980be13ec5944a4d570a38bc4047b0f9`
- Root Git tree: `26851c1d6564c78b25600afc7de8ec435e4aa3b8`
- Exact study-directory tree identities: `upstream_profile.json`
- Paper: Liu et al., *HumanStudy-Bench: Towards AI Agent Design for Participant Simulation*, [arXiv:2602.00685](https://arxiv.org/abs/2602.00685)

The profile is immutable. Updating upstream requires a new protocol ID, source commit, root tree and complete study inventory.

## What the Rust gate does

- Requires all 16 pinned study receipts; rejects missing, duplicate, extra or changed study trees.
- Requires exact upstream source identity and frozen model/configuration/analysis digests.
- Preserves official PAS and ECS as upstream-provided outputs, checking only their declared scales (PAS [0,1], ECS [-1,1]). It does not recalculate or approximate them.
- Preserves effect-level estimates and optional 95% intervals with finding/test identities and explicit effect units.
- Requires the independent unit and separate study receipt; the subsequent runner must map the original design's unit (participant, group, session or cluster).
- Requires artifact roles for human reference aggregates, simulated responses, official analysis outputs and official score outputs.
- Rejects publication of record-level human data unless redistribution is explicitly authorized; an aggregate artifact is not allowed to contain record-level human data.
- Fails exact-comparability validation when the original analysis protocol is modified.

## Boundary: this is a receipt validator, not the upstream runner

The upstream Python runner remains the reproduction authority. This crate does not call it, port its statistical implementation, read arbitrary upstream JSON filenames, or claim that the 16-study suite is the original paper's 12-study snapshot.

The typed Rust API accepts normalized receipts. The checked-in schema defines the JSON boundary; the next increment should implement a narrow exporter from a *specific pinned upstream run artifact set* into these types. That exporter must first discover the actual output paths for the chosen run profile instead of assuming names such as `full_benchmark.json`, `evaluation_results.json` or `detailed_stats.csv` are stable upstream API contracts.

## Run

```sh
cargo fmt --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all-targets
```

Tests use wholly synthetic receipts. They are software conformance fixtures, not model performance.

## Rights and privacy

Store digests, aggregate outputs, study IDs and rights metadata in version control. Do not commit participant-level source data or raw human traces by default. Source-code licensing does not grant rights to redistribute each study's data, instruments, stimuli, or supplementary files. A study-specific rights/consent review remains required.
