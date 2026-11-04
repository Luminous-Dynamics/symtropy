# Robotic Masonry — Accepted Output Adapter V0.1

**Issue:** [#3629 — robotic wall quality and accepted-area benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3629)  
**Generic contract:** [#3605 — accepted output and total-workflow productivity](https://github.com/Luminous-Dynamics/symtropy/issues/3605)  
**Profile:** `profile/masonry-wall-accepted-output-v0.1.json`  
**Status:** task-adapter proposal; no physical masonry work has been benchmarked by Symtropy.

## Purpose

The masonry task is not "blocks laid per hour". Its functional unit is a **finally accepted, specification-compliant wall segment** or a declared square metre of accepted wall, with block format, wall assembly, openings, corners, bond/pattern, adhesive/mortar and downstream interface frozen in advance.

Block/hour remains a machine-cycle metric. It does not by itself establish accepted wall area, structural suitability, first-pass quality, complete interface details or total workflow productivity.

## Industry comparator: FBR Hadrian

### H04 factory acceptance test

FBR's FY2026 Annual Report (released 30 September 2026) reports that Hadrian H04 completed a 751-block factory acceptance test in October 2025, achieving an effective lay rate exceeding 285 blocks/hour and approximately 36 m² of wall/hour using large-format Wienerberger Porotherm blocks.

This is a specific FAT result on a specific block system, not a field deployment result or independent competitive comparison.

Source: https://investors.fbr.com.au/announcements/xzoJwm-fbr-annual-report-2026

### Manufacturer specifications and current marketing claims

FBR's Hadrian product page lists:
- design maximum up to 360 blocks/hour;
- block envelope up to 600 × 400 × 300 mm and 45 kg;
- 32 m boom reach;
- a two-person operating crew: Hadrian operator and telehandler operator;
- operating range 0–45°C, light rain, and wind peak gust up to 60 km/h;
- current published base price A$7.8 million, subject to final specification and options.

These are vendor-declared capabilities and price indications, not an independently observed task result or TCO.

Source: https://www.fbr.com.au/view/hadrian

FBR's FAQ claims over 300 blocks/hour and over 70 m² wall/hour with its largest-format blocks. That claim has a different block geometry from the 751-block Porotherm FAT; do not pool or relabel the claims as the same achieved output.

Source: https://www.fbr.com.au/view/faqs/faqs

The benchmark therefore retains three separate evidence items: (1) H04 factory-tested Porotherm throughput, (2) design maximum specifications and (3) current manufacturer FAQ claims. None is an independent field comparison.

## Acceptance predicate

The project's structural engineer, architect/specifier and competent inspection authority must provide the frozen wall-specific acceptance criteria. This adapter does not invent structural tolerances or building-code requirements.

Bind:
- design/model revision, wall-segment identity, site epoch, frame and work-package;
- block type, dimensions, mass, material, batch and permitted substitutions;
- bond/pattern, course, alignment, orientation, corners/returns, openings, lintels, reinforcement, ties and service penetrations;
- adhesive/mortar product, batch, mixing/application process, joint geometry, open time and cure conditions;
- robot model/serial, software/firmware, gripper/end effector, boom/stabilisation configuration and telehandler identity;
- independent as-built line, level, plumb, course height, wall-plane/straightness, joint and feature-completeness measurements with uncertainty;
- project-required material, bond/adhesion or structural tests;
- defects, missing/incorrect units, damage, cracks, adhesive failures, removals, repairs and final disposition.

Machine vision and the robot's own placement counters are not independent acceptance evidence. Required structural, geometry and material tests are determined by the project specification and jurisdiction.

## Full workflow accounting

Keep robot-hour, direct crew-hour and total workflow crew-hour separate. Include BIM/model preparation, design revisions, trade coordination, block procurement and staging, telehandler work, robot calibration and stationing, operator and spotter, adhesive/mortar mixing/application/cure, cuts and special pieces, lintels/rebar/ties, inspection, testing, rework, removal, reinstatement, weather waits, maintenance and closeout.

Report:
- block placements/hour by declared block class;
- finally accepted wall-segment count and accepted wall m²;
- accepted wall m² per robot-hour, direct crew-hour and total workflow crew-hour;
- first-pass and final acceptance with exact denominators;
- line/level/plumb/plane/joint error distributions by course and stratum;
- critical detail omissions and defects by severity;
- material/adhesive waste, rework and removal;
- elapsed time per finally accepted wall segment;
- cost per accepted wall area only when labour, equipment/capital, service, consumables and material assumptions are independently documented.

Never compare block/hour from one format with wall-m²/hour from another without the exact conversion, wall assembly and acceptance rules. A machine may lay high block counts while producing a poor accepted wall or leaving manual work for the corners, openings and reinforcement.

## Matched benchmark and ODD

Prefer randomized or balanced within-site comparisons using identical wall geometry, materials, design revision, acceptance predicate, inspection and trade scope. Otherwise disclose counterfactual method and confounding.

Stratify by block dimensions/mass/material/batch, wall assembly and thickness, wall height/reach, bond pattern, straight/curved geometry, corners, openings, lintels and reinforcement, adhesive/mortar and cure conditions, foundation level, weather, wind, temperature, precipitation, site access, material staging, telehandler support and crew ratio.

A factory test is not a field ODD. A single block format does not qualify the full product envelope. Cross-site transfer requires a held-out site, block/material system, serial unit and crew/configuration replication.

## Safety separation

Performance does not qualify long-boom stability, payload retention, block-drop hazard control, telehandler interaction, wind limits, exclusion zones, worker/plant separation, access, emergency stops or recovery. These remain separate physical safety/ODD claims. A higher lay rate never compensates for a failed safety condition.

## Ownership

- #3605: generic accepted-output ledger and workflow accounting.
- #3629: masonry functional unit, acceptance adapter and matched baseline.
- #3591: cross-laboratory measurement reproducibility.
- #3521/#3522: physical trial receipts and raw evidence references.
- #1607: survey control, frame, calibration and uncertainty.
- #3505/#3506 and construction safety owners: separate physical safety/ODD qualification.
- #3531 and Symthaea #4918: claim-specific admissibility and common-mode independence.

This adapter does not implement a structural solver, building-code authority, robot controller, survey engine, productivity framework or safety certification.
