# WPT PCC power-quality profile evaluator v0

**Status:** standards-aware applicability contract and synthetic TDD oracle  
**Parent:** [#3760 WPT-PCC-POWER-QUALITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/3760)  
**Related:** #3757 dual-frequency spectral evidence; #3715 grid/network stability; #3477 cross-lab metrology.

## Purpose

This crate distinguishes equipment-terminal emissions from site-level point-of-common-coupling (PCC) power quality. It computes THD from the declared fundamental and harmonic RMS lines, and computes total demand distortion (TDD) only when the profile supplies an explicit PCC maximum-demand current (I_L).

TDD is:

`TDD = 100 * sqrt(sum(I_h^2, h >= 2)) / I_L`

It is not THD with a different name. (I_L) is a separately defined maximum-demand-current reference, and must never be silently replaced by the instantaneous fundamental or charger nameplate current.

## Pinned profiles

- IEEE 519-2022: site-PCC system quality profile. The evaluator requires PCC identity, voltage, (I_{SC}), (I_L) and the definition identity of (I_L). It computes TDD and (I_{SC}/I_L), but does not load normative limit tables or declare compliance.
- IEC 61000-3-12:2011+AMD1:2021: product emission profile. The evaluator classifies the broad scope for equipment above 16 A and up to 75 A per phase on applicable public low-voltage systems. An in-scope result is only APPLICABLE_SCOPE_ONLY; there is no threshold comparison or certificate.
- IEC 61000-4-30:2025 COR1:2026: measurement-method-only profile. It is not an emission limit standard.

The code does not embed copyrighted normative harmonic limit tables. Any future comparison must use reviewed/licensed profile data pinned to an edition, jurisdiction and connection type.

## Synthetic known-truth

The fixture has a declared 50 Hz / 400 V three-phase PCC with:
- fundamental current: 100 A;
- harmonic RMS currents: 4, 3, 2, 1 and 0.5 A;
- maximum-demand current (I_L=100) A;
- short-circuit current (I_{SC}=25,000) A.

Hand-calculated:
- THD = 5.5%;
- TDD = 5.5%;
- (I_{SC}/I_L=250).

Changing only (I_L) to 50 A leaves THD at 5.5% but raises TDD to 11%. This is the core negative control proving that THD and TDD are not interchangeable.

All fixture values are synthetic, not a measured installation or a standards-compliance claim.

## Scope and claim limits

- An IEEE 519-2022 result measured at equipment terminals is NOT_APPLICABLE as a PCC assessment.
- IEC 61000-3-12 is not applied outside its declared equipment-current and public-LV scope.
- IEC 61000-4-30 is classified as a measurement method only.
- The evaluator checks profile identity and computes point metrics; it does not compare against normative limit tables.
- It does not replace instrument grouping/aggregation methods, calibration, Class A/S testing, grid-code study or third-party certification.
- No hardware/grid injection, EMC certification or utility connection is authorized.

## Run

`cargo test --locked -p wpt-pcc-power-quality`
