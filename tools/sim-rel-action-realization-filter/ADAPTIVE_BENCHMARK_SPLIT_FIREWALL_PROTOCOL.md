# SIM-REL-049 — split-firewalled adaptive scientific benchmark generation

Status: first lifecycle verifier implemented on a research branch. Exact-head qualification pending. It validates the lifecycle/provenance contracts; it does not prove corpus-wide non-contamination or authenticate the generator/verifier/auditor.

## 1. Why adaptive benchmarks need a firewall

Static scientific-agent benchmarks saturate as agents learn fixed task surfaces, solution formats and shortcuts. AutoSciBench (October 2026) explores recipe-based adaptive generation: a task concept declares domain/modality/reasoning, while a low-level recipe constructs the question, environment and ground-truth answer. Solver trajectories and judge feedback help close shortcuts and revise recipes. Its paper reports reductions in average solver accuracy of 22.4 and 25.5 percentage points against curated tasks in computational biology and materials science.

Adaptation is not automatically valid. A generator can overfit one solver; generator and verifier can share a bug; task recipes can become unsolvable or ambiguous; and a hidden holdout becomes a training set if its labels/trajectories are used to refine the next release.

Princeton's agent-evaluation guidance recommends secret holdouts, overlap reporting, trajectory inspection, clean-room harnesses and limited holdout reuse. SIM-REL-049 makes the split firewall an executable contract.

References:
- AutoSciBench (4 October 2026): https://arxiv.org/abs/2610.05140
- Princeton evaluation guidance: https://sage.cs.princeton.edu/trends/
- ScienceAgentBench: https://hal.cs.princeton.edu/scienceagentbench
- Agentic evaluation review (2026): https://link.springer.com/article/10.1007/s10462-026-11571-0

## 2. Lifecycle states

### Development — adaptation is allowed

Development adaptation events may consume Development-only source artifacts, solver trajectories and judge feedback. Every event binds:
- contiguous sequence and unique event ID;
- registered Development family group;
- seed namespace;
- concept and candidate recipe-template digests;
- optional parent recipe;
- solver panel, full trajectory and judge-feedback digests;
- split-bound source artifact inputs;
- monotonic adaptation timestamp.

A Development event that names a Calibration or SealedHoldout input is rejected.

### Freeze — the adaptive policy becomes immutable

Before Calibration or SealedHoldout generation, a freeze receipt binds:
- benchmark/version and plan digest;
- exact generator artifact and refinement-policy digests;
- concept registry, mechanism-family registry and recipe-schema digests;
- oracle-verifier and semantic-auditor identities;
- complete frozen concept and recipe-template registries;
- adaptation-transcript digest and event count;
- freeze timestamp.

Every concept/recipe produced by the supplied Development transcript must appear in the frozen registry. Later recipe changes require a new benchmark version and freeze receipt.

The current verifier shape-checks and cross-binds supplied digests. It does not implement cryptographic recomputation or authenticate the issuer; that belongs to the SIM-REL-043 evidence/provenance layer.

### Calibration — qualification only, no adaptation

Calibration runs after freeze using a seed namespace not used in Development. It binds the freeze receipt, frozen generator/verifier, registered Calibration group and a distinct seed commitment.

Calibration must meet the preregistered floor for task count, independent oracle verification and solvability, and must remain below the declared ambiguity cap. The aggregate qualification receipt is the only Calibration input allowed into sealed generation. Calibration task-level answers, raw labels and solver outcomes are not generator inputs.

A failed calibration gate blocks the sealed release. It does not permit recipe mutation after freeze. A corrected generator requires a new version and a new freeze.

### SealedHoldout — hidden, rotated evaluation release

The sealed release binds:
- frozen plan and freeze receipt;
- passed aggregate Calibration receipt;
- registered SealedHoldout group;
- a hidden seed namespace distinct from every Development and Calibration namespace;
- a hidden seed commitment not included in generation inputs;
- generator artifact and recipe identities;
- split-typed generation-input inventory;
- all concrete task artifacts and independent oracle validation receipts;
- prior public/private exposure ledger;
- semantic overlap auditor identity and artifact digest;
- release timestamp and immutable release digest.

The release generator may consume frozen-plan material, admitted Development evidence and the aggregate Calibration qualification receipt. It may not consume any SealedHoldout artifact, hidden seed commitment, oracle answer or validation receipt. Each task carries the same exact input inventory as the release manifest.

## 3. Independent generation, verification and overlap audit

Three roles are distinct:
1. **Generator** constructs the challenge and hidden ground truth.
2. **Oracle verifier** independently checks solvability, unique ground truth, negative controls and ambiguity against the concrete task.
3. **Semantic overlap auditor** checks whether a new challenge is too similar to previously exposed tasks.

The plan requires distinct identities and artifact digests for all three. The verifier receipt binds the public-task digest to the sealed oracle commitment and records all required pass flags. The semantic auditor is a provider interface; hashes alone cannot detect paraphrase or strategy-level task leakage.

The initial implementation fails closed on a potential near-duplicate. It also rejects exact duplicates by digest before invoking the semantic auditor. False-positive semantic overlap requires independent adjudication/removal and a new immutable release; it must not be waved through by the generator itself.

## 4. Exposure and contamination controls

A prior exposure record binds:
- unique exposure ID and previous benchmark release ID;
- exposed task digest;
- agent and version;
- prompt/scaffold digest;
- exposure timestamp.

The verifier checks exact task-digest collisions and performs pairwise semantic comparisons against prior exposures and within the new release. A configured comparison cap prevents an unexpectedly large release from creating unbounded audit work.

Additional production requirements:
- publish test/train/prompt/scaffold/tool overlap reports;
- inspect trajectories for solution-artifact leakage and benchmark-specific prompt tuning;
- maintain a bounded holdout reuse policy and rotate hidden task families;
- retain hidden seed custody outside the generator/solver account;
- keep task-instance, oracle-answer and validation artifacts sealed until adjudication;
- use canary artifacts to detect unauthorized exposure;
- disclose the number of prior uses and affected agent versions.

A secret static holdout is not secret after release. The system therefore needs new hidden seeds and refreshed challenge families over time.

## 5. Task quality and coverage outputs

The lifecycle receipt reports counts and pass/fail state; a production benchmark dashboard should add:
- domain, modality and reasoning-profile coverage vector/entropy;
- concept/family coverage and rare-regime counts;
- solvability, unique-answer, ambiguity and verifier disagreement rates;
- difficulty distribution across a fixed reference panel;
- pass-rate floor/ceiling and cross-agent discrimination;
- shortcut closure/adversarial perturbation resistance;
- exact/semantic duplicate and exposure overlap rates;
- generation/replay cost and reliability.

Do not equate lower solver accuracy with better task quality. Harder tasks must remain solvable, unambiguous, scientifically meaningful and discriminative across a fixed reference panel.

## 6. Qualification tests

Focused tests cover:
- a valid Development → freeze → Calibration → SealedHoldout sequence;
- Development attempts to consume Calibration/SealedHoldout artifacts;
- adaptation after freeze and omitted recipe/concept in frozen registries;
- Calibration policy/freeze mutation, insufficient verification or quality-gate failure;
- hidden seed namespace reuse across Development, Calibration and SealedHoldout;
- sealed answer commitment and oracle validation receipt leakage into generation inputs;
- generator/verifier/auditor identity collisions;
- task recipe/family/split/domain/modality/reasoning mismatch;
- unsolvable, ambiguous or failed-negative-control tasks;
- exact duplicate exposure and semantic near-duplicate rejection;
- exposure ledger identity/timestamp validation;
- semantic comparison workload cap.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

## 7. Claim boundary

A passing lifecycle receipt establishes split/provenance consistency under the supplied IDs, digests and verifier/auditor providers. It does not prove:
- the semantic auditor is unbiased or complete;
- every prior task or solution is known to the exposure ledger;
- corpus-wide non-contamination;
- the hidden seed remains secret outside the declared custody process;
- the task represents a scientifically useful real-world problem;
- the generator/verifier code is cryptographically authentic;
- the independent oracle is correct.

Use SIM-REL-043 for evidence provenance and real cryptographic verification; use SIM-REL-041–048 for matched conditions, statistical inference and repeated-run reliability. No SOTA claim follows from benchmark hardness alone.
