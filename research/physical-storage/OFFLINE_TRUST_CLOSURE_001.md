# Offline Cold-Start and Trust-Closure Benchmark — v0.1

**Programme:** OFFLINE-TRUST-CLOSURE-001  
**Tracking:** [Symtropy #7636](https://github.com/Luminous-Dynamics/symtropy/issues/7636)  
**Capsule parent:** [Symtropy #7571](https://github.com/Luminous-Dynamics/symtropy/issues/7571)  
**Service workloads:** [Symtropy #7582](https://github.com/Luminous-Dynamics/symtropy/issues/7582)  
**Independent decoder scoring:** [Symthaea #13734](https://github.com/Luminous-Dynamics/symthaea/issues/13734)  
**Common-cause assurance:** [Mycelix #5762](https://github.com/Luminous-Dynamics/mycelix/issues/5762)

## 1. Research question

Can an independent team start from a clean, network-isolated machine and only the declared Recovery Capsule plus explicitly provisioned external trust material, then bootstrap a reader, recover exact bytes, verify fixity and provenance, satisfy machine-checkable Preservation Objectives, establish runtime dependency closure, and produce a reproducible evidence receipt?

This is a qualification benchmark, not a new archive container, signature scheme, operating system, sandbox or preservation standard.

## 2. Paradigm shift: from self-describing archive to trust-closed recovery

A self-describing archive can still fail:
- the archive may carry its own untrusted key and self-consistent replacement payload/manifest;
- the executable may depend on an undeclared library, font, plugin, system resource or package cache;
- two reader binaries may share source, libraries, compiler, schema parser, calibration or key store;
- an archive can be byte-perfect while its schema, units, renderer or representation information are absent;
- old signature evidence can establish a historical state without establishing current revocation status.

Keep four axes separate:

| Axis | Question | Independent authority |
|---|---|---|
| Bitstream integrity | Do recovered bytes match the expected digest? | Scorer-held digest, not candidate-supplied digest |
| Authenticity/provenance | Is content accepted under an independently provisioned trust policy? | Out-of-band trust root and preserved validation evidence |
| Interpretability | Can independent tools satisfy declared Preservation Objectives? | Machine-checkable objective corpus and independent outputs |
| Operational closure | Can runtime work offline without undeclared dependencies/capability violations? | Clean harness, network denial, dependency trace and sandbox telemetry |

A pass in one axis cannot compensate for failure in another. Known bad evidence is fail; missing/unresolved evidence is incomplete; an unrun test is not evaluated.

## 3. Standards composition and industry baseline

The benchmark profiles established standards instead of creating a competing package format.

### OAIS / ISO 14721:2025

OAIS remains the reference information model for AIP/DIP, representation information, preservation planning and independent understandability. Edition 3 adds Preservation Objectives to make independent understandability more testable, and introduces Preservation Watch. OCSTC operationalizes this by requiring objective IDs, expected semantic outputs, units and acceptance rules. A descriptive statement that a file is understandable is not a result.

### BagIt / RFC 8493

BagIt provides hierarchical package structure, payload/tag manifests and fixity metadata. Its RFC explicitly limits the security claim: manifest fixity is not designed to secure against active attacks, and signatures are outside its scope. Use BagIt for packaging/fixity where appropriate, but never interpret an in-package digest as an independent trust anchor. Verify provenance/signatures and external expected digests separately.

### OCFL 1.1

OCFL provides application-independent object/version layout, inventory and digest rules, completeness and storage diversity. It helps preserve reconstructible object history and co-locates content and metadata. It does not itself provide a reader executable, independent trust root, clean offline runtime or successful cold-start demonstration.

### PREMIS 3.0

PREMIS supplies preservation objects, events, agents, rights and environment descriptions. Use it to preserve software/hardware environment, agent identity and event lineage. A PREMIS record documents an environment; this benchmark tests whether that environment can actually be reconstructed and executed.

### NARA Digital Preservation Risk Matrix

NARA's matrix provides a screening baseline for format disclosure, adoption, transparency, self-documentation, external hardware/software dependencies, renderers and plugins. OCSTC converts these risks into adversarial removal and cold-start cases. A low declared dependency risk is not enough; the harness must observe runtime dependency closure.

### Competitive position

The proposed differentiation is not replacing BagIt, OCFL, PREMIS or OAIS. It is testing whether the information package, runtime, trust material and representation information together support independently verified, offline, exact and understandable recovery. Existing standards give strong package/object/metadata foundations; this benchmark adds reproducible adversarial qualification across them.

## 4. Threat model and independence boundaries

The harness controls a clean microVM or similarly isolated environment, immutable inputs/fault schedule, deny-by-default network policy, empty package caches, no mounted user home, independent expected digest/trust-root material, expected Preservation Objectives, file/process/device/sandbox telemetry and independent reader comparison.

The candidate receives only the allowed package, published format specification, explicitly provisioned trust anchors and declared dependencies. It must not receive scorer-only original bytes or expected outputs except as part of public known-answer tests.

### Trust-root rules

A key embedded only in the candidate archive cannot authenticate that archive. The harness provisions accepted trust roots through a separately administered path, such as an offline public-key bundle, separately stored recovery sheet, distinct offline medium or threshold-custodian package. The receipt records root identity, source, policy version, acquisition path and freshness evidence. Out-of-band does not automatically mean trusted: custody, policy and key status still require qualification.

For long-term signatures, distinguish historical validation at ingest from present-day authenticity/trust status. Preserve validation material and contemporaneous revocation/timestamp evidence. If offline evidence cannot establish freshness, current trust status is unknown/incomplete. An expired certificate or stale revocation list must not be interpreted without policy support.

### Reader independence

Different language, executable name, vendor or source hash is insufficient. Record shared source/generated code, libraries/codecs/schema parsers/compilers, build toolchains/base runtime, OS services/locale/font databases/plugins, calibration references/physical reader components, keys/trust stores/cloud KMS/registries and human approval steps.

Mycelix #5762 remains the common-cause/dependency authority. Symthaea compares decoded bytes and semantic outputs. Symtropy owns capsule and physical reader evidence.

## 5. Frozen synthetic case corpus

| Case | Perturbation | Expected |
|---|---|---|
| Trusted clean cold start | Two independent readers, out-of-band digest+trust root, full objectives, no network/deps | Pass |
| Missing external trust root | Payload digest available; authenticity unresolved | Incomplete |
| Archive-only self-signed key | Candidate archive supplies its own key but claims authenticity pass | Fail |
| Payload+manifest replacement | Attacker replaces bytes and in-package manifest/key together | Fail |
| Hidden runtime dependency | Runtime uses undeclared library/resource | Fail |
| Network fallback | Reader tries registry, remote KMS, CDN, DNS or telemetry | Fail |
| Reader divergence | Independent outputs disagree | Fail |
| Representation-information loss | Bytes recover but schema/units/objective cannot be checked | Incomplete |
| Unknown revocation freshness | Historical signature evidence exists; current status unknown | Incomplete |
| Sandbox escape | Reader violates file/process/device/network boundary | Fail |
| Wrong-but-valid format | Syntactically valid descriptor has wrong version, units, map or code parameters | Fail |
| Single reader | Only one implementation executed | Incomplete |
| Not run | All dimensions remain not evaluated | Not evaluated |

The fixture is deterministic and synthetic. Case counts are not probability estimates.

## 6. Scoring contract

The dependency-free Rust reference in offline-trust-closure-reference scores harness-produced receipts. It does not inspect a VM, verify hashes/signatures, enforce sandboxing or prove reader independence. Those are external evidence-producing functions.

The scorer enforces:
- non-empty scenario and reader identifiers; no duplicate reader identifiers;
- impossible Preservation Objective counts rejected;
- any measured failure, undeclared dependency, network attempt or sandbox violation => fail;
- integrity cannot claim pass when expected digest came only from archive;
- authenticity cannot claim pass when trust root came only from archive;
- interpretability cannot claim pass with zero or partially satisfied objectives;
- fewer than two readers or zero objectives => incomplete;
- unknown/unrun dimensions never become pass;
- full pass requires every evidence axis and independence/agreement checks to pass, out-of-band digest and trust, all objectives, zero undeclared dependencies, zero network calls and zero sandbox violations.

Pass statuses must come from harness observations, not candidate assertions. Future versions should bind every evidence statement to a digest-addressed artifact, tool identity, source/build provenance and exact trial head.

## 7. Metrics — vector, not weighted score

Correctness: exact byte recovery/full archive acceptance; false integrity/authenticity/semantic acceptance; independent reader agreement; Preservation Objective pass rate.

Operational closure: cold-start success; declared vs observed runtime dependencies; attempted/successful external network calls; sandbox violations and undeclared process/device access.

Resources: time to first verified byte and full restore time; CPU, peak RAM, energy and bootstrap package bytes; reader/decoder size and dependency footprint.

Preservation: representation-information completeness; historical/current signature status separately; reproducible-build rate/source provenance; migration byte fixity, semantic fidelity and preservation-event completeness.

Do not average away a hard security, authenticity or exact-recovery failure. Publish every case result, denominator, exclusion and unknown.

## 8. Prospective acceptance targets

These are research targets, not achieved results:
- 100% exact byte recovery for positive cases within declared ECC limits;
- zero false acceptance across all negative controls;
- zero undeclared runtime dependencies in a cold offline run;
- zero successful network dependencies; every attempted request appears in evidence and is policy-scored;
- two independent readers produce byte-identical payloads and agree on every declared machine-checkable Preservation Objective;
- unknown trust/revocation/representation evidence never becomes pass;
- clean rebuild and execution do not require package-registry access;
- each accepted output includes scorer-held digest evidence, external trust-policy decision, dependency closure and complete trial receipt.

## 9. Qualification ladder

- T0 — contract: case/schema/receipt semantics and scorer negative controls.
- T1 — synthetic runner: clean microVM, deny-all network, empty caches, deterministic test packages.
- T2 — independent implementation: second scorer reproduces every receipt classification from frozen fixtures.
- T3 — blind reader: external team receives package and out-of-band trust material, original vendor runtime withheld.
- T4 — physical read chain: calibrated observations → independent readers → exact payload digest and Preservation Objectives.
- T5 — migration: restore and migrate to another medium; retain fixity, semantics, trust and preservation-event lineage.

A stage cannot be skipped by a higher headline result. Physical results require specimen, reader, calibration and batch provenance.

## 10. Evidence boundary and current status

The v0.1 Rust scorer and synthetic corpus are authored research artifacts. Rust unit tests are authored but have not been run in this execution environment because Cargo, rustc and rustfmt are unavailable. No isolated microVM runner, actual sandbox test, external trust-root ceremony, independent second reader or physical-media trial has been run. No formal JSON Schema 2020-12 validator result is claimed unless recorded separately.

A schema-valid fixture is not an offline recovery pass; a container image is not proof of dependency independence; a matching in-package digest is not an external trust anchor; a signature does not prove that signed bytes are semantically correct.

## References

- ISO 14721:2025, OAIS Edition 3: https://www.iso.org/standard/87471.html
- CCSDS 650.0-M-3: https://public.ccsds.org/Pubs/650x0m3.pdf
- RFC 8493, BagIt v1.0: https://www.rfc-editor.org/rfc/rfc8493
- OCFL 1.1: https://ocfl.io/1.1/spec/
- PREMIS 3.0: https://www.loc.gov/standards/premis/v3/
- NARA Digital Preservation Risk Matrix: https://github.com/usnationalarchives/digital-preservation/tree/master/Digital_Preservation_Risk_Matrix
- University of Michigan Digital Preservation Baseline: https://deepblue.lib.umich.edu/handle/2027.42/179514
