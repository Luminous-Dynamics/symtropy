# Power Fabric derived-artifact receipt v0.1

**Status:** executable output-provenance receipt; not a digital signature or safety certificate  
**Date:** 2026-11-15  
**Related:** build receipt PR #5262; human projections PR #5231; change-impact PR #5218

## 1. Purpose

A verified graph is not enough if the diagrams, labels, circuit directory, resident guide or emergency map can silently drift away from it. Every generated artefact should be linked to the exact build and graph revision that produced it.

The DerivedArtifactReceipt binds:
- artifact identity and revision;
- artifact kind and output format;
- exact generator identity/version, source digest and generator-configuration digest; the source compiler version and mapping-profile version remain separate and are used by change-impact compatibility checks; the source build's compiler version remains a separate field;
- complete declared dependency ID set and dependency-inventory completeness state;
- source build-lock digest;
- source graph ID, revision and declared content digest;
- computed ServiceGraph payload digest;
- exact output-byte SHA-256 digest.

The receipt itself has a digest over its declared serialized representation. Candidate output bytes can be checked against the recorded output digest.

## 2. Supported artifact kinds

v0.1 includes:
- ResidentView
- OperatorView
- QualifiedPractitionerView
- EmergencyResponderView
- SingleLineDiagram
- CircuitDirectory
- SourceMap
- CommissioningDossier
- InterchangeExport
- Other

The receipt does not validate the safety, engineering correctness or completeness of the content. It only binds the bytes to the stated build and generator metadata.

## 3. Lifecycle and invalidation

The output receipt should be a derived node in the dependency graph. Its dependencies include:
- source IR document and revision;
- mapping profile and exact module/configuration identities;
- compiled ServiceGraph payload and maturity;
- standards applicability profiles, amendments and correction labels;
- compiler source, build configuration, toolchain and dependency lock;
- generator version/source/configuration and the exact render inputs.

A mutation in any dependency should invalidate affected artefact receipts and derived documents. Dependency IDs name stable semantic inputs; digests/versions are the values expected by `InputChange`, not embedded in the dependency ID. The previous receipt remains historical; the newly generated output gets a new artifact revision. Do not overwrite the old receipt or claim that an obsolete diagram is current.

The receipt now converts directly into the existing #5218 DerivedArtifact node, using the source build's compiler version and mapping-profile version for the invalidation engine's version checks. Renderer version, renderer source and renderer configuration use stable dependency IDs with separate mutable fingerprints. Build-lock and graph-payload IDs are stable per graph; their content digests are the mutable fingerprints. The receipt preserves dependency IDs and inventory completeness. Its helper emits fingerprints for mandatory build-lock, graph-payload and renderer dependencies; callers append fingerprints for module, standards, evidence and source dependencies.

## 4. Claim ceiling

A matching output digest proves that the checked bytes match the receipt's recorded output digest. The source build-lock reference proves that the receipt names a particular build. Neither establishes reviewer authority, source authenticity, truthful measurements, physical installation state, safety, regulatory compliance or a Certificate of Compliance. Cryptographic signatures and qualified review attestations remain a separate trust layer.
