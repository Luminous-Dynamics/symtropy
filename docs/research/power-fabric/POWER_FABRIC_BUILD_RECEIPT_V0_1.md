# Power Fabric reproducible build receipt v0.1

**Status:** executable digest/lock-manifest contract; no signature, trusted timestamp, equipment attestation or installation certification  
**Date:** 2026-11-14  
**Related:** IR-to-graph bridge PR #5245; standards adapter PR #5229; benchmark issue #5241

## 1. Purpose

A Power Fabric output must be reconstructable from the exact source data, mapping policy, exact installed module identities/configuration, compiler/toolchain, dependency lock and applicable standard profiles that produced it.

The build receipt records four distinct SHA-256 digests:
1. validated source IR document;
2. explicit IR-to-ServiceGraph mapping profile;
3. compiled ServiceGraph payload;
4. complete build lock material.

These digests answer different questions. The caller-provided GraphRevision.content_digest remains a declared revision input and is recorded separately; the compiler's computed service_graph_payload_digest is the digest of the serialized graph actually produced by this build.

## 2. Locked inputs

Every receipt requires:
- source IR document ID, revision and schema version;
- exact compiler ID and version;
- SHA-256 digest of the exact compiler source tree/commit;
- build-configuration digest covering Cargo features, build profile, target flags and the relevant Nix/CI configuration;
- toolchain fingerprint (at minimum Rust compiler version and compilation target);
- SHA-256 digest of the workspace dependency lockfile (Cargo.lock);
- mapping profile ID/version, explicit namespaced maps and per-object exact module/configuration identities;
- graph ID/revision/maturity and declared graph content digest;
- at least one standard applicability profile pin;
- each standard profile's exact ID/version, standard/part, edition, amendments, correction notices/corrected-consolidated-version label, publication status and jurisdiction;
- digest of the standards-register record used to identify the publication;
- digest of the profile's applicability-rule set;
- upstream artifact IDs/revisions/roles and SHA-256 digests.

An unknown standard publication lifecycle cannot be pinned as a complete build input. Published editions and working drafts remain distinct values. A working-draft profile can be pinned for research but must not be relabelled as published. Correction notices are recorded separately from edition and amendment identifiers: IEC 60364-8-82 remains edition 1.1, IEC 60364-8-82:2022+AMD1:2026 CSV, with the catalogue currently labelling the consolidated copy CORRECTED VERSION 2026-11.

## 3. Deterministic serialization contract

v0.1 hashes serde_json::to_vec over validated Rust values under this explicit profile:

serde-json-struct-order-btree-map-values-sorted-arrays-v1

- Struct fields serialize in declaration order.
- PowerFabricDocument's BTreeMap-backed object, relationship, claim and extension collections serialize as values in BTreeMap key order.
- IR-to-graph type maps serialize as arrays of {source, target} records in NamespacedType/ClaimDomain BTreeMap order, not JSON object keys.
- ServiceGraph node and edge vectors are sorted by the validated graph constructor.
- Standard profile pins and upstream artifact pins are sorted by stable ID and revision before hashing.

This is a declared serde serialization profile, not a claim of RFC 8785 JSON Canonicalization Scheme compliance. A change to serialization behavior, the compiler, or the dependency lock changes the lock digest and requires a new receipt.

## 4. Build lock digest

The lock digest covers:
- receipt and canonicalization profile versions;
- compiler ID/version, compiler-source digest, build-configuration digest and toolchain fingerprint;
- dependency-lock digest;
- source document ID/revision/schema and source digest;
- mapping profile ID/version and profile digest;
- graph ID/revision/maturity and declared content digest;
- computed ServiceGraph payload digest, including the exact module identities/configuration attached to nodes;
- all standard profile pins and upstream artifact pins.

The receipt is reproducible: an independent verifier with the same inputs and serialization profile can recompute all four digests and compare them. It is not authentic by itself: SHA-256 does not prove who supplied a source, whether a piece of evidence is truthful, when a measurement occurred, or whether the equipment is physically installed as described.

## 5. Why this matters

This creates the foundation for a physical-infrastructure equivalent of a lockfile/build manifest:
- documentation can be traced to the exact graph revision;
- an adapter update creates a new mapping-profile digest;
- a standard edition/amendment change creates a different build lock;
- a changed equipment catalogue, upstream model, or Cargo.lock creates a new build lock;
- old receipts remain reproducible rather than being silently rewritten.

Next increments should add role-projection artifact digests, export/import output receipts, independent verification signatures, and a dependency invalidation bridge. The module-identity mutation test explicitly verifies that changing one mapped module configuration changes the mapping-profile digest, compiled graph payload digest and build-lock digest while leaving the source IR digest unchanged. Signatures must be separate from these content hashes and must identify the signer, authority, key lifecycle and signing time.

## 6. Claim ceiling

The receipt demonstrates only that specified serialized inputs and outputs are bound by digests under the declared serialization contract. It does not authenticate any source, attest a physical installation, prove source-path completeness, validate engineering calculations, establish isolation, certify standards conformity, or issue a Certificate of Compliance.


## Correction-state discipline (2026-11)

The IEC Webstore identifies the consolidated IEC 60364-8-82:2022+AMD1:2026 CSV as **CORRECTED VERSION 2026-11**, while the same page reports Edition 1.1 and publication date 2026-04-16. Power Fabric stores this correction label in `StandardReference.corrections`, not as a new edition or amendment. A different correction label therefore changes the build-lock digest without falsifying the edition history. Source: https://webstore.iec.ch/en/publication/113148


## Compiler identity and build configuration

Compiler version strings are not sufficient to identify executable code. The receipt therefore requires a SHA-256 digest of the exact compiler source tree and a separate build-configuration digest. The configuration digest should cover enabled Cargo features, release/debug profile, target triple/flags, linker settings and relevant Nix/CI environment declarations. These digests are supplied by the build orchestrator and must be computed from the actual immutable source/configuration inputs, not typed as descriptive labels.
