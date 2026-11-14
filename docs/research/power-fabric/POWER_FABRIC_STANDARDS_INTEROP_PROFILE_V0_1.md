# Power Fabric Standards Interoperability Profile v0.1

**Status:** research interoperability profile; no claim of standards conformity or product certification  
**Date:** 2026-11-14  
**Owner issue:** [POWER-FABRIC-STANDARDS-INTEROP-001 #5195](https://github.com/Luminous-Dynamics/symtropy/issues/5195)  
**Related work:** [module contract #5159](https://github.com/Luminous-Dynamics/symtropy/issues/5159), [human interaction #5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158), [service graph #5183](https://github.com/Luminous-Dynamics/symtropy/issues/5183)

## 1. Position

Power Fabric must not invent a competing electrical object model or reference-designation system where established standards already exist.

The proposed architecture is a **standards-aligned semantic overlay and evidence compiler**:

- existing standards retain authority over their specific information models and communication/configuration scopes;
- Power Fabric retains a canonical, typed, evidence-bound graph for the system it is modelling;
- versioned adapters map only the semantics that actually correspond;
- unsupported, ambiguous or unknown source fields are preserved or explicitly reported as unmapped;
- human-facing views are derived from the same graph revision and stable object designations.

A successful transform is not an electrical installation approval, standards conformity assessment, utility acceptance or permission to energize.

## 2. Standards landscape and version discipline

This is a public-source snapshot, not a complete normative applicability determination. Before implementing a normative mapping, verify the exact authoritative publication, amendment, corrigendum, national adoption, effective status and installation scope.

### 2.1 Object structuring and reference designations

**IEC 81346-1:2022** provides general structuring principles and reference designation rules for systems and information about systems. It introduces an information model for reference designation and relationships.

**ISO 81346-10:2022** supplements those principles for power supply systems and provides rules/classes for structuring objects in this domain. The ISO catalogue currently shows a second-edition Working Draft under development; that draft must not be represented as a published standard.

Power Fabric implication:
- use stable reference designations where the installation/owner applies the 81346 scheme;
- preserve the site's existing designation system and namespace;
- bind physical labels and circuit directories to the object ID and graph revision;
- do not mint official-looking 81346 designations without applying the standard's actual rules and owner policy.

Sources:
- IEC 81346-1:2022: https://webstore.iec.ch/en/publication/64021
- ISO 81346-10:2022: https://www.iso.org/standard/75471.html
- ISO/WD 81346-10 (edition 2, working draft): https://www.iso.org/standard/93711.html

### 2.2 Utility and distribution network information models

**IEC 61970-301:2020 + AMD1:2022** defines the Common Information Model (CIM) base for power-system network objects. The published edition information lists edition 7.1 as valid.

**IEC 61968-11:2013** provides distribution extensions to CIM, including distribution-network equipment and concepts for single-line diagram exchange.

Power Fabric implication:
- use CIM mappings for the utility/distribution-network layer where the object and relationship semantics genuinely match;
- preserve original CIM mRID/identifiers, namespaces, profile versions and source provenance;
- do not force household-specific installation, human-authority, module maintenance or evidence semantics into a utility model class that does not represent them;
- use extension mechanisms and explicit unmapped fields rather than lossy coercion.

Sources:
- IEC 61970-301:2020 + AMD1:2022: https://webstore.iec.ch/en/publication/62698
- IEC 61968-11:2013: https://webstore.iec.ch/en/publication/6199

### 2.3 IED and substation configuration exchange

**IEC 61850-6:2009 + AMD1:2018 + AMD2:2024** specifies the System Configuration description Language (SCL) for describing IED configurations and parameters, communications systems, switchyard structures and their relationships. Its purpose includes exchanging IED capability and system descriptions between engineering tools from different manufacturers.

**IEC TR 61850-1-1:2026** is the current introductory overview of the series and describes namespace concepts and the power utility automation context.

Power Fabric implication:
- provide an SCL adapter for the appropriate IED/substation scope;
- do not treat SCL as a general household electrical installation schema;
- retain exact IED identity, logical node/configuration, namespace and source file digest;
- distinguish an SCL-described capability/configuration from a measured or independently tested physical capability.

Sources:
- IEC 61850-6 consolidated version: https://webstore.iec.ch/en/publication/103863
- IEC TR 61850-1-1:2026: https://webstore.iec.ch/en/publication/105222

### 2.4 Cybersecurity for power-system communications

**IEC 62351** is a series of standards and technical specifications covering security for power-system management information exchange and communications. Parts have different publication years, scopes and applicability. The 2026 series package includes parts from multiple editions, not one monolithic 2026 protocol.

Power Fabric implication:
- map each communications profile to the applicable IEC 62351 part and version;
- model authentication, identity, authorization, freshness, replay resistance, revocation and audit as separate properties;
- never infer a secure connection merely because a protocol name or IEC 62351 reference is present;
- keep local electrical protection independent of cloud, supervisory compute and Symthaea.

Source:
- IEC 62351 series catalogue (2026 package): https://webstore.iec.ch/en/publication/6912


### 2.5 Prosumer installations and South African applicability

**IEC 60364-8-82:2022 + AMD1:2026** is a high-priority profile for Power Fabric's PV, battery and grid-connected residential/industrial use cases. The consolidated edition 1.1 covers prosumer low-voltage installations with local generation and/or storage, whether connected or not to a distribution network. Its scope includes grid interaction and islanded/stand-alone aspects; the 2026 amendment clarifies earthing-system transition sequencing and conditions for connection/disconnection from the distribution network, and adds requirements including a normative annex for single-dwelling or similar islandable PEIs.

Power Fabric implication:
- represent GridConnected, Islanded and Standalone as distinct operating-mode claims;
- keep local generation, local storage, grid boundary and mode-transition records explicit;
- preserve the earthing arrangement before and after a mode transition, with the required source-specific evidence and unresolved state;
- represent DSO connection/disconnection preconditions as qualified external requirements, not software actuation authority;
- do not infer black-start, grid-forming, islanding stability, correct earthing or permission to reconnect from a generic inverter or battery label.

**South African jurisdiction profile update:** ECA(SA) announced availability of **SANS 10142-1:2026 Ed 3.03** on 2026-06-26, superseding the previously referenced 2024 Ed 3.2 snapshot. The current normative text, amendments, Electrical Installation Regulations and utility-specific requirements must be verified from authoritative sources and reviewed by competent registered South African electrical practitioners before clause-level rules are encoded. The ECA availability notice is not itself the normative standard.

Power Fabric implication:
- maintain SANS 10142-1:2026 Ed 3.03 as a separate South African jurisdiction profile;
- never infer SANS conformity from an IEC 60364-8-82 mapping result;
- require professional inspection, tests and applicable CoC evidence to be supplied from the statutory process;
- do not issue or automate a CoC.

Sources:
- IEC 60364-8-82:2022 + AMD1:2026 consolidated edition: https://webstore.iec.ch/en/publication/113148
- IEC 60364-8-82:2022/AMD1:2026 amendment record: https://webstore.iec.ch/en/publication/96654
- IEC 60364-1:2025: https://webstore.iec.ch/en/publication/63699
- ECA(SA) SANS 10142-1:2026 Ed 3.03 availability notice, 2026-06-26: https://ecasa.co.za/technical/notification-of-the-availability-of-sans-10142-12026-ed-3-03/
- South African Department of Employment and Labour CoC notice, 2025-10-22: https://www.gov.za/news/media-statements/employment-and-labour-electrical-installation-laws-and-certificate-compliance

## 3. Canonical mapping architecture

```
Vendor / utility / engineering documents
      |          |           |
      v          v           v
 IEC 81346     CIM       IEC 61850 SCL
  IDs       network model   IED config
      \          |           /
       \         |          /
        v        v         v
    Versioned, loss-aware adapters
                 |
                 v
      Power Fabric Service Graph
        + Module Contract
        + Evidence / provenance
        + Standards applicability
                 |
         +-------+--------+
         |       |        |
         v       v        v
      Human    Engineer  Export /
      views    views     round-trip
```

The canonical graph is not a replacement standard. It is the internal composition and evidence layer between standards-specific models and the human/operator lifecycle.

## 4. Mapping principles

### 4.1 Preserve original identity

For every imported object, retain:
- source standard/profile identifier and version;
- source document/file identity and digest;
- original reference designation or mRID;
- source namespace and extension payload;
- adapter version and mapping profile version;
- canonical Power Fabric node/edge ID;
- mapping confidence and any ambiguity.

Never replace a source identity with a newly invented ID without preserving the relationship.

### 4.2 Map relations semantically, not by name

A Power Fabric PowerPath is a directed declared possible energy-delivery relationship. A CIM connectivity association, IEC 61850 logical association or a physical cable/reference designation may be relevant but is not automatically identical.

Every mapping rule must say:
- source class/property/relation;
- destination class/property/relation;
- transformation or derivation;
- applicability conditions;
- evidence required;
- known information loss;
- reverse-mapping behavior.

### 4.3 Preserve unknowns and extensions

Every adapter must retain an opaque source fragment or loss record for unsupported extensions. Unknown objects remain unknown. A transform cannot silently discard a vendor-specific field, source namespace, protection relation, module identifier or evidence object and then claim a lossless round trip.

### 4.4 Edition-aware operation

Mapping profiles must pin:
- standard and part;
- edition/amendment/corrigendum;
- publication status (published, technical report, working draft);
- jurisdiction/national adoption if relevant;
- adapter schema version;
- tested source/target implementation versions.

A new edition must not silently change the interpretation of an existing design. Migrations should produce a diff, compatibility result, unresolved mappings and a new graph revision.

## 5. Proposed mapping boundaries

| Power Fabric concept | Standards alignment | Important limitation |
|---|---|---|
| Node/reference designation | IEC 81346-1 and ISO 81346-10 | The site's designation practice and exact rule set remain authoritative |
| Utility/distribution topology | IEC 61970 CIM and IEC 61968 distribution extensions | Does not replace building-level wiring, safety, user or service semantics |
| IED identity/configuration | IEC 61850 SCL | Only the appropriate substation/IED scope |
| Power-system communications security | Applicable IEC 62351 parts | Reference alone does not prove an implementation is secure |
| Module electrical/thermal/control ports | Power Fabric Module Contract | New Luminous contract; must interoperate with source standards without pretending to be an IEC standard |
| Human roles, safe actions and explanations | Power Fabric human views | Human-computer interface, not an alternate safety procedure |
| Prosumer operating modes and transitions | IEC 60364-8-82:2022 + AMD1:2026 | Mode claims, earthing transitions and DSO interfaces need exact jurisdictional and professional evidence; no autonomous switching |
| Evidence, module revision and as-built change impact | Power Fabric evidence layer | Must preserve source records and not issue certification |

## 6. Loss-aware adapter result

Every import/export returns one of:
- ExactMapping;
- MappedWithDeclaredDerivation;
- PartiallyMappedWithLossReport;
- UnsupportedSourceSemantics;
- AmbiguousMapping;
- StaleOrUnknownStandardEdition;
- InvalidInput.

The receipt includes:
- source and target format/profile identities;
- exact standard edition and adapter version;
- source and output digests;
- mapped object/relationship counts;
- unmapped/ambiguous objects;
- preserved opaque extensions;
- derived values and their assumptions;
- evidence and provenance links;
- round-trip comparison result.

A partially mapped result can be useful, but it must not be called lossless or complete.

## 7. Benchmarks

### A. Identity preservation
Measure exact preservation of object identifiers, namespaces, reference designations and module identity across import/export.

### B. Semantic preservation
Compare nodes, edges and relationship meaning before and after transformation. Do not score only XML/JSON byte similarity when formats legitimately reorder or normalize data.

### C. Unknown-extension preservation
Inject unknown vendor extensions, unsupported classes and newer edition fields. Verify that they are preserved or explicitly reported—not silently dropped.

### D. Edition migration
Compare old/new mapping profiles using frozen versioned fixtures. Detect changes in class definitions, relation semantics, profile constraints and unsupported features.

### E. Cross-vendor interoperability
Use rights-cleared SCL/CIM fixtures and, where available, vendor/utility exports. Measure exchange success, semantic loss, manual repair effort and configuration drift. Do not claim interoperability based on parsing one hand-authored fixture.

### F. Human-facing consistency
Generate labels, single-line diagrams and source maps from the canonical graph. Compare object identity and revision against the imported designation and physical as-built evidence.

### Hard gates
- Zero silent loss of in-scope fields or relations.
- Zero unreported dropping of unknown extensions.
- Zero standard-edition coercion or draft-to-published promotion.
- Zero source-identity replacement without an explicit cross-reference.
- Zero “standards compliant”, “installation approved” or “safe” claims from successful serialization alone.

## 8. Implementation sequence

1. Maintain the dated standards applicability registry with editions/status and official source URLs.
2. Add IEC 81346 reference-designation binding to ServiceNode and the human-facing graph.
3. Prioritize a theme-level prosumer profile for IEC 60364-8-82:2022 + AMD1:2026 and the separate SANS 10142-1:2026 Ed 3.03 jurisdiction profile; obtain normative texts and professional review before clause rules.
4. Freeze a narrow CIM distribution fixture profile and map only in-scope source/bus/feeder/switching/measurement objects.
5. Freeze a narrow IEC 61850 SCL IED-configuration fixture and map IED identity/configuration relationships.
6. Add explicit loss-aware import/export receipts and opaque extension preservation.
7. Add round-trip, edition mismatch, stale source, ambiguous relation and unsupported extension tests.
8. Obtain independent standards/interoperability review before making claims about conformance or vendor exchange.

## 9. Non-goals

- No attempt to reimplement all of CIM, IEC 61850, IEC 81346 or IEC 62351.
- No claim that this profile is published, endorsed or certified by IEC/ISO.
- No normative installation or protection calculation engine.
- No reverse engineering of proprietary vendor formats without permission.
- No actuation authority and no assumption that successful import/export proves electrical safety.
