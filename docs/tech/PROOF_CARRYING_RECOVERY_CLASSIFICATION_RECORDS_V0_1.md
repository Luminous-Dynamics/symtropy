---
title: Proof-Carrying Recovery Classification Records
version: 0.1
status: research-design
date: 2026-10-18
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1915
parent_pr: https://github.com/Luminous-Dynamics/symtropy/pull/1914
---

# Proof-Carrying Recovery Classification Records v0.1

## Purpose

[RDO-005 / PR #1914](https://github.com/Luminous-Dynamics/symtropy/pull/1914) made applicability conditional on a versioned taxonomy, a reviewed classification status and a record reference. That still leaves an important gap: the reference may not resolve, its contents may have changed, a record may have been replayed for another case, and the decision may rely on expired or revoked material.

This tranche builds a content-verifiable packet for the exact synthetic classification records used by the scenario-envelope audit. It is intentionally a separate executable tool so a failing packet audit cannot be hidden by the parent tool's success.

The first version verifies **integrity and semantic binding**, not external authenticity or the truth of the claims.

## Paradigm shift: proof-carrying decisions, not trust-by-label

A decision should carry machine-checkable bindings to the records that make it applicable. Separate these assurance layers:

1. **Reference presence:** a case names a unique record, and exactly one record resolves.
2. **Content integrity:** the record's SHA-256 digest matches its exact canonical JSON payload and the case binding cites the same digest.
3. **Semantic binding:** the record's case, profile, domain, decision criteria, taxonomy version, family labels, classification status and evidence tags match the case, with no cross-case replay.
4. **Lifecycle validity:** a record is within its declared validity window, and the unique revocation entry is known and active under the packet's versioned synthetic policy.
5. **Origin authenticity and authority:** not established here. The fixtures explicitly declare `UNVERIFIED_SYNTHETIC`; there are no real reviewer signatures, keys, credentials, authorizations or trusted issuers.

A content hash is not a signature. It detects a mismatch against the cited digest, but does not prove who authored the record or that its contents are true. Version control makes the tested packet reproducible at an exact commit; it does not by itself establish the authority of a classification reviewer.

## Data packet

`docs/tech/recovery-classification-record-packet-v0.1.json` contains 17 records, 17 case-to-record bindings, and 17 synthetic revocation entries. Each record stores a canonical compact JSON payload as a UTF-8 string alongside its digest. The hash is computed over the exact bytes of that payload; the digest field is outside the payload to avoid self-reference.

Payload arrays used as sets are sorted before serialization. Payload fields are serialized in the frozen Rust struct order. A verifier rejects a payload whose serialized representation is not canonical, whose digest differs, or whose outer record ID does not match the embedded record ID.

Each case binding contains the case ID, record ID and expected digest. Exactly one binding must exist for every parent corpus case; every record and revocation entry must be referenced; orphaned, duplicate, missing or ambiguous identities fail closed.

The lifecycle is intentionally finite and synthetic: the packet declares a `decision_day`, each record declares issue/validity days, and each record has one active/revoked/unknown entry under the named policy. Those day counts are fixture units, not a proposed real-world retention policy.

## Independent evaluator and result semantics

Run the standalone Rust crate with Cargo. It embeds the exact parent scenario corpus, reads the content-bound record packet, and emits a deterministic JSON receipt with:
- packet and algorithm identifiers, canonical packet digest and optional exact source commit;
- total record, binding, revocation and case counts;
- per-case content-integrity, case-binding, profile-scope and lifecycle states;
- synthetic candidate count, a persistent `UNVERIFIED_SYNTHETIC` authenticity ceiling, and actuation-disabled status;
- sorted errors from structural, digest, semantic, version, validity or revocation failures.

A record integrity/binding match does not automatically make a case a candidate. The candidate count is limited to cases that the parent corpus already labels `REVIEW_ONLY_CANDIDATE`, and which also have matching record integrity and case binding, in-scope profile applicability, and valid active lifecycle state. This remains a synthetic eligibility check for further human review—not permission to act.

The audit rejects duplicate IDs, missing/orphan records or bindings, cross-case replay, noncanonical or digest-mismatched payloads, binding digest mismatches, profile/criteria/taxonomy/family mismatches, unknown classification/revocation states, expired or not-yet-valid records, revoked/unknown revocation states, invalid lifecycle ordering, authenticity claims above the fixture ceiling, and actuation-authorized outputs.

## Frozen synthetic baseline

The parent applicability corpus contains 17 scenarios across forestry recovery, PV second-life and infrastructure recovery. The packet binds each case to one corresponding synthetic record.

| Receipt measure | Expected value |
|---|---:|
| Records / case bindings / revocation entries | 17 / 17 / 17 |
| Cases | 17 |
| Record integrity MATCH | 17 |
| Case binding MATCH | 17 |
| Profile applicability IN_SCOPE / OUT_OF_SCOPE | 10 / 7 |
| Lifecycle VALID_ACTIVE | 17 |
| Synthetic review candidates | 3 |
| Authenticity verified | false |
| Actuation authorized | false |

These are frozen fixture counts, not field coverage, real record volume or an estimate of risk prevalence.

## Adversarial tests

The Rust suite freezes tests for:
- payload tampering without digest update;
- binding digest mismatch;
- semantic mutation even after rehashing;
- taxonomy mismatch even after rehashing;
- cross-case record replay;
- expiration and revocation;
- unknown revocation status and future revocation entries;
- missing, duplicate and orphan bindings/records;
- unauthorized elevation of the authenticity claim;
- invalid exact-source commit;
- deterministic results under input row permutation;
- unknown profiles and a mandatory no-actuation ceiling.

Mutations that rehash a modified payload are important: the digest alone is not the policy oracle. Semantic checks must still reject a modified taxonomy, criteria regime or case identity.

## Proposed benchmarks (targets, not achieved results)

| Benchmark | Proposed target | Claim ceiling |
|---|---|---|
| Frozen packet conformance | 100% | Only the exact finite synthetic packet |
| Record and case binding cardinality | 100% exact, no duplicates/orphans | Contract integrity |
| Frozen digest tamper detection | 100% | Only the exercised mutations |
| Cross-case replay detection | 100% | Finite replay fixtures |
| Expired/revoked/unknown lifecycle state promoted | 0 | Test harness behavior |
| Semantic/version mismatch promoted | 0 | Declared profile and fields |
| Row-order invariance | Byte-identical canonical receipt | Determinism under defined canonicalization |
| Candidate with integrity, binding, scope or lifecycle failure | 0 | Synthetic gate behavior |
| Actuation-authorized output | 0 | Output contract only |
| Scale characterization | 10 / 100 / 1,000 / 10,000 records | Measure p50/p95 latency, throughput and peak memory |

Do not report the target table as measured achievement. The first implementation should establish repeatable finite-model conformance; real historical replay and empirical performance come afterwards.

## Research basis

- NIST SP 800-63A-4 (July 2025) distinguishes whether evidence is authentic, accurate and valid, and discusses cryptographic validation of digital evidence and its issuing source. This project adopts that separation as a conceptual design constraint, not as a claim of identity-proofing compliance: https://pages.nist.gov/800-63-4/sp800-63a.html
- NIST FIPS 186-5 and FIPS 205 describe digital signatures as mechanisms for detecting modification and authenticating the signatory. This v0.1 packet has no signatures, so it must not claim signatory identity or authenticity: https://csrc.nist.gov/pubs/fips/186-5/final and https://csrc.nist.gov/pubs/fips/205/final
- SLSA's provenance format is a useful example of structured, versioned provenance statements binding outputs to inputs and build context. It also makes explicit that the producer remains part of the trust model; a provenance-shaped statement is not self-authenticating: https://slsa.dev/spec/v1.2/provenance
- Nakashima et al. (2026), *Dynamic Adaptive Pathways Planning with Changing Criteria for Cross-Sectoral Climate Adaptation*, argues that a changing criteria regime can change the interpretation of current and future pathways. It supports version-bound criteria semantics, but does not validate this recovery evaluator: https://doi.org/10.1016/j.procs.2026.08.180

## Boundaries

All initial record contents, reviewers, timestamps and revocation entries are synthetic. No cryptographic signature scheme, trust store, identity proofing, professional qualification, consent, legal authorization, field safety assessment, factual validation or external revocation service is implemented. A matching digest only establishes consistency with the digest that the packet cites. Passing the audit establishes finite content-integrity and semantic-binding conformance for this fixture—not truth, authenticity, field accuracy, product safety, economic benefits or legal compliance.
