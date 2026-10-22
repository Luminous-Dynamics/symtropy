# CIV-INFRA Decision-Impact Planner v0.1

A standalone Rust tool that ranks candidate surveys/measurements using the decision ambiguity they can remove over a **declared finite set of states**. It sits after the primitive-derived evaluator (#1784 / CIV-INFRA-ORACLE-002) and does not create a second simulator.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/civ-infra-decision-impact-planner/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-decision-impact-planner/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-decision-impact-planner/Cargo.toml -- \
  tools/civ-infra-decision-impact-planner/fixtures/decision-impact-v0.1.json \
  /tmp/civ-infra-decision-impact-report.json
\`\`\`

The CLI hashes the exact input bytes and binds that digest to the report. Each action is bound to a named outcome-model reference; an optional SHA-256 is syntax-checked and preserved. It rejects malformed, duplicate, or incomplete state partitions instead of filling gaps with assumptions.

## What it ranks

- Worst-case number of distinct decision signatures left after an observation.
- Worst-case decision-discordant state pairs still unresolved.
- Worst-case number of states left.
- Number of decision-discordant state pairs that the outcome partition would separate.

The ranking is an explicit lexicographic rule, not a weighted scalar. Estimated cost and duration are retained with their units and provenance basis, but are not silently combined into an invented utility score. The user can see trade-offs instead of inheriting hidden weights.

No probabilities are assigned. An outcome partition says which declared states an outcome could match; it does **not** mean those outcomes are equally likely, calibrated, or even physically achievable. Expected value of information is out of scope until defensible priors and observation likelihoods are separately admitted.

## Deterministic evidence

- Frozen 4-state, 3-candidate synthetic fixture.
- 18 Rust tests including fail-closed partition mutations and an independently implemented pairwise oracle that checks action metrics and the final lexicographic ranking.
- A seeded/index-driven generated suite: 100 seeded worlds × 20 states × 5 actions = 10,000 state-action evaluations; the generator has explicit seed-sensitivity and multi-signature-diversity assertions.
- CI can emit full generated inputs, input digests, reference summaries, and planner receipts through CIV_INFRA_DECISION_CORPUS_OUT.
- Permuting state/action/outcome order cannot change impact metrics or action ranking.
- A work cap rejects more than 10,000,000 state-action evaluations before evaluating actions.
- Action reports preserve the declared outcome-model reference and optional digest so downstream review can see the basis of each partition assumption.

## Claim ceiling

Upstream decision signatures and outcome partitions are explicit inputs. v0.1 verifies their shape and their downstream finite-state consequences; it does not verify that a survey will produce those outcomes, re-read the referenced upstream reports, or independently establish civil-engineering truth. Optional upstream-report and outcome-model SHA-256 fields are syntax-checked, not resolved against an external artifact store. The bundled fixture is fully synthetic and uses no solver-backed hydraulic or utility evidence. See the protocol for the planned integration boundary.
