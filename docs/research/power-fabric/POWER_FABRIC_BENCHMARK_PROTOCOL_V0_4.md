# Power Fabric benchmark protocol v0.4

**Status:** proposed benchmark protocol; no comparative performance or human-subject result claimed  
**Date:** 2026-11-15

## 1. Benchmark philosophy

Power Fabric must compete on independently testable outcomes, not feature counts or self-reported product checklists. Incumbents already have substantial capabilities in monitoring, electrical engineering analysis, digital twins, utility CIM integration, IED/SCL configuration, and DER interoperability. The research opportunity is a verifiable composition layer: stable identities, traceable claims, loss-aware transforms, reproducible diagnostics, human comprehension, and lifecycle change impact.

Benchmark against frozen inputs and matched baselines. Publish unsuccessful runs, exclusions, hardware, software versions, test profiles, confidence intervals, and limitations.

## 2. Benchmark families

### A. IR contract and hostile-ingestion correctness

Fixtures:
- duplicate object, relationship, claim, or extension IDs;
- dangling relationship endpoints and extension owners;
- unknown claim with an evidence ID;
- observed or verified claim without an evidence ID;
- contradictory claim with zero, one, duplicate, or self-referential conflict IDs;
- malformed, non-finite, and precision-sensitive decimal strings;
- future schema major/minor;
- unrecognized top-level fields;
- namespaced vendor extensions with opaque payloads;
- repeated property claims across distinct claim domains.

Targets:
- 100% expected rejection/acceptance across the frozen contract corpus;
- zero silent coercions;
- exact preservation of extension payload UTF-8 bytes at the model boundary;
- deterministic ordered iteration for identical input structures.

### B. Standards transformation conformance

For each adapter, freeze the standard part, edition, amendment, implementation profile, mapping rules, and source/target fixtures. Test source identity and namespace retention, classes, attributes, units, relationships, unknown and unsupported source content, vendor extensions, round-trip semantics, explicit losses and ambiguities, source and target digests, and migration across profile versions.

Targets:
- 100% preservation of in-scope semantics or explicit machine-readable loss/ambiguity entries;
- zero unreported deletion of in-scope identities, relationships, or extensions;
- zero claims of equivalence where semantic equivalence was not demonstrated.

Textual equality is not the primary metric. Use semantic comparison for formats whose serializers reorder XML, RDF, or attributes.

### C. Incremental compilation and change impact

Use paired full and incremental runs. Mutate one frozen input field at a time, with a precomputed dependency oracle.

Targets:
- 100% of affected derived claims invalidated in the frozen mutation corpus;
- zero unrelated invalidations for changes outside declared dependencies;
- incremental output semantically equivalent to full recomputation;
- deterministic diagnostic codes and ordering;
- report p50/p95 latency and memory at 100, 1,000, 10,000, and 100,000 graph objects.

### D. Resilience and failure-domain analysis

Fixtures include independent alternate paths, edge-disjoint paths with common upstream sources, shared boards/routes/enclosures/communications/cooling/authority dependencies, single points of failure, simultaneous failures, unknown common-cause relationships, and bounded path/cut-set cases.

Targets:
- 100% expected answers on small graph fixtures with independently enumerated oracles;
- never equate graph-theoretic edge-disjointness with physical/common-cause independence;
- unknown dependency produces an explicit incomplete result;
- no reliability probability without supported failure rates and validated assumptions.

### E. Human comprehension and misuse resistance

Pre-register a matched comparison against conventional documentation. Report participant groups separately. Use non-hazardous interpretation tasks for general participants.

Measures: task correctness, time to correct interpretation, confidence calibration, severity-weighted error count, recognition of unknown/unavailable/contradictory information, critical false-safe interpretations, accessibility, and offline usability.

Proposed research target: at least 95% correct on preregistered non-hazardous tasks and zero critical false-safe interpretations in frozen qualification scenarios. These are goals, not regulatory thresholds or safety evidence.

### F. Interoperability integration cost

Compare matched tasks against manual/vendor-specific workflows: engineer hours to map a profile, adapter code and maintenance effort, unmapped semantic count, reconciliation count, round-trip failures, defect discovery time, and time to propagate an equipment revision.

Do not claim superiority until tasks, files, participants, tools, versions, and acceptance criteria are matched.

### H. Scoped source inventory closure

Separate **inventory closure** from **semantic mapping**. The source inventory receipt is scoped to an exact source system/namespace, query/filter digest, included record classes, explicit exclusions, provider snapshot/revision, traversal contract, page artifacts/cursor chain, and as-known-at cutoff.

Freeze page and record oracles. Include immutable exports, stable snapshot pagination, missing first/middle/terminal pages, repeated/skipped cursors, source revision/token drift, same-cursor payload changes, page/record count mismatch, namespaced identity collisions, hidden filters, unknown permissions, explicit exclusions, unsupported-but-accounted records, missing/duplicate dispositions, late pages, best-effort mutable pagination and order-invariant replay.

Required states:
- complete_within_declared_scope;
- incomplete;
- unknown;
- conflicted.

Semantic status is orthogonal: an inventory may close within its declared scope while unsupported or ambiguous source semantics keep the adapter result incomplete. No caller-supplied boolean may confer closure.

Targets:
- zero false-complete results on missing pages, unstable snapshots, hidden filters, unknown permissions or missing terminal boundaries;
- 100% expected closure state and diagnostic agreement;
- 100% page/cursor/record-count reconciliation when those source values are available;
- one and only one semantic disposition per enumerated record;
- 100% as-known-at replay correctness for late page arrivals;
- independent reconstruction of the closure result from exact pinned artifacts.

The source-inventory v0.1 fixture is synthetic and authored, not executed. A complete-within-scope receipt does not establish physical truth or universal source completeness.

### G. Physical equipment identity and temporal reconciliation

Treat the logical design slot, product model, individual physical unit and time-bounded installed assignment as different identity types. Freeze append-only delivery/install/remove/replace/relocate/reconcile events and replay them at explicit as-known-at cutoffs.

Every lifecycle event must first pass the upstream evidence-admission verifier at the frozen decision cutoff. A serial-label observation must carry the observed identity namespace and serial value; exact matching is necessary but not sufficient for authenticity or installation.

Compare:
- model/catalogue matching only;
- supplier/delivery records;
- serial-label observations;
- device-authenticated identity assertions where a credible trust policy exists;
- qualified physical reconciliation and independently scoped review.

Negative controls include label-only identity, duplicate serial within one issuer namespace, wrong product model, disallowed hardware revision, one instance assigned to two active slots, replacement without new reconciliation, late evidence, and conflicting or reordered events.

Targets:
- zero false physical-instance matches in the frozen negative-control corpus;
- zero reconciled installations from design-only, catalogue-only or label-only evidence;
- 100% correct as-known-at replay under late-arriving evidence;
- 100% expected disposition for replacement, relocation, identity collision and hardware-revision mutation;
- 100% rejection/abstention on evidence-admission unknown or rejected and exact serial-namespace mismatch;
- 100% invalidation recall for affected outputs and zero invalidation outside the declared closure in complete fixtures;
- 100% deterministic projection across permutations of the same causally equivalent event set.

The current physical-identity suite is synthetic and authored, not executed. No fixture proves authenticity, installation, electrical safety, standards conformity or statutory approval.

## 3. Industry comparison boundaries

- SPAN materials document circuit-level energy/cost insights, backup priorities, local on-premise access for supported functions, and explicit warnings that software OFF is not a suitable service isolation position. Power Fabric should not claim to invent circuit monitoring, backup prioritization, local access, or the physical-isolation distinction.
- ETAP describes its digital twin as a unified platform to model, design, visualize, analyze, predict, control, and provide insight into electrical power-system management and performance. Power Fabric should not claim to invent electrical digital twins or power-system analysis.
- CIM and IEC 61850 SCL already provide established information models/configuration exchange in their domains. OpenFMB is an existing reference architecture using CIM/IEC 61850-derived data models for peer-to-peer interoperability. Power Fabric should compete by composing and qualifying model outputs, evidence, human views, and lifecycle changes—not by relabeling these capabilities.

## 4. Research success criteria

The strongest result would be a reproducible public benchmark in which at least two independent source systems can be imported into a shared Power Fabric IR, transformed to one or more standards profiles, changed, recompiled, and checked for explicit semantic preservation/loss; an independent verifier reproduces the result; and users complete matched comprehension tasks with fewer or less severe errors than conventional documentation.

A result showing Power Fabric performs worse is still publishable and useful. Do not weaken baselines after observing results.


## 5. Public research references

Sources checked for this snapshot on 2026-11-14:

- ISO 81346-10:2022 publication record: https://www.iso.org/standard/75471.html
- ISO/WD 81346-10 edition 2 lifecycle (working draft, not published): https://www.iso.org/standard/93711.html
- IEC 61850-6 consolidated SCL publication: https://webstore.iec.ch/en/publication/103863
- IEC 61970-301 CIM base: https://webstore.iec.ch/en/publication/62698
- IEC 61968-11 distribution CIM extensions: https://webstore.iec.ch/en/publication/6199
- IEC 62351 2026 series catalogue: https://webstore.iec.ch/en/publication/6912
- IEC 62351-7:2025 network/system management objects: https://webstore.iec.ch/en/publication/76108
- IEC 62933-3-1:2025 EES planning/performance: https://webstore.iec.ch/en/publication/75427
- IEC 62933-5-1:2024 EES safety: https://webstore.iec.ch/en/publication/72239
- IEC 60364-1:2025 low-voltage installation fundamental principles: https://webstore.iec.ch/en/publication/63699
- IEEE 1547-2018 DER interconnection and interoperability: https://standards.ieee.org/ieee/1547/5915/
- OpenFMB reference architecture and CIM/61850-derived model: https://openfmb.openenergysolutions.com/docs/openfmb/intro/
- NIST smart-grid testing and certification programme: https://www.nist.gov/programs-projects/smart-grid-testing-and-certification
- ETAP Digital Twin fact sheet: https://etap.com/docs/default-source/brochures/fact-sheets/digital-twin-fact-sheet-web.pdf
- SPAN Home app update: https://www.span.io/blog/a-new-span-home-app-experience
- SPAN documented on-premise operation: https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- SPAN software OFF service-isolation warning: https://support.span.io/hc/en-us/articles/4411214235159-Navigating-the-SPAN-Home-App
- South African Government standards notice identifying SANS 10142-1:2024 Ed 3.2: https://www.gov.za/sites/default/files/gcis_document/202408/51050gen2686.pdf
- South African Department of Employment and Labour CoC notice (2025-10-22): https://www.gov.za/news/media-statements/employment-and-labour-electrical-installation-laws-and-certificate-compliance

The referenced publications establish scope and publication metadata, not a complete standards-compliance rule set. Standards access, edition, amendment, national adoption, applicability and licensing must be checked before normative rules or conformance claims are implemented.


## 6. Machine-readable corpus v0.2 (2026-11-15)

The frozen v0.1 registry remains unchanged at 36 cases and seven families. The versioned v0.2 registry expands this to **50 synthetic cases across eight families**:

| Family | Cases |
|---|---:|
| IR contract | 5 |
| Standards adapter | 5 |
| Change impact | 5 |
| Resilience | 5 |
| Human projection | 4 |
| Reproducible build receipt | 7 |
| Derived artifact | 5 |
| Physical identity and temporal reconciliation | 14 |
| **Total** | **50** |

The v0.2 machine files are `POWER_FABRIC_BENCHMARK_SCENARIOS_0_2.json` and `POWER_FABRIC_BENCHMARK_SCENARIOS_SCHEMA_0_2.json`. Their measurement status is `scenario_registry_authored_not_executed`. These are synthetic expected-outcome cases, not executed test results, measured performance, independent reproductions, physical observations or safety evidence. The digest strings in physical identity fixtures are explicitly marked as placeholders rather than content commitments to real evidence.

Keep v0.1 immutable for reproducibility. Benchmark reports must name the exact corpus version and digest, not merely say "Power Fabric benchmark corpus".


## 6. Machine-readable corpus v0.3 (2026-11-15)

The v0.2 registry remains frozen at 50 cases across eight families. The additive v0.3 registry contains **74 synthetic cases across ten families**, adding 24 `source_inventory_closure` cases to cover scoped pagination, snapshot continuity, record/disposition accounting, permissions, exclusions and temporal replay.

Files: `POWER_FABRIC_BENCHMARK_SCENARIOS_0_3.json` and `POWER_FABRIC_BENCHMARK_SCENARIOS_SCHEMA_0_3.json`. Measurement status remains `scenario_registry_authored_not_executed`. These cases are expected-outcome oracles, not executed tests, observed provider behaviour or comparative performance results. v0.1, v0.2 remain immutable.
