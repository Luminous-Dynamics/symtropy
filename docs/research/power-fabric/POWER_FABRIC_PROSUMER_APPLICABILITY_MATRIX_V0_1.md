# Power Fabric prosumer-installation applicability matrix v0.1

**Status:** theme-level research map only; no clause-level rules, SANS conformity or installation approval  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5234  
**Related:** #3400, #3418, #3752, #5158, #5195, #5215

## 1. Why this profile now matters

IEC 60364-8-82:2022 + AMD1:2026 has a direct scope relationship to Power Fabric's first real-world target: a low-voltage prosumer installation that may operate connected to a distribution network, with local generation and/or storage, and in islanded or stand-alone modes. The consolidated edition describes local-source/storage energy flows, grid interaction, islanded/stand-alone aspects, clarified earthing-system transition sequencing and DSO connection/disconnection conditions.

South Africa's installation standard reference also changed after the original Power Fabric research snapshot. ECA(SA) announced availability of SANS 10142-1:2026 Ed 3.03 on 2026-06-26 and identifies it as the current new edition superseding the previous 2024 Ed 3.2 reference. This public notification is not a substitute for the authoritative SABS normative text.

Sources:
- IEC 60364-8-82:2022 + AMD1:2026 consolidated edition 1.1: https://webstore.iec.ch/en/publication/113148
- IEC 60364-8-82:2022/AMD1:2026 record: https://webstore.iec.ch/en/publication/96654
- ECA(SA) SANS 10142-1:2026 Ed 3.03 availability notice (2026-06-26): https://ecasa.co.za/technical/notification-of-the-availability-of-sans-10142-12026-ed-3-03/
- South African Department of Employment and Labour CoC notice (2025-10-22): https://www.gov.za/news/media-statements/employment-and-labour-electrical-installation-laws-and-certificate-compliance

## 2. Scope boundary

This matrix identifies themes that Power Fabric should represent and trace to evidence. It does not translate normative clauses into rules. Clause-level implementation requires access to the current licensed standards, full edition/amendment history, applicable regulations and utility requirements, followed by independent review by competent registered South African electrical practitioners.

The two profiles are distinct:
- IEC 60364-8-82:2022 + AMD1:2026 is an international published standard profile.
- SANS 10142-1:2026 Ed 3.03 is a South African national installation profile.

A successful IEC semantic mapping cannot establish SANS compliance. The SANS edition, Electrical Installation Regulations, utility/grid requirements, site context, registered-person responsibilities and Certificate of Compliance remain separate.

## 3. Theme-level applicability candidates

| Theme | Power Fabric representation | Evidence dependency | Claim ceiling |
|---|---|---|---|
| Grid-connected mode | Explicit operating-mode claim and PCC relationship | Exact installation topology, grid-interface identity, competent verification | Does not prove permission to connect |
| Islanded mode | Separate mode claim and declared local source/storage paths | Exact inverter/grid-forming mode, transition design and test evidence | Does not prove stable island operation |
| Stand-alone mode | Separate mode from islanded mode | Source, storage, operating envelope and intended-service evidence | Does not imply a grid-forming capability |
| Local generation | Source nodes, exact asset IDs and declared operating envelopes | Manufacturer/model identity, ratings and applicable verification | Does not prove generation is currently available |
| Local storage | Storage asset, charge/discharge service and constraints | Exact configuration, BMS/firmware, usable energy and test evidence | Does not infer backup runtime or black-start |
| Grid connection/disconnection | Transition record, grid boundary, preconditions and authority/evidence references | Applicable DSO requirements, equipment evidence, competent approval | No software-issued grid switching authorization |
| Earthing arrangement | Namespaced system/earthing designation and scope | Installation-specific assessment and test evidence | No inferred earthing correctness |
| Earthing transition sequencing | From/to mode, before/after earthing claims, transition evidence and unresolved conditions | Exact normative clauses, installation drawings, protection design and qualified review | No inference that the transition is safe or compliant |
| Protective measures | Explicit relationship to protection/isolation model and applicable studies | Exact product/installation data and qualified verification | No protection coordination calculation in this profile |
| Verification and handover | Evidence ledger, test records, revision binding and CoC reference | Competent inspection/testing and statutory process | Power Fabric does not issue CoCs |

## 4. Existing programme ownership

Do not create a second electrical solver or duplicate these owners:

- #3400 owns the system digital-twin and energy/service model.
- #3418 owns resilience, grid-forming, islanding, black-start, AC/DC and protection qualification research.
- #3752 owns PCC metrology and measurement evidence.
- #5158 owns the human-facing design and misuse-resistant explanation.
- #5195 owns standards interoperability profiles.
- #5215 owns adapter conformance and semantic round-trip contracts.
- #5234 owns the prosumer standards applicability mapping and cross-owner integration requirements.

Power Fabric may link to evidence from those owners, but it must not convert a model result into independent authorization.

## 5. Frozen synthetic negative controls

The companion fixture includes scenarios where:
1. grid supply is absent but local storage/generation may remain an available source;
2. the operating mode is unknown and earthing transition evidence is unresolved;
3. a planned islanding transition is not equivalent to an observed/verified mode transition;
4. a grid disconnect command is not proof of completed physical disconnection;
5. a field observation is stale or bound to a previous inverter configuration;
6. an IEC profile is present but the South African jurisdiction profile is unresolved.

Expected behavior is fail-closed for the affected qualification claim, with the source record and uncertainty preserved. None of these scenarios is an electrical design or switching instruction.

## 6. Acceptance targets

- Exact IEC edition/amendment and SANS edition are pinned in separate profiles.
- Grid-connected, islanded and stand-alone remain separate concepts.
- Every transition is a versioned record with explicit before/after claims and evidence references.
- Unknown or stale mode, earthing, protection, grid-interface or approval facts prevent the affected qualification claim from being reported complete.
- No generated output implies automatic DSO permission, safe switching, absence of voltage, safe-to-touch status, protection adequacy, SANS conformity or CoC issuance.
- All mappings report unsupported/ambiguous semantics through the adapter ledger.
