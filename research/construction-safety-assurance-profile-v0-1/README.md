# Construction Robotics Safety Applicability Profile v0.1

**Status:** standards-scope triage only. This is not a conformity assessment, legal opinion, safety case, or permission to operate.

## Why this profile exists

Construction robots rarely fit one neat equipment category. A system may combine an industrial manipulator, a mobile base, earth-moving plant, lifting equipment, worksite infrastructure, autonomy software, human access and an underground operating environment. Applying a generic “industrial robot” label can hide standards exclusions and leave the mobile machine or lifting function outside the chosen safety framework.

The machine-class hypotheses and standards ledger in `standards/standard-applicability-v0.1.json` therefore record:
- the published edition and status checked on 15 November 2026;
- a short public-scope summary and material scope exclusions;
- a default status of `REQUIRES_EXPERT_REVIEW` until the exact machine, application, site and jurisdiction are known;
- what the Luminous evidence system can preserve (configuration identity, hazard/control references, validation records and change lineage);
- an explicit statement of what software evidence does **not** establish.

The schema is at `schema/standard-applicability-v0.1.schema.json`.

## Main applicability distinctions

- **Fixed industrial robot application/cell:** ISO 10218-1:2025 and ISO 10218-2:2025 are starting points for the robot and its application respectively, alongside general risk assessment and safety-control-system engineering.
- **Mobile manipulator / AMR / AGV:** evaluate the mobile base under ISO 3691-4:2023 when its industrial-truck definitions apply. Do not assume the mounted manipulator standard automatically covers the combined mobile system.
- **Autonomous earth-moving / mobile mining machine:** ISO 17757:2019 is directly relevant where the equipment fits its earth-moving or mobile-mining definitions, including surface and underground operation.
- **Cranes / forklifts / lifting plant:** ISO 10218-2 expressly excludes multi-purpose lifting devices or machinery such as cranes and forklift trucks. The applicable equipment-specific and local legal requirements must be independently identified; this profile intentionally does not pretend that the robot-cell standard fills the gap.
- **Specialist underground robot:** ISO 17757 may be relevant only when machine definitions and system scope fit. An inspection robot or tunnel service platform is not automatically an earth-moving or mining machine.

ISO 12100 provides general machinery risk-assessment/risk-reduction principles. ISO 13849-1 concerns the design and integration of safety-related control-system parts. ISA/IEC 62443 addresses industrial automation/control cybersecurity; it complements rather than replaces physical and functional safety.

## Evidence architecture

For every deployment, build a site- and machine-specific record that joins:

`machine class → application boundary → lifecycle hazards → required risk controls → safety functions → validation evidence → residual-risk acceptance`

The evidence layer should carry exact design, robot, tool, payload, controller, software, firmware, calibration, site epoch, operating-zone and task identities. It should retain:
- hazard and foreseeable-misuse analysis references;
- safety-function specification and required performance rationale from competent safety engineers;
- operating-zone preparation, access-control and environmental surveys;
- stopping/braking, protective sensing and fail-safe test receipts;
- software/firmware change and cybersecurity records;
- independent test method, instruments, calibration, raw readings and acceptance authority;
- unresolved hazards, exclusions and deviations.

Symtropy can make these records traceable and replayable. It must not decide a required Performance Level, declare a machine safe, approve a lift, or mark a standard as conformed based on metadata or software tests.

## Status vocabulary

Allowed per-standard applicability states:
- `APPLICABLE`
- `PARTIALLY_APPLICABLE`
- `OUT_OF_SCOPE`
- `REQUIRES_EXPERT_REVIEW`
- `NOT_ASSESSED`

A new project starts at `REQUIRES_EXPERT_REVIEW` or `NOT_ASSESSED`. No generic `COMPLIANT` status exists in this schema.

## Sources and limits

The data was derived from public ISO catalogue/scope pages and the ISA/IEC 62443 series overview, checked on 15 November 2026. It does not reproduce normative clauses. The complete purchased standards, national adoptions, applicable legislation, manufacturer instructions and the exact deployment risk assessment must be reviewed by competent professionals.

Machine-class examples in this profile are prompts for scope review, not final legal determinations. In particular, the draft third editions of ISO 3691-4 and ISO 12100 are tracked separately and are not treated as published normative editions.
