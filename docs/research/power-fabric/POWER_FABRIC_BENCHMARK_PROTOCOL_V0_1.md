# Power Fabric reproducible benchmark protocol v0.1

**Status:** benchmark contract and frozen scenario registry; seven machine-readable scenario families plus comparative performance and lifecycle-economics study dimensions; no comparative performance or human-subject outcome claimed  
**Date:** 2026-11-15  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5241  
**Parent:** standards adapter PR #5229

## 1. Positioning against the state of the industry

The benchmark must start from current products and standards, not an invented strawman.

| Ecosystem | Existing capability to acknowledge | Power Fabric comparison hypothesis |
|---|---|---|
| ETAP 2026 | Physics-based digital twin for design/operation, real-time feeder visualization, switching optimization, equipment library at industrial scale, protection/arc-flash and power-system studies | Do not replace ETAP solvers. Measure provenance and lifecycle integration of its outputs, identity preservation, revision-impact coverage, and portable human projections |
| SPAN | Circuit-level monitoring/control, backup prioritization, and an authenticated local-only web app for live power, circuit relay toggles, priority changes and a grid-status override | Do not claim local control or backup priorities as novel. Test independent as-built graph, cross-vendor composition, evidence maturity, and configuration drift beyond one panel ecosystem |
| EPLAN Electric P8 2026 | Electrical engineering, enclosure design, ERP/PLM/PDM links, bidirectional neutral exchange and downstream lifecycle data | Do not claim lifecycle engineering or PLM integration as novel. Test evidence lineage and change impact across heterogenous products and systems |
| Schneider EcoStruxure Power | Digital power-distribution architectures and connected building-power solutions | Treat open data/connectivity and digital building architecture as incumbents; compete on evidence quality and lifecycle replay, not dashboards |
| OpenFMB | Peer-to-peer interoperability reference architecture using CIM and IEC 61850-derived semantic models | Do not claim to invent semantic peer-to-peer interoperability. Measure loss-aware mapping receipts, lifecycle revision provenance, cross-profile evidence, and human comprehension |
| NIST test-programme model | Standards-based interoperability profiles, test programmes, certification and stakeholder processes | Build reproducible, narrowly scoped profiles and public negative controls; seek independent test-authority review |

Sources checked 2026-11-15:
- ETAP 2026 product features and 23,000+ model library: https://etap.com/product-releases/etap-2026-release
- ETAP release list showing 2026.0.1N on 2026-11-11: https://etap.com/products/whats-new
- SPAN Home On-Premise (official page updated 2026-11-13): https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- EPLAN Electric P8 2026 performance description (status 08/2025): https://www.eplan.com/content/dam/eplan/corporate/performance-descriptions/2026/en/performance-description-eplan-electric-p8.pdf
- OpenFMB: https://openfmb.openenergysolutions.com/docs/openfmb/intro/
- Schneider EcoStruxure Power: https://www.se.com/za/en/work/campaign/innovation/power-distribution/
- NIST Smart Grid Testing and Certification project (status Completed, updated 2025-03-26): https://www.nist.gov/programs-projects/smart-grid-testing-and-certification

The likely differentiation is not a better panel, another digital twin, 3D electrical design, PLM integration, a monitoring dashboard or another universal electrical ontology. It is a **verifiable lifecycle compilation layer** joining existing domain models, exact module identities, evidence/authority boundaries, configuration changes and role-specific outputs.

NIST's Smart Grid Testing and Certification project page reports Project Status Completed (updated 2025-03-26). Its published approach remains a useful method reference for interoperability profiles, test programmes, procurement language and certification authorities, but this benchmark must not describe that particular NIST project as currently active. The IEC Webstore labels IEC 60364-8-82:2022+AMD1:2026 CSV CORRECTED VERSION 2026-11 while retaining publication edition 1.1 and publication date 2026-04-16; correction metadata is a separate build-lock input. South Africa's ECA(SA) announces SANS 10142-1:2026 Ed 3.03 availability.

Sources: https://webstore.iec.ch/en/publication/113148 ; https://ecasa.co.za/technical/notification-of-the-availability-of-sans-10142-12026-ed-3-03/.

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

### I. Integration effort and lifecycle economics
Compare matched tasks with manual/vendor-specific integration workflows. Measure engineer hours, adapter creation/maintenance, unresolved semantics, manual reconciliation, stale drawing discovery, rework, and change propagation time.

Candidate hypothesis: reduce measured integration/reconciliation effort by 30% without increasing critical error rate. Preregister before observing results; this is a target to test, not an achieved outcome.



### G. Reproducible build receipts
For each frozen IR/graph fixture:
- Pin compiler source digest, compiler version, toolchain, build configuration digest, Cargo.lock digest, mapping profile, module hardware/configuration identities, standard profile editions/amendments/correction labels and upstream inputs.
- Rebuild the same graph twice under identical inputs and compare source/profile/graph/build-lock digests.
- Mutate exactly one input at a time (compiler source, build configuration, dependency lock, standard correction, module identity) and assert the affected digest changes while unrelated digests remain stable.
- Verify the exact serialized receipt bytes and reject altered fields, truncation, appended data or noncanonical serialization.

Targets: 100% reproducibility on frozen inputs; 100% detection of each declared single-input mutation; zero silently missing locked inputs. These targets establish software build reproducibility, not equipment authenticity or safety.

### H. Derived artifact integrity
For every view, drawing, circuit directory, emergency map, dossier and interchange export:
- Bind artifact identity/revision/kind/format, generator ID/version, source build-lock digest, source graph revision/payload digest and output byte digest.
- Verify exact output bytes; altered bytes must be rejected.
- Change build lock, graph revision, module identity, standard edition/correction or generator version independently and check that the derived receipt changes.
- Connect receipt dependencies to the change-impact DAG; a mutation marks all dependent outputs stale until regenerated.

Targets: 100% derived-output digest match for unmodified bytes; 100% stale marking recall across the frozen dependency oracle; zero stale output retained as current.


Only the seven correctness-oriented scenario families (IR contract, adapter, change impact, resilience, human projection, build receipt and derived artifact) are represented as individual JSON cases in the v0.1 machine corpus. Performance scaling and integration/lifecycle economics are separate comparative study dimensions: they require benchmark hardware, matched incumbent workflows and/or participant/practitioner data, not just synthetic JSON fixtures.

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
