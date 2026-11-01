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