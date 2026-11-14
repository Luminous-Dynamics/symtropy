# Power Fabric reproducible benchmark protocol v0.1

**Status:** benchmark contract and frozen scenario registry; no comparative performance or human-subject outcome claimed  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5241  
**Parent:** standards adapter PR #5229

## 1. Positioning against the state of the industry

The benchmark must start from current products and standards, not an invented strawman.

| Ecosystem | Existing capability to acknowledge | Power Fabric comparison hypothesis |
|---|---|---|
| ETAP 2026 | Physics-based digital twin for design/operation, real-time feeder visualization, switching optimization, equipment library at industrial scale, protection/arc-flash and power-system studies | Do not replace ETAP solvers. Measure provenance and lifecycle integration of its outputs, identity preservation, revision-impact coverage, and portable human projections |
| SPAN | Circuit-level monitoring/control, backup prioritization, local authenticated on-premise management without cloud dependence | Do not claim local control or backup priorities as novel. Test independent as-built graph, cross-vendor composition, evidence maturity, and configuration drift outside one panel ecosystem |
| OpenFMB | Peer-to-peer interoperability reference architecture using CIM and IEC 61850-derived semantic models | Do not claim to invent semantic peer-to-peer interoperability. Measure loss-aware mapping receipts, lifecycle revision provenance, cross-profile evidence, and human comprehension |
| NIST test-programme model | Standards-based interoperability profiles, test programmes, certification and stakeholder processes | Build reproducible, narrowly scoped profiles and public negative controls; seek independent test-authority review |

Sources checked 2026-11-14:
- ETAP 2026: https://etap.com/software/whats-new
- SPAN Home On-Premise: https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- OpenFMB: https://openfmb.openenergysolutions.com/docs/openfmb/intro/
- NIST Smart Grid Testing and Certification: https://www.nist.gov/programs-projects/smart-grid-testing-and-certification

The likely differentiation is not a better panel, another digital twin, or another universal electrical ontology. It is a **verifiable lifecycle compilation layer** joining existing domain models, equipment identities, evidence, configuration changes and role-specific views.

## 2. Benchmark families and acceptance

### A. Contract correctness (hard gates)
Frozen adversarial corpus: reject all malformed or contradictory identities, dangling references, unknown claims with forged evidence, observed/verified claims without evidence, unknown standard lifecycle status, unsupported mappings disguised as success, duplicate semantic atoms, stale change receipts, incomplete dependency inventories, and unknown common-cause dependencies.

Target: 100% expected acceptance/rejection across the frozen corpus. Zero silent coercion. These are software contract goals, not electrical safety results.

### B. Standards and semantic round-trip
Use at least two rights-cleared independent source ecosystems when adapters exist (initially CIM distribution and IEC 61850 SCL). Pin exact editions, implementation profiles, source digests and adapter versions. Compare normalized semantic inventories rather than XML/JSON bytes alone.

Targets:
- 100% source inventory accounting, or explicitly incomplete.
- Zero silent loss of in-scope object identities, properties, relationships, units, namespaces or extensions.
- 100% agreement with frozen semantic round-trip oracle.
- Unsupported, ambiguous and explicit-loss mappings never reported as lossless.
- Independent review of mapping semantics and source/target digest computation.

### C. Change impact
For each mutation type, compare incremental invalidation against a separately authored dependency oracle and compare final incremental outputs with a full recomputation.

Targets:
- 100% invalidation recall on the frozen oracle corpus.
- Zero false invalidations outside declared dependency closure.
- 100% semantic equality between incremental and full recomputation.
- Unknown input, incomplete dependency inventory, stale compiler/profile, or unresolved evidence blocks cache reuse.

### D. Resilience
For small graph fixtures, independently enumerate directed edge-disjoint routes, bridges, articulation nodes and shared declared failure domains.

Targets:
- 100% agreement with independent oracle on frozen graphs.
- Unknown/incomplete failure-domain inventory never yields a clean no-shared-dependency statement.
- Shared intermediate nodes are exposed even with parallel edge-disjoint arcs.
- Never infer physical independence, adequate capacity, protection coordination, or reliability probability from topology alone.

### E. Human comprehension and misuse resistance
Preregister matched tasks against conventional paper single-line diagrams, circuit schedules and labels. Report residents, operators, apprentices and qualified electricians separately.

Measures: task correctness, time, severity-weighted errors, confidence calibration, detection of unknown/stale/contradictory data, recognition of all possible sources, correct interpretation of no-path and backup claims, accessibility, language and colour-vision variation.

Proposed targets:
- At least 95% correct on preregistered non-hazardous interpretation tasks.
- Zero critical false-safe interpretations in the frozen scenario suite.
- Demonstrable reduction in severity-weighted critical interpretation errors relative to the paper baseline, with confidence intervals.
These are research targets, not regulatory thresholds or guarantees. Only paper/screen simulations and de-energized mock-ups; no exposure to energized conductors or electrical work.

### F. Compiler performance and reproducibility
Benchmark 100, 1,000, 10,000 and 100,000 graph objects. Disclose hardware, graph shape, fan-in/fan-out, source distributions, compiler and adapter versions, standard profiles, and cold/warm cache conditions.

Report p50/p95:
- import and validation latency;
- graph construction and source-map latency;
- change invalidation latency;
- projection compilation latency;
- peak memory and artifact size.

Do not claim byte-identical canonical output until canonical serialization and cryptographic digest contracts are implemented and qualified. Current deterministic BTreeMap ordering is not RFC 8785 canonical JSON.

### G. Integration effort and lifecycle economics
Compare matched tasks with manual/vendor-specific integration workflows. Measure engineer hours, adapter creation/maintenance, unresolved semantics, manual reconciliation, stale drawing discovery, rework, and change propagation time.

Candidate hypothesis: reduce measured integration/reconciliation effort by 30% without increasing critical error rate. Preregister before observing results; this is a target to test, not an achieved outcome.

## 3. Frozen corpus discipline

Every fixture must record:
- immutable fixture ID and version;
- provenance/rights and source digest;
- exact standard/profile/edition and adapter version where applicable;
- expected output or independent oracle;
- negative controls;
- known exclusions and unresolved semantics.

Do not weaken fixtures after observing candidate failures. If a fixture is wrong, issue a new version and preserve the prior one with a documented correction.

## 4. Publication rules

Publish all runs, including failures and deviations. A report names exact Git commit, build profile, compiler version, hardware, source artifact hashes, schema/standard editions, test command and output digest. CI queued, absent or skipped is neither pass nor fail. Authored tests are not executed tests.

A synthetic fixture pass is not human comprehension, field reliability, electrical safety, standards conformity, utility acceptance, HIL, laboratory or regulatory qualification.
