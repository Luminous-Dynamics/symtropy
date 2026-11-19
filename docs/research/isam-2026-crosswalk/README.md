# NASA 2026 ISAM capability crosswalk — seed v0.1

This directory begins the source-grounded taxonomy layer for Symtropy issue #6167.

## Primary source

NASA Langley Research Center et al., In-space Servicing, Assembly, and Manufacturing (ISAM) State of Play, 2026 Edition.

- Official report: https://assets.science.nasa.gov/content/dam/science/missions/isam/ISAM_State_of_Play_2026.pdf
- NASA ISAM overview: https://www.nasa.gov/isam/
- Printed pp. 1–3: functional and cross-cutting capability definitions.
- Printed p. 20: Figure 3 area counts and the statement that individual activities/technologies may be assigned to one or more capability areas.
- Printed p. 24: RPO, proximity operations, capture, docking and mating definitions.
- Printed p. 32: fluid-transfer maturity distinctions.

## Count discipline

The seed records NASA's 11 Figure 3 functional-area counts. Those values sum to **523 category assignments**. That is not a unique-project count: NASA explicitly allows one activity or technology to be assigned to multiple areas. Do not use 523 as the denominator for unique technology coverage, and do not import unsupported headline totals.

The cross-cutting areas (Software and Algorithms; Logistics and Operations; Laws, Policies, and Standards) are maintained separately from the 11 functional areas. They must not be added to the Figure 3 functional-area total.

## What this seed does and does not contain

The JSON is a compact taxonomy and source-locator manifest. It is not yet a full extraction of the report's individual activity entries, a claim that Luminous has demonstrated any capability, or a market/technology ranking.

The next extraction increment must preserve original source-entry identity, multi-area membership, publisher/developer, evidence status, source date and locator. If a source entry has no stable printed identifier, assign a local extraction key while preserving enough page/table text to reproduce it. Never merge entries solely because titles look similar.

## Evidence-state firewall

A local project match should be assigned one of these states only with a cited artifact and a declared context of use:

- no_public_match
- concept_or_design
- software_or_synthetic_benchmark
- analytical_or_external_model_checked
- ground_hardware_demonstrated
- environmental_or_analogue_demonstrated
- in_space_demonstrated
- operational_service
- independently_qualified_for_declared_use

not_assessed is also valid and is not equivalent to no_public_match. A code file, issue title, model output, or passing synthetic test is never hardware or flight evidence.

## Capture vocabulary

Keep the report's physical distinctions intact:

- rendezvous: orbital manoeuvres reduce separation;
- proximity operations: relative distance and/or orientation is managed;
- capture: a physical connection is established, often by a robotic manipulator;
- docking: a rigid connection without use of a robotic arm;
- mating: a general physical connection between objects or interfaces.

A relative-motion terminal state must not be mapped to capture or docking without the additional evidence.

## Reproducibility

The JSON contains source metadata and extracted aggregate values, not a copy of the NASA report. When the individual-entry extraction is added, publish its transformation script, source digest, extraction coverage and an explicit list of omissions/ambiguous rows.
