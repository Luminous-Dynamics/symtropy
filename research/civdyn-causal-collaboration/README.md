# CivDyn Causal Collaboration v0.1

A deterministic Rust kernel for scoring an externally supplied multi-agent action trace and causal-action graph.

## Why this exists

AgentWorld (2026) evaluates long-horizon black-box collaboration with asymmetric roles and separates task success from Causal Collaboration Effectiveness (CCE), defined as the fraction of all actions in the causal ancestry of task success. AgentWorld reports 52% success and CCE 0.320 for its strongest tested main-set model; its human/LLM causal-edge agreement study reports 82% raw agreement and Cohen's kappa 0.64 on 84 judgments. This crate retains the deterministic CCE definition but adds explicit adjudication uncertainty:

- verified causal edges define the CCE lower bound;
- candidate edges define the inclusive upper bound;
- per-agent PAC bounds show whose actions contributed;
- message effectiveness is measured separately from message volume;
- cross-agent causal edges and useful endpoints expose whether contribution crossed agent boundaries;
- duplicate action fingerprints quantify repeated work.

The scorer does not infer causal relationships. It receives causal edges with evidence references and computes graph metrics deterministically. Model-proposed edges are candidate-only and cannot be marked verified.

## Metric semantics

For successful tasks:
- CCE lower = verified contributing actions / all actions.
- CCE upper = verified + candidate contributing actions / all actions.
- Contributing actions are the success actions plus their backwards causal ancestors.

For failed tasks:
- CCE lower and upper are both zero, consistent with the source AgentWorld convention.
- Partial checkpoint success is retained separately as PSR-like progress.

Per-agent PAC is contributing actions for the agent / all actions for that agent. Idle agents have no PAC ratio (None), not a fabricated zero denominator. Useful-effort share is each agent's contribution count divided by the number of contributing actions overall.

The graph only accepts edges that point strictly forward in a global action sequence. This makes the supplied graph acyclic by construction. Every edge needs an evidence reference; duplicate edges, unknown action IDs and evidence-free claims are rejected.

## Evidence status

Evidence kind describes the source (resource lineage, state transition, acknowledged communication, deterministic precondition, human adjudication, model proposal, other). Status is separate:
- Verified: admitted to the CCE lower bound.
- Candidate: admitted only to the inclusive upper bound.

A ModelProposed edge cannot be marked Verified. An independent receipt or adjudication is required before promotion. The graph scorer does not itself certify that an evidence reference is true.

## Synthetic reference fixture

The CLI emits a successful eight-action trace and a failed-task negative control. The successful trace includes miners, a courier, a smith, a wizard, and an idle agent; verified and candidate causal edges; two communication actions; a resource transfer; and a duplicate crafting fingerprint.

Expected fixture controls:
- successful CCE lower: 5/8 = 0.625;
- successful CCE upper: 7/8 = 0.875;
- verified cross-agent causal edges: 3;
- candidate-inclusive cross-agent causal edges: 4;
- two messages, zero verified message contribution and two candidate-inclusive message contributions;
- one repeated action fingerprint;
- failed-task CCE: 0, with partial success ratio 0.5.

All values are artificial software fixtures, not real-world observations and not a reproduction of AgentWorld or PARTNR.

## Run

```sh
cargo fmt --manifest-path research/civdyn-causal-collaboration/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-causal-collaboration/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-causal-collaboration/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-causal-collaboration/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-causal-collaboration/Cargo.toml --bin causal-collaboration-sweep
```

## Claim ceiling

This crate validates the graph metric implementation on hand-computed fixtures. It does not establish human-level collaboration, real causal truth or model performance. Actual benchmark integration requires licensed action traces, independently verified task outcomes, causal-edge evidence and human/model adjudication studies.

## Source and extension boundary

This is a scoring-kernel implementation motivated by AgentWorld (Shu et al., 2026), not a reproduction of the 100-task AgentWorld benchmark, its game sandbox, its model calls or its human annotation study.

Compared with the source CCE, the extension is evidence-qualified:
- model-proposed links remain candidate edges and cannot enter the verified lower bound;
- verified and candidate causal ancestries produce a lower/upper CCE interval;
- resource-lineage evidence and cross-agent causal paths are counted explicitly;
- per-agent PAC and useful-effort shares expose idle agents and concentrated contribution;
- message count and message effectiveness remain separate;
- repeated action fingerprints expose duplicate work.

The candidate upper bound is not a claim that every proposed edge is true. It is a sensitivity envelope until edge adjudication is independently resolved. The metric cannot infer causal truth from temporal order alone.
