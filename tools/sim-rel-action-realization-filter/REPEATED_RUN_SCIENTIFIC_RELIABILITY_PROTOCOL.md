# SIM-REL-046 — repeated-run scientific reliability and task-cluster metrics

Status: first executable task-cluster reliability adapter on a research branch. Exact-head qualification is pending. This layer reports repeatability; it does not independently establish statistical generalization across task families.

## 1. Motivation

Scientific-agent outcomes can vary between repeated runs of the same task because of stochastic planning, non-deterministic execution, sampling, hidden state and tool behaviour.

A benchmark must distinguish:
- whether an agent can succeed at least once if allowed k attempts;
- whether it succeeds reliably on every one of k attempts;
- average run-level safe success;
- whether any attempt caused a hazard or required abstention.

Most importantly, repeating one task k times does not create k independent scientific task samples. The task instance is the cluster and the generalization unit; repetitions estimate within-task reliability.

This follows the distinction between pass@k and pass^k used in agent-evaluation practice. pass@k is a capability measure; it is not by itself a reliability guarantee.

## 2. Fixed repeated-run plan

A plan binds analysis ID/version, campaign and evaluation scope, SealedHoldout group, fixed agent panel, task/model/prompt/harness/toolset identities, reference/candidate arm identities, shared resource budget, fixed repetitions per task k, minimum valid task-cluster count per family, and the complete preregistered mechanism-family set.

This first version accepts SealedHoldout only for final reliability reporting. Development and Calibration results may be used for engineering but cannot be returned as a final sealed reliability receipt.

Each task/family cluster must contain exactly k paired repetitions. An incomplete cluster is rejected. A cluster in which some repetitions are valid and others invalid is also rejected; all k repetitions must agree on task validity.

The underlying SIM-REL-041 validator checks unique pair IDs, unique task/family/seed exposures, evidence artifacts, matched harness/model/scope/split and resource budgets.

## 3. Metrics

For each distinct task instance and each arm:
- pass@k: at least one safe success among k repetitions.
- pass^k: every one of k repetitions is a safe success.
- run safe-success rate: exact numerator / denominator across valid repeated runs.
- completion at least once / completion every run: reported separately from safety.
- any-hazard: whether any repetition observed a hazard.
- any-abstention: whether any repetition abstained.

A safe success is inherited from SIM-REL-041 per-arm validation and cannot co-exist with a hazard, non-completion or abstention.

The output retains every task-cluster outcome, not only family aggregates. This makes it possible to inspect which task instances are repeatably solved and which are stochastic.

Run-level rates are stored as exact integer numerator/denominator pairs. A family with no valid clusters has an undefined rate (None), not a fabricated zero.

## 4. Aggregation and inference boundary

Family pass@k and pass^k counts are calculated over distinct valid task clusters. Their aggregate rates are stored as exact numerator/valid-task-cluster-denominator pairs. Run-level safe-success rates use all valid paired repetitions in those clusters and are stored separately as exact numerator/valid-run-pair-denominator pairs. A family with no valid task clusters has undefined rates, not zero.

For generalization claims, the statistical sample size is the number of distinct task instances per mechanism family, not k times that number. The k repetitions from one task must never be fed to SIM-REL-042 or SIM-REL-045 as if they were independent task samples.

This module performs descriptive cluster aggregation only. It does not yet implement a cluster bootstrap, paired task-level permutation test, beta-binomial hierarchical model or confidence interval for the family pass@k/pass^k proportions. Those should be a separate inferential layer, with task clusters as the resampling units.

## 5. Interpretation

Consider two agents that both achieve pass@5 = 80%. Agent A may succeed once on most tasks but fail the other four attempts. Agent B may succeed on every attempt for 80% of tasks and fail every attempt on the rest. Their pass@5 is equal, but their pass^5 and operational reliability can differ sharply.

Both metrics matter: pass@k asks whether the agent can solve a task under a retry allowance; pass^k asks whether it can do so consistently across all repetitions.

Neither metric alone establishes safety. Any-hazard, completion and abstention remain separate.

## 6. Qualification tests

The focused tests cover same or similar pass@k but distinct pass^k, exact run-level success numerators and denominators, hazard and abstention separated from completion, incomplete task clusters, mixed valid/invalid repetitions, fully invalid cluster exclusion, family registry and manifest mismatch, and resource/harness/model/scope matching through SIM-REL-041.

The branch workflow must run pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

## 7. Limits

The adapter output describes outcomes under a declared repeated-run harness. It does not authenticate the agent stochastic seed generator, establish independence between task instances, correct for task-selection bias, or prove real-world reliability.

The distinction between task instance and run repetition is enforced structurally; the scientific sampling frame and independence assumptions still need to be justified by benchmark design.