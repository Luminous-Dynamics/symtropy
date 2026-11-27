# Physical Recovery Capsule — v0.1

**Programme:** PHYSICAL-RECOVERY-CAPSULE-001  
**Tracking:** [Symtropy #7571](https://github.com/Luminous-Dynamics/symtropy/issues/7571)  
**Compiler:** [Symtropy #7482](https://github.com/Luminous-Dynamics/symtropy/issues/7482)  
**Physical observations:** [Optical #7534](https://github.com/Luminous-Dynamics/symtropy/issues/7534), [DNA #7567](https://github.com/Luminous-Dynamics/symtropy/issues/7567)

## 1. Thesis

A storage medium is not archival if future operators cannot identify its representation, reconstruct its information and verify the original bytes without the original vendor's software or service.

The Physical Recovery Capsule is a qualification profile for packaging the information needed to recover and verify an archive. It is not a new generic archive file format. It composes existing preservation standards:
- **BagIt RFC 8493 v1.0** for filesystem payload/tag organization and payload/tag fixity manifests.
- **OCFL 1.1.1** for application-independent object/version layout and rebuildability.
- **PREMIS 3.0** for preservation objects, events, agents, rights and software/hardware environment information.
- **E-ARK CSIP/AIP 2.2.0** for interoperable information-package structure and provenance metadata.
- **OAIS AIP/DIP** concepts for long-term preservation and future dissemination.

The profile adds physical-medium representation information, reader input contracts, decoder implementation records, known-answer vectors and a blind reader-replacement challenge.

## 2. Why compose rather than replace

BagIt supplies a known package and checksum convention; it does not describe the meaning of the payload, physical medium or decoder. OCFL provides a transparent versioned storage layout and supports rebuildability of objects from stored files. PREMIS captures preservation events and agents. E-ARK AIP builds interoperable structures around METS and PREMIS, including package provenance and migration lineage.

The capsule profile binds these existing pieces to a medium-specific recovery contract. It must not fork BagIt manifests, invent a replacement object layout or claim that a hash proves data origin.

References:
- BagIt RFC 8493: https://www.rfc-editor.org/rfc/rfc8493.html
- OCFL 1.1.1: https://ocfl.io/1.1/spec/
- PREMIS 3.0: https://www.loc.gov/standards/premis/v3/
- E-ARK AIP 2.2.0: https://aip.openpreservation.org/
- E-ARK CSIP 2.2.0: https://www.dilcis.eu/specifications/common-specification

## 3. Capsule layout profile

An illustrative BagIt-composed profile:

    capsule/
      bagit.txt
      bag-info.txt
      manifest-sha256.txt
      tagmanifest-sha256.txt
      capsule/
        recovery_capsule.json
        format_spec.json
        recovery_procedure.md
        known_answer.json
        decoder-a/
          source/...
          build...
        decoder-b/
          source/...
          build...
        schemas/...
        calibration/...
        preservation/PREMIS.xml
      data/
        payload/...
        raw-observations/...
        physical-layout/...
        original-manifests/...

The root descriptor must be represented in BagIt tag files and covered by the tag manifest. All payload files must be listed in the payload manifest. A production capsule can also be placed within an OCFL object/version; the OCFL inventory is separate from BagIt payload/tag manifests.

## 4. Representation and recovery information

The capsule descriptor identifies:
- medium family and format/version;
- logical data model, symbol alphabet, field and code parameters;
- physical geometry and logical-to-physical mapping;
- calibration/reference pattern locations and recovery semantics;
- raw reader observation schema and coordinate frame;
- decoder sources, licenses, source digests, versions, dependency locks and reproducible build steps;
- known-answer test vectors and deterministic fault cases;
- full recovery procedure and bootstrap dependencies.

Never rely only on a binary executable. Preserve the format specification, source, known-answer vectors and toolchain/dependency details. When a compiler or operating environment is needed, record it and test a reproducible build. A container image alone is not a future-proof bootstrap.

## 5. Independent decoder and reader contract

Separate the recovery pipeline:

physical sensor → raw observations → calibrated symbols/likelihoods → error correction → payload reassembly → external fixity verification → authenticity validation.

Symtropy owns the physical reader/channel contract and calibration. Symthaea owns the code/decoder comparison and exact payload recovery. The Physical Recovery Capsule records these interfaces and their versions.

A pair of decoders is not independent merely because it uses a different language, filename, executable hash or vendor label. The profile records shared source modules, libraries, compilers, model weights, firmware, calibration, cloud services, keys and human authorities. Independence must be assessed from the dependency graph; Mycelix's existing transitive/common-cause profile remains authoritative for operational recovery paths.

## 6. Blind Reader Replacement Challenge

The qualification test is performed by an independent team:
1. Provide the media, recovery capsule, open format specification and known-answer vectors.
2. Withhold the original vendor reader application and undocumented decoder.
3. Disable network access for the recovery exercise.
4. Permit an independent reader adapter to emit the documented raw observation contract.
5. Build and run two independently developed decoder implementations.
6. Require both to produce exactly the same byte sequence and match the separately preserved expected digest.
7. Inject corruption into the payload, descriptor, calibration, code parameters, signatures and decoder build. Verify that unsupported or inconsistent state fails closed.
8. Migrate to another medium using a separately documented pipeline. Retain original payload, new representation, fixity, provenance and migration event.

A pass requires exact full-payload equality, independently validated source/build identities, transparent shared dependencies and no reliance on the withheld vendor runtime or network. It does not by itself prove centuries of retention or independent physical failure domains.

## 7. Integrity, authenticity and authority

Keep these separate:
- **Fixity:** cryptographic checksums detect accidental changes.
- **Authenticity:** signatures and provenance link a package to an asserted producer/authority under a trust policy.
- **Recovery correctness:** byte-exact reconstruction is checked against a preserved expected digest.
- **Reader independence:** independently built readers/decoders are evaluated against the same known-answer corpus.
- **Failure-domain independence:** site/power/network/operator/key/software dependencies are assessed through Mycelix's canonical dependency profile.

A checksum is not a signature. A signature does not prove that the data remains uncorrupted. Both manifest and signature algorithms need an explicit crypto-agility/migration policy for long-retention service.

## 8. Qualification states

- **fixture_only:** synthetic example; never evidence of a physical recovery.
- **unqualified:** format/capsule exists, but a required measurement or test is missing.
- **in_review:** the physical and independent implementation evidence is under assessment.
- **qualified:** physical measurement evidence exists; two independently implemented decoders/read paths pass the blind challenge; exact payload equality is independently verified; signature evidence and dependency qualifications meet the service contract.

The schema makes qualified status conditional on physical measurement evidence, qualified decoder independence, passed offline reader replacement and signed provenance. A semantic validator must additionally resolve references, verify BagIt, check source digests/signatures, validate path safety and cross-check PREMIS/OCFL records.

## 9. Benchmark metrics

Report:
- payload files, byte counts and fixity coverage;
- format/representation information completeness;
- number of independently implemented decoders/readers and common dependencies;
- successful offline bootstrap and reproduction time;
- exact payload recovery for both decoder paths;
- negative-control rejection;
- reader replacement success/failure and migration fidelity;
- recovery latency, throughput, cost and energy at a declared full-system boundary;
- external dependencies, missing metadata and unsupported regimes.

## 10. Evidence boundary

The initial schema and sample bag are synthetic. They do not demonstrate a physical reader, real decoder independence, retention, manufacturing yield, authenticity or production archive recovery. A BagIt-valid package is a packaging/fixity result only. Full Reader Replacement qualification requires independent builds and physical media observations.
