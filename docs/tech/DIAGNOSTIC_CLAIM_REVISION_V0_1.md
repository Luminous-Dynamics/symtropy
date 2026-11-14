# Diagnostic claim revision: immutable history, revisable current diagnosis

**Workstream:** DIAGNOSTIC-CLAIM-REVISION-001  
**Parent issue:** [#5085](https://github.com/Luminous-Dynamics/symtropy/issues/5085)  
**Status:** source contract and synthetic tests authored; hosted qualification pending  
**Scope:** read-only diagnostic evidence. No controller, actuator, maintenance or operating-authority path.

## The paradigm shift

A diagnosis must be both historically immutable and epistemically revisable.

These are not contradictory requirements:

- The system must never rewrite a prediction that it actually emitted at an earlier time.
- The system must be able to revise its *current* engineering conclusion when genuinely new evidence arrives.
- The new conclusion must explicitly identify why it changed, which evidence became available, and which prior evidence was invalidated.
- A correction must not erase the record that an earlier diagnostic claim existed, was acted upon, or may have caused cost or downtime.

This is a missing lifecycle distinction between a streaming output, a belief state, and the current operational disposition. The current implementation is a reference ledger for that distinction; it does not prove that the upstream diagnosis or source appraisal is correct.

## How the new module composes with existing work

The module deliberately owns only revision mechanics:

| Existing owner | Responsibility retained | New ledger contribution |
|---|---|---|
| streaming_contract.rs | One sample in, one output out; input/output sequence and receipt integrity | Does not revise past sample outputs |
| behavioral_sets.rs | Which declared physical hypotheses remain compatible with evidence | Stores successive dispositions, including ambiguity |
| sequential_evidence.rs | Calibrated block-level sequential evidence under explicit dependence assumptions | Consumes no raw-sample pseudoreplication; records downstream disposition revisions |
| diagnosis_revision.rs | — | Evidence chronology, revision kind, invalidation/tombstones, current disposition and replay |

A sequential likelihood ratio crossing is not itself proof of a physical cause. A behavioral-set singleton is only unique within its declared catalogue. The revision ledger preserves those claim ceilings rather than upgrading them.

## Contract

Each receipt binds a fixed run, recording, asset, configuration digest, model digest and hypothesis-catalogue digest. Model or configuration changes start a new lineage; they do not silently alter the meaning of prior revisions.

Each revision adds at least one previously unseen evidence digest and records:
- source identifier and role;
- time the observation occurred;
- time the evidence became available to the diagnoser;
- its declared relationship to the current disposition;
- the new disposition and, where applicable, invalidation of prior evidence;
- contiguous revision ordinal, prior receipt digest and current active-evidence-set digest.

Temporal admissibility is:

`observed_at le available_at le issued_at`

Late-arriving evidence may cause a later revision, but cannot rewrite what the system knew at an earlier decision time.

### Revision kinds

- **Initial assessment:** establishes the first disposition with evidence.
- **Corroboration:** adds supporting evidence while retaining the same disposition.
- **Reassessment:** changes the current disposition because new evidence contradicts, narrows, expands or changes the operating context.
- **Evidence retraction:** explicitly invalidates earlier active evidence using a new, digest-bound invalidation basis. A withdrawal is allowed only through this route.

Invalidated evidence becomes a permanent tombstone. It cannot be resurrected under a different source wrapper. A withdrawn lineage closes; a later assessment requires a new lineage identity.

### Disposition vocabulary

- No fault.
- One or more known hypotheses. A singleton means unique only inside the frozen catalogue.
- Unknown or out of support.
- Not evaluable.
- Explicitly withdrawn.

Missingness is not evidence of health, and missingness alone cannot withdraw a prior diagnosis.

## Why this matters for the field

Many diagnosis pipelines emit one label per time step, while their training and evaluation metrics score those labels independently. The engineering workflow needs a second, explicit question:

**What is the current disposition as of time t, and what evidence arrived after the earlier disposition that justifies changing it?**

This separates:
1. what the system believed at each historical decision time;
2. what later evidence invalidated or corroborated;
3. what the system currently believes;
4. what an engineer actually reviewed or authorized.

That makes false retractions, late corrections and evidence invalidation measurable instead of hiding them inside an accuracy score.

The distinction also makes maintenance evaluation more honest. A diagnosis can be withdrawn without erasing that a work order was previously recommended. Any intervention and its outcome must remain linked to the diagnosis version that caused it.

## Research positioning and external comparators

This capability is intended to compose with—not replace—four established diagnostic approaches:

1. **Analytical/model-based diagnosis:** structural residuals, MSO reasoning, and physical response models remain necessary baselines.
2. **Statistical process control:** remains a low-complexity baseline for timely change detection.
3. **Deep and grey-box diagnosis:** remains a strong pattern/generalization baseline; the DX2026 LiU-ICE solution is a published grey-box comparator.
4. **Open-set and conformal diagnosis:** prediction sets and unknown rejection remain important uncertainty baselines.

A 2026 mechanical OOD survey distinguishes condition shifts, fault-semantic shifts and combined shifts, and emphasizes evaluating known-fault generalization and unknown-fault rejection separately: https://doi.org/10.1007/s10462-026-11722-3. A 2026 class-conditional conformal diagnosis paper proposes class-specific prediction sets and empty-set rejection for open-set fault diagnosis: https://doi.org/10.1016/j.jprocont.2026.103701.

The revision ledger does not claim those methods are inferior. It defines an additional evaluation axis that can be used with any of them: whether the diagnostic lifecycle remains coherent as evidence is added, contradicted, invalidated or withdrawn.

For industry comparisons, commercial platforms such as Siemens Senseye, Wärtsilä Expert Insight and ABB Ability are deployment/integration comparators, not matched score baselines. A fair comparison needs common assets, evidence availability, costs and outcome definitions. We should not claim a commercial advantage from synthetic ledger tests.

## Benchmark protocol

The 14-case synthetic corpus tests:
- initial claim followed by contradictory evidence;
- corroboration without changing disposition;
- late-arriving evidence;
- unavailable/future evidence;
- duplicate wrappers for the same evidence bytes;
- evidence invalidation and tombstone enforcement;
- ambiguity-to-singleton transitions;
- unknown-to-known coercion;
- model/configuration lineage changes;
- receipt tampering and time non-monotonicity;
- missingness-only withdrawal and post-withdrawal mutation.

The Rust unit tests exercise the receipt/session semantics. The JSON corpus is a protocol manifest, not empirical data. The schema is a structural contract; the Rust validator additionally checks cross-field and temporal semantics.

### Measures

Report these separately from the LiU-ICE FAR/TDR/FIA score:

| Metric | Definition |
|---|---|
| Revision precision | Fraction of disposition changes supported by newly available evidence and a valid declared transition |
| Missed revision rate | Contradicted or invalidated current claims not revised after qualifying evidence becomes available |
| False retraction rate | Claims withdrawn without valid invalidation evidence |
| Revision latency | Time from evidence availability to the next qualified current-disposition update |
| Ambiguity preservation | Fraction of cases retaining multiple hypotheses when the evidence does not distinguish them |
| Evidence replay integrity | Exact replay success and counts of tampered, forked or missing receipts |
| Historical mutation count | Number of prior receipts/outputs changed or deleted; target zero |
| Intervention lineage integrity | Fraction of interventions linked to the exact diagnostic revision that prompted them |

## Aspirational research targets (not results)

- 100% identical replay for valid revision histories.
- Zero accepted duplicate, future-available, resurrected or ungrounded evidence references in the frozen corpus.
- Zero current-disposition changes without evidence-bearing revisions.
- Zero historical receipt mutations or forked chains.
- Every singleton transition identifies the evidence that removed competing hypotheses.
- Every withdrawal distinguishes actual evidence invalidation from sensor missingness, model failure and simple loss of support.
- P95 validation under 1 ms for 10,000 active evidence references on pinned hardware, after measurement.

## Claim ceiling

The module checks the consistency and chronology of *declared* evidence and revisions. Hashes bind bytes; they do not authenticate issuers or prove measurement truth. A declared effect is not an independently verified causal fact. A unique hypothesis is unique only within the declared catalogue.

This work does not establish fault-diagnosis accuracy, calibration validity, physical cause, maintenance benefit, safety, standards compliance, controller authority or permission to operate.
