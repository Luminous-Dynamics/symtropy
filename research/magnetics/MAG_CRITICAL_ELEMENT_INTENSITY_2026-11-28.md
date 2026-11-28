# Critical-element intensity per accepted service protocol

**Programme:** MAG-CRITICAL-ELEMENT-INTENSITY-001  
**Issue:** [Symtropy #7912](https://github.com/Luminous-Dynamics/symtropy/issues/7912)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Decision question

Does an AM/hybrid architecture deliver the same or better accepted useful shaft service as the strongest relevant conventional comparators while consuming less virgin mass of one or more preregistered critical elements per MWh, without quietly increasing another element's intensity?

The decision is a vector across elements, not a scalar criticality score.

## Why magnet volume is insufficient

Magnet mass or volume does not reveal the element composition of a given grade or lot. Nd, Pr, Dy and Tb can have materially different proportions; Co, Cu and other materials may change elsewhere in the bill of materials. A magnet-lean design may shift material and power requirements into the stator, inverter or cooling system. Replacement events can erase a first-life material reduction.

All claims therefore bind actual measured material mass and element-specific assay intervals to a complete frozen BOM/inventory scope.

## System and service boundary

Freeze application/target function, ordered mission profile, stator/rotor/shaft interfaces, inverter/DC supply, control, cooling and auxiliaries, measurement setup, acceptance policy, complete element-inventory/BOM scope and service horizon. Candidate internals may differ inside the frozen external contract. A changed application duty, inverter/DC supply, control, cooling boundary, measurement setup or service horizon is NotComparable unless preregistered as a separate comparison class.

## Element inventory

For every physical material lot:
1. identify component and material family;
2. bind lot origin (Virgin, ExternalSecondary, InternalRecycled or DirectReuse);
3. measure actual material mass in grams;
4. measure each preregistered element mass fraction using a traceable physical method;
5. bind laboratory and method scope, method, calibration, uncertainty and instrument receipts;
6. bind the lot to the complete material inventory scope digest.

A non-detect is not zero: the measured interval must include the applicable detection limit. Missing element assays and unregistered assay evidence never default to zero.

The evaluator computes each element's mass by interval multiplication of material mass and mass fraction, then aggregates by origin. It reports virgin, external secondary, internal recycled, direct reuse and total embodied mass separately.

No end-of-life credit is subtracted from virgin input in v0.1. Cut-off, avoided burden, substitution and other LCA allocation methods are not silently combined. End-of-life recovery is reported as its own measured stream and only credited in a separate lifecycle method with a declared allocation policy.

## Useful-service denominator

Useful service is accepted shaft energy across a declared observed/durability-tested horizon: bounded useful shaft kWh per mission; integer count of completed missions; total completed useful shaft kWh; physical service receipt; separate durability test receipt, method, instrument, calibration and uncertainty.

A modelled lifetime or validated projection cannot be used to declare observed lifetime intensity. Short exploratory tests can be reported as short-horizon g/MWh, but must not be renamed lifetime intensity.

## Metric and decision

For each element e:

    virgin_intensity_e = virgin_element_mass_e_g / accepted_shaft_service_MWh

Also report external-secondary, internal-recycled, direct-reuse and total embodied g/MWh for the same element. Report cost, drive energy, carbon/LCA and supply risk separately.

A CriticalMaterialAdvantage requires M4 physical component acceptance for candidate and comparator; exact scope, service and inventory/BOM matching; physical element assays and mass evidence for all lots; observed or durability-tested physical service, not projection; complete registered element coverage and optimization receipts at matched budget tier; conservative non-inferiority on accepted service and every required element intensity; and strict uncertainty-separated improvement in at least one predeclared element intensity against every required baseline.

An increase in Dy, Tb or Co beyond its predeclared margin blocks a Nd/Pr improvement claim. No universal weighted sum or green badge is produced.

## Industry comparator interpretation

Devito et al. (2026), *Design and Optimization of Electric Traction Motors with Reduced Dependence on Rare Earth Materials*, Energies 19(18), 4316, DOI 10.3390/en19184316, reviews application-specific design and test comparisons. One equal-rated 60 kW light-vehicle case reports IPM/WFSM/SynRM drive-cycle energy at 93.70/94.24/94.86 Wh/km (about 1.3% spread at the study boundary). One PM-assisted SynRM retrofit case reports 26% lower rare-earth magnet volume without performance loss. Another hypercar FEA/design comparison reports about 45% magnet-volume reduction with a small drive-cycle penalty; this is a design study, not physical qualification. A heavy-duty case reports a different WFSM/IPM ranking under its own mission. None is a universal performance or critical-element intensity target; preserve original system boundary, application and evidence class.

## Validation and claim ceiling

Synthetic known-answer fixtures must validate origin segregation, vector comparisons, uncertainty, mass-fraction closure, scope and no-double-counting. The v0.1 fixture is synthetic throughout and is expected to remain Indeterminate. No physical material reduction, lifetime service, commercial savings, carbon advantage, critical-material security or standards conformity is claimed.
