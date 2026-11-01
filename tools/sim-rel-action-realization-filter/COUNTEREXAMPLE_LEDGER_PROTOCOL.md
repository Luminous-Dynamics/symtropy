# SIM-REL-034: append-only counterexample admission and sealed-holdout firewall

Status: finite model-family evidence and lifecycle protocol. This is not open-world model adequacy, a physical safety certificate, or a statistical coverage theorem.

## 1. Purpose

SIM-REL-029 already generates deterministic perturbations and replays a frozen shared policy against them. SIM-REL-034 closes a different gap: a falsification result needs a controlled, append-only path into the development family, and final evaluation data must not become an accidental source of adaptive tuning.

The ledger enforces a one-way sequence:

1. Initialize immutable development worlds and reserve sealed family-group IDs.
2. Admit only a validated, nontrivial Development-origin counterexample generated against the currently active policy.
3. Increment the model-family revision and invalidate the attacked policy.
4. Replan and independently replay the candidate shared policy against the complete current family.
5. Seal the final policy and family revision.
6. Consume each registered sealed holdout group at most once.

Calibration and sealed-holdout evidence are never eligible for development-family admission.

## 2. Evidence types

### 2.1 Split identity

Every candidate has exactly one origin split:
- Development: eligible for counterexample admission, subject to all other gates.
- Calibration: evaluation/calibration only; cannot expand the family.
- SealedHoldout: final evaluation only; cannot expand the family.

The family-group ID is the split unit. A Development candidate whose group ID collides with a pre-registered sealed group is rejected. This assumes the external split registry is honest; the ledger cannot infer whether two differently named groups share a physical source.

### 2.2 Counterexample provenance

Each admitted counterexample retains:
- source and candidate world IDs;
- exact source and candidate model-world snapshots;
- perturbation enum and parameters;
- generator ID and version;
- search-run ID and seed;
- family-group ID and split;
- identity of the policy that was falsified;
- a distinct validation receipt.

Policy, generator, search-run, manifest and artifact identities are opaque caller-supplied identifiers. This module deliberately does not call a non-cryptographic hash a digest or claim the supplied identifiers are cryptographically authenticated.

### 2.3 Validation receipt

Admission requires a non-empty validator ID/version and four explicit passing checks:
- schema validity;
- physical plausibility review;
- independent provenance verification;
- origin-split verification.

The ledger verifies the receipt fields and single-use receipt ID. It cannot prove the validator is competent or truthful. The underlying physical/plausibility evidence remains an external artifact referenced by the validation receipt.

## 3. Admission state transition

A candidate is rejected unless all conditions hold:
- Its witness class is not NoViolation.
- Candidate ID agrees with the witness specification and is distinct from the source ID.
- Source world exists in the current development family.
- Candidate kernel passes the complete kernel validator.
- Candidate ID is new and candidate kernel content is not already present under another ID.
- Candidate is generated against the currently active policy identity.
- Candidate split is Development and its family group does not overlap reserved sealed holdout groups.
- All validation checks pass and receipt IDs are single-use.
- Family and event resource caps are not exceeded.

Successful admission appends the candidate world, increments the family revision exactly once, records the prior source-world snapshot and candidate snapshot, records the invalidated policy identity, and removes active-policy status.

The ledger is append-only through its public API; internal world and event vectors are private. A failed admission must not expand the family.

## 4. Replan transition: the ledger performs the replay

A caller cannot reactivate a policy by passing a Boolean success flag or a claimed list of worlds.

The replan operation requires:
- current family revision;
- a candidate policy identity and actual policy tree;
- nominal world ID;
- loss table and observation protocol;
- complete command-cost vector and hazard loss;
- horizon and hazard/abstention budgets.

The ledger invokes the exact shared-policy replay itself against its stored current development worlds. It rejects:
- stale family revisions;
- malformed or incomplete public-observation branch coverage;
- invalid commands or terminal decisions;
- any world omitted from or unexpectedly absent from the canonical replay;
- hazard or abstention budget violation in any model world;
- resource exhaustion or replay arithmetic error.

Only a successful internal replay reactivates the policy. The record contains the actual plan, exact family revision, canonical replayed world IDs and per-world objective receipts. The opaque policy identity may remain unchanged if the re-evaluated plan remains valid; it is still bound to the new revision.

The replay is exact for the supplied finite worlds, loss table, horizon and policy tree. It does not establish that the true physical process is represented in those worlds.

## 5. Final seal and sealed evaluation

The final seal binds one active policy identity to one exact family revision. Once sealed:
- no counterexample can be admitted;
- no replan can be recorded;
- the policy identity and development family are immutable.

Each pre-registered sealed family group has a fixed manifest ID and exact world-ID list. A holdout evaluation receipt must:
- refer to a registered group;
- match the frozen policy identity and family revision;
- cover exactly the group's registered world IDs, with no missing, extra or duplicate ID;
- provide a unique event receipt ID and non-empty evaluation artifact ID.

A group is consumed whether its acceptance criteria pass or fail. Re-opening the same group to tune the policy is forbidden. Distinct pre-registered groups can be evaluated independently against the same frozen policy.

## 6. Independent audit surface

The snapshot exposes:
- current development-family revision and complete model-world snapshots;
- active or frozen policy identity;
- frozen policy revision;
- pre-registered sealed groups and their world IDs;
- consumed sealed groups;
- single-use validation receipt IDs;
- append-only sequenced events.

Every event has a unique caller-supplied receipt ID and a monotonic sequence. Family revisions never decrease. Admission increments revision; replan, seal and evaluation remain bound to the relevant revision.

The audit verifier should replay event transitions independently and check:
- candidate snapshots exactly match each admission event;
- source snapshots existed at the prior revision;
- each policy invalidation is followed by a successful exact replan before another attack;
- the replan's per-world metrics correspond to all current family IDs;
- no event mutates the family or policy after final sealing;
- each sealed group is consumed at most once.

## 7. Conformance controls

The authored tests cover:
- successful admission increments revision and invalidates policy;
- incomplete branch policies cannot reactivate because the ledger performs the exact replay;
- stale revision and failed replan rejection;
- NoViolation, Calibration and SealedHoldout admission rejection;
- reserved holdout group leakage rejection;
- stale source, stale policy, duplicate ID, duplicate model content and failed validation rejection;
- final policy sealing and one-time holdout consumption, including when acceptance criteria fail;
- wrong holdout group, world list, policy ID or revision rejection;
- duplicate event IDs and invalid registration rejection;
- sequenced event and snapshot integrity.

## 8. Benchmark protocol

- Exhaustively enumerate small campaign state machines and compare ledger transitions with an independently implemented transition oracle.
- Zero accepted calibration/holdout leakage.
- Zero duplicate candidate content or receipt reuse.
- Zero policy activation after an incomplete or budget-violating full-family replay.
- Zero family mutation after policy sealing.
- Measure attempted, rejected and admitted candidates by reason; family revisions; replan attempts/cost; false-counterexample frequency; held-out risk; policy-switch rate; and unconsumed holdout groups.
- Split by mechanism family, apparatus family and acquisition source, not random rows.
- Freeze policy and family before sealed holdout evaluation.
- Publish all seeds, rejection reasons, validation artifacts and consumed/unconsumed holdout records.

## 9. Literature and scope

This is an evidence-lifecycle mechanism for the existing finite counterexample-guided refinement programme, not a new theory of CEGAR.

Trustworthy scientific ML verification and validation guidance stresses explicit problem domains, verification, calibration, validation, and application domains: https://doi.org/10.1088/2632-2153/ae59ec

The SIM-REL sequential conformal module remains responsible for its separate fresh-fold calibration contract. SIM-REL-034 prevents those calibration or application/holdout observations from entering the development counterexample family through the ledger API.

Claim boundary: the ledger enforces declared provenance and split contracts. It cannot prove external group labels are honest, validator judgements are correct, the perturbation family is complete, or the true physical system lies inside the finite development family. No open-world proof, physical safety guarantee, or distribution-free coverage guarantee is claimed.

## 10. Immutable replay context and independent counterexample verification

At ledger initialization, one immutable `DecisionReplayContext` is captured:
- nominal development world ID;
- complete terminal `DecisionLossTable`;
- observation protocol;
- command-cost vector and hazard loss;
- finite horizon;
- development hazard and abstention budgets.

The initial policy is replayed against every baseline development world before it can be stored as active. Its plan and per-world metrics are retained in the snapshot.

Every later replan uses this stored context rather than accepting a new loss table, observation protocol, cost schedule or horizon. This prevents policy status from being changed by silently changing the evaluation problem between family revisions.

Counterexample admission independently replays the exact active policy on the current family plus the candidate world. The ledger classifies the actual result as:
- policy invalidation due to a newly reachable public observation branch;
- hazard-budget violation;
- abstention-budget violation;
- terminal-decision loss amplification over the current family worst case;
- no violation.

The claimed search-witness outcome must agree with the independently replayed typed class and exact metrics (policy-invalidation reason text may differ because the ledger records its own specific error). A NoViolation result or any mismatch is rejected. The actual replay-derived outcome is stored in the admission record. Search-run/generator provenance remains attached but is not treated as proof by itself.

## 11. Sealed holdout replay and computed acceptance

The sealed evaluation operation no longer accepts a caller-supplied pass/fail Boolean or caller-supplied loss/measurement semantics. It requires:
- the actual sealed `DecisionModelWorld` snapshots;
- an exact match to the pre-registered group world IDs;
- a frozen policy and revision;
- pre-registered numeric acceptance ceilings for worst-case total loss and worst-case terminal loss;
- pre-registered hazard and abstention budgets;
- a non-empty evaluation artifact ID.

The ledger validates every supplied holdout kernel and independently runs the frozen shared policy against the sealed worlds using the immutable replay context. It computes:
- exact per-world experiment cost, terminal loss, hazard probability and abstention probability;
- worst-case total loss and terminal loss;
- pass/fail by comparing all metrics to the supplied criteria.

The resulting event retains the registered manifest ID, holdout world snapshots, evaluation artifact ID, criteria, per-world metrics, worst-case values and computed verdict. The group is consumed whether the verdict passes or fails. A caller cannot reopen the same group and tune the policy against it.

The ledger verifies world IDs and captures the actual supplied snapshots, but the relationship between those snapshots and an external sealed manifest remains an external provenance contract. Opaque IDs are not cryptographic digests; a future signed-manifest or canonical content-addressing layer may strengthen that binding.

## 12. State-machine conformance additions

Additional hosted tests cover:
- initial policy branch completeness and baseline replay before activation;
- policy tree identity, not merely a caller-supplied policy label;
- independent candidate replay and rejection when the claimed counterexample outcome disagrees;
- immutable replay context through every replan;
- sealed holdout replay on actual supplied model worlds;
- computed passing and failing holdout verdicts, including retention of exact per-world metrics and manifest ID.

The current implementation still relies on the external validator to attest physical plausibility and correct split lineage. Independent ledger replay verifies the supplied finite-world consequences; it does not prove open-world model adequacy.


## 13. Revision-bound attacks and candidate-outcome replay

Counterexample provenance now includes the exact family revision attacked in addition to the opaque policy identity and exact policy tree. A still-identical policy tree on a later revision does not make an old search run admissible against that later family.

The candidate world ID is also checked against every pre-registered sealed world ID, not only the current development world IDs and sealed group labels.

Most importantly, the ledger does not trust the search report's outcome class. It independently replays the current active shared policy against the complete current family plus the candidate using the immutable replay context. It derives the actual result:
- policy contract invalidation for a newly reachable public branch or another classified policy-contract failure;
- hazard budget violation;
- abstention budget violation;
- terminal loss amplification beyond the current family worst-case baseline;
- no violation.

The witness claim must match the replay-derived typed outcome and exact metric payload. For policy-contract invalidation, the class must match and the ledger retains its own specific replay error string. A mismatched claim, NoViolation result, or unclassified replay error is rejected. The admission event stores the independently replayed outcome.

This turns candidate admission from “the generator says this is a counterexample” into “the ledger independently reproduced the claimed finite-world policy/decision failure.”


## 14. Calibration split reservation and deterministic evaluation

The ledger now requires calibration family groups to be registered separately from sealed holdout groups before search begins. Calibration and sealed group IDs must be globally disjoint; their world IDs must be disjoint from one another and from all baseline development world IDs.

A candidate is rejected as Development if either its family-group ID or its world ID aliases any reserved calibration or sealed group. This is stronger than checking the candidate's declared `EvidenceSplit` alone: the split-group registry is an independent admission constraint.

The constructor canonicalizes baseline world order, calibration/holdout group world-ID order, and group map order. Sealed world snapshots are canonicalized by world ID before exact replay and event recording. Reordering input world snapshots therefore does not change the replay result's canonical ordering.

Policy identities and sealed evaluation artifact IDs are single-use across the entire ledger lifetime, not just the currently invalidated revision or one holdout group. A stale ID cannot be recycled after later family revisions or across distinct sealed groups.

The holdout attempt is consumed after group/world/policy/manifest bindings pass, even if the shared-policy replay or checked metric aggregation fails. Replay failure is retained as `replay_succeeded=false` plus an explicit error, and cannot be retried against the same sealed group. A valid replay with failing acceptance criteria is also consumed.

