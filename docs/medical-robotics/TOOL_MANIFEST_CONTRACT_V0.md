# Medical Robot Tool Manifest Contract v0.1

**Claim ceiling:** validates synthetic tool-design proposals for simulation eligibility only. It does not generate certified CAD, prove physical strength, prove cleanability/sterility, validate clinical performance, or authorize a physical build.

## Required manifest fields
- Identity: `schema_version`, `tool_id`, `revision`, `tool_kind`, `intended_task`, `contact_class`.
- Stage/evidence: `design_stage=concept`, `evidence_class=synthetic_design_proposal`.
- `mechanical`: mount interface, mass, payload, bounding box, contact force/torque proposal, fail-closed retention.
- `hygiene`: material declaration and cleaning claim state. V0 requires `not_validated`; validation claims must not appear without a later evidence review.
- `electrical`: powered, maximum power, protective-isolation design intent.
- `privacy`: audio/video capture, physical shutter, default capture state.
- `authority`: `simulation_only=true`, and physical actuation, patient contact, clinical use all false.

Unknown fields are rejected. This is intentional: a newly added field must not silently introduce a new capability.

## Supported v0 task bindings
- `sealed_supply_gripper -> sealed_supply_transfer`
- `tray_carrier -> tray_transfer`
- `telepresence_head -> telepresence_support`
- `trainer_probe -> training_mannequin_measurement`

Patient-transfer, medication-dispensation, invasive tools, and patient-contact authority are blocked.

## Tool gate dispositions
- `SIMULATION_ONLY`: the manifest is complete and within the synthetic contract. This only allows modeling/testing in the software simulator.
- `BLOCKED`: the manifest is malformed, outside v0 scope, over the declared synthetic envelope, or tries to claim unsupported validation/authority.

A passing gate never implies `HARDWARE_VERIFIED`, `CLEANING_VALIDATED`, `CLINICALLY_VALIDATED`, or `REGULATORY_APPROVED`. Those values are not producible in v0.
