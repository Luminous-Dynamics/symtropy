# Construction robot claim-admissibility validator

Rust validator for the claim-specific evidence proposal in this directory.

## Input and output

The CLI accepts a single packet and optional output path:

```sh
cargo run -p symtropy-construction-evidence-validator -- \
  --packet research/construction-evidence-claim-admissibility-v0-2/fixtures/valid/bench-stopping-distance-claim.json
```

Exit 0 means the packet was structurally evaluated. Each claim receives one computed outcome: `admissible_for_named_claim`, `inadmissible_for_named_claim`, `indeterminate`, or `not_evaluated`. A non-admissible claim is still a valid evaluation result. Exit 2 means the invocation, packet, schema/profile binding, or reference is invalid.

The CLI accepts no runtime profile or schema override. Packet, profile, and output schemas plus the requirements catalog are embedded with `include_str!`, validated against Draft 2020-12, restricted to local references, and SHA-256 pinned in source. The whole profile catalog digest is bound by each claim.

## Non-goals and limitations

This is an evidence-admissibility evaluator, not a robot safety controller or certification authority. It does not itself fetch/authenticate referenced source bytes, cryptographically verify signatures, validate issuer/custodian/assessor identity or competence, resolve Symthaea's common-mode dependency lineage, independently confirm metrology, establish physical truth, or authorize operation. Those remain with their owning systems and competent assessors.

The evaluator never sums sample counts across evidence units because this version lacks a verified overlap/identity model. Cross-site generalization requires distinct site and configuration replication dimensions. A bench claim is limited to bench origin and cannot establish field performance.

The profiles are explicit research/benchmark hypotheses, not universal regulatory thresholds. Physical trial use requires a preregistered task-specific risk/statistical plan and independent metrology. Fixtures are synthetic and invented.
