# Kitchen Outcome Accounting V1

Status: draft Rust implementation and synthetic contract fixtures. No physical kitchen run has been measured.

This module is designed for the existing `symtropy-kitchen-planner` crate. It produces a deterministic arithmetic receipt from an explicit, frozen boundary. It does not create a second scheduler, appliance controller or food-safety authority.

## Quantities and denominators

- Mass is converted exactly to integer nanograms from mg/g/kg.
- Water is converted exactly to integer nanolitres from mL/L.
- Electrical energy is converted exactly to integer microwatt-hours from Wh/kWh.
- These dimensions remain separate. Gas, fuel and delivered thermal energy are not included in the electrical total.
- Resource intensity is represented as an exact numerator/accepted-serving denominator. Zero accepted servings produce no per-serving ratio.
- Accepted edible output, trim, reject, residue, drainage and evaporation are distinct output categories. The input-minus-accounted-output residual is reported explicitly.

## Missing data and evidence

Every preregistered flow is observed, explicitly not applicable with a justification, or unknown with a reason. Missing/unknown terms never become zero. Invalid units, duplicate terms, arithmetic overflow and output mass greater than input fail closed.

Measured evidence requires a calibration reference; allocated and attested evidence require a method reference. Declared, modelled and simulated evidence may yield a complete exploratory receipt but never a claim-eligible result.

## Run

`cargo test -p symtropy-kitchen-planner --all-targets`

Exact-head CI remains authoritative. No local Rust test result is claimed by this documentation.

## Claim ceiling

The receipt establishes only deterministic arithmetic and completeness for submitted records. It does not establish that the observations are true, nor food safety, sanitation, nutrition, sensory quality, lifecycle impact, energy savings or product superiority.