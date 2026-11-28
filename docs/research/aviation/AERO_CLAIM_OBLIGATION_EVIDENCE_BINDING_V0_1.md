# AERO Claim Obligation Evidence Binding v0.1

**Status:** synthetic, source-bound evidence-class adapter increment; exact-head build/tests pending  
**Authority ceiling:** evidence-class mapping only; no complete claim decision, physical truth, engineering acceptance, physical-work authorization, airworthiness or certification  
**Relationship:** additive source adapter after claim-specification binding and obligation-profile derivation

## 1. Why this adapter exists

The existing ClaimGate has aggregate required-evidence admissions and an owner evidence cut. Those are valuable, but they do not yet establish the exact link between every required claim-specification evidence class and each owner-native evidence record. A single receiver watermark on an owner cut also cannot establish the first-seen time for every individual record.

This module adds a narrow source-bound contract:

1. Bind raw claim-specification bytes to the exact typed claim specification and decision context.
2. Derive the closed registered obligation profile from that claim specification.
3. Require one mapping entry for every derived EvidenceIntegrity obligation.
4. Bind each mapping to exact source references, the owner evidence cut, the trust-policy snapshot and an owner-approved mapping receipt.
5. Require one source receipt per owner-native record. The receipt binds owner, producer, claim, subject, configuration, acquisition interval, record digest, trust-policy snapshot, integrity disposition, producer-authority disposition, first-seen time and effectivity.
6. Recompute the owner source-receipt manifest and require it to equal the owner cut's receiver_receipt_manifest_sha256.
7. Require the receipt first-seen time to be no later than the owner-cut watermark and the historical as-known cutoff.
8. Preserve all missing, stale, disputed, rejected, late, unauthorized or out-of-scope facts as distinct reasons and dispositions.

The source receipt manifest deliberately does not include the final decision-context digest. This avoids a circular commitment: the manifest is first bound into the owner evidence cut; the decision-context digest then binds that owner cut; the mapping approval finally binds the exact decision context and complete mapping. Per-record source receipts instead bind the claim/subject/configuration and acquisition interval.

## 2. Digest chain

The input is linked in the following order:

raw claim payload -> typed claim semantic digest -> derived obligation profile/manifest

per-record source receipts -> source receipt manifest -> owner evidence cut digest

owner evidence cut + claim specification + evidence manifest -> decision-context digest

derived evidence-class obligations + exact refs + owner cut + context -> mapping manifest -> owner approval receipt

All manifests use the declared rust-serde-json-struct-order-v1 profile with deterministic sorting where declared. The CI-only Python oracle reconstructs the raw/semantic, evidence-reference, profile, obligation, source-receipt, owner-cut, context and mapping commitments independently.

## 3. Disposition semantics

- BOUND: every mandatory evidence-class obligation is bound to the exact declared evidence set; all source receipts and admissions are known, effective, integrity-verified and owner/producer-authorized under the supplied upstream dispositions.
- INCOMPLETE: evidence or source records are absent, not received, stale, not effective for the acquisition interval, or a dependency/reverification requirement remains.
- UNVERIFIED: one or more receipts, integrity dispositions, producer-authority states or historical chronology remain unresolved.
- REJECTED: claim/context/mapping/source digests disagree, a source or mapping receipt failed verification, the mapping names an undeclared/reused source, a record is revoked/withdrawn, authority is denied, or a receipt is outside its exact owner-cut commitment.

These statuses describe only the evidence-class mapping adapter. Even when its disposition is BOUND, complete_claim_assurance_state is always false.

## 4. Obligations explicitly not evaluated

The derived profile also contains claim identity, semantic validity, producer authority at the claim/decision level, model applicability, temporal policy, dependency observation, per-population-stratum model credibility, model uncertainty, assumptions and exclusions.

This module does not populate those complete obligations. They are listed in unassessed_obligation_ids, and the result includes NON_EVIDENCE_CLASS_OBLIGATIONS_REMAIN_UNASSESSED. Per-record producer authorization here is only one input to the evidence-class mapping; it does not satisfy the separate claim-level authority obligation. The existing ClaimGate remains the authoritative aggregate disposition until a complete source adapter demonstrates parity across its full frozen corpus.

## 5. Tests and adversarial controls

The Rust tests cover:
- exact class-to-obligation mapping on a synthetic, fully bound input;
- omission of one derived evidence-class obligation;
- a late source receipt appended to an older owner cut;
- a mapping approval receipt first seen after the historical cutoff;
- NOT_RECEIVED admissions remaining incomplete;
- producer authority denial;
- stale mapping-manifest approval after substitution;
- source-record digest substitution;
- order invariance of source receipts.

The CI-only Python oracle independently checks the digest chain and schema, then exercises ten mutation/permutation cases. Its outputs are contract-level checks, not receipt signature authentication.

## 6. Non-claims and remaining integration

- VERIFIED is an upstream trust-verifier disposition. This module does not authenticate signatures, certificates, keys or revocation lists.
- An owner receipt and manifest do not prove the physical truth, completeness or representativeness of the underlying measurement.
- BOUND does not imply the evidence is sufficient for model credibility, observer coverage, safety analysis or the claim's requested engineering decision.
- This module does not change or replace evaluate_claim_gate.
- No engineering acceptance, release, physical work or certification authority is granted.

The next source-adapter increment should bind claim-specification strata and uncertainty requirements to the exact ModelCredibilityCut report, then bind observer/change/effectivity evidence to their own source-native receipts. Only after all derived obligation classes have source-bound evaluators should the adapter compare its aggregate result against the existing ClaimGate over the complete frozen corpus.
