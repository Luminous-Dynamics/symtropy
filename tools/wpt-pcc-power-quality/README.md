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


## IEC 61000-3-12: scope versus limit domain

The evaluator keeps three distinct outcomes:

- `NotApplicable`: outside the equipment/public-LV/current-range scope or the boundary is wrong.
- `ScopeOnlyLimitDomainNotCovered`: the equipment is in the broad scope (>16 A and <=75 A/phase; single phase up to 240 V or three phase up to 690 V; public low voltage), but the declared nominal system is not the limit system stated for this edition.
- `ApplicableScopeOnly`: broad scope and the stated 230 V single-phase / 400 V three-phase, 50 Hz limit domain match. This remains scope classification only; no limit comparison is performed.

This distinction follows the IEC catalogue text: although the broad scope describes 50/60 Hz and voltage ranges, the limits in this edition are stated for 230/400 V, 50 Hz systems. A 60 Hz or 480 V case must not silently inherit that limit table.

## Why power class does not determine IEC current applicability

For a balanced three-phase system:

`I = P / (sqrt(3) * V_LL * PF * eta)`.

At 400 V and ideal PF=eta=1, illustrative input currents are:
- 6 kW: 8.66 A/phase;
- 11 kW: 15.88 A/phase;
- 22 kW: 31.75 A/phase;
- 30 kW: 43.30 A/phase.

Actual current increases as power factor and conversion efficiency fall below one. These arithmetic examples do not establish IEC 61000-3-12 applicability for a device: measured/rated AC input current per phase, public-LV connection, voltage, frequency and intended connection are required. In particular, an 11 kW product may fall on either side of 16 A depending on actual input current and operating conditions.


## Declared harmonic coverage

The synthetic PCC fixture declares complete harmonic coverage through order 50. Each integer harmonic order 2..50 is present exactly once; orders with no synthetic harmonic energy have an explicit zero RMS amplitude and a non-zero uncertainty field. The evaluator returns `maximum_harmonic_order`, `thd_through_order_percent` and, for an applicable IEEE 519-2022 PCC profile, `tdd_through_order_percent`. It rejects missing harmonic bins and refuses an IEEE 519 TDD result if the run declares less than 50th-order coverage.

## IEC 61000-3-12 limit-domain distinction

The broad equipment/public-LV scope (greater than 16 A and up to 75 A per phase; voltage up to 240 V single-phase or 690 V three-phase; 50/60 Hz) is not identical to the stated limit domain of this edition. The IEC catalogue states that the limits in IEC 61000-3-12:2011+AMD1:2021 apply to 230/400 V, 50 Hz systems. The evaluator therefore returns `ScopeOnlyLimitDomainNotCovered` for an in-scope equipment class connected to 60 Hz or a different nominal system voltage; it does not silently apply 230/400 V, 50 Hz limit tables to those cases.

## Demand-reference provenance

An IEEE 519 TDD report requires (I_L), its definition identity, averaging window, reference period and source digest; (I_{SC}) likewise requires a calculation method and source digest. These identifiers make the benchmark result traceable. They do not prove that the underlying demand study or short-circuit study was technically correct; independent review remains necessary.


## Phase and conductor configuration

The profile declares both phase count and wire count. The validator permits 1-phase 2/3-wire and 3-phase 3/4-wire combinations, and rejects inconsistent declarations before applying a standards profile. This prevents an incomplete connection description from being treated as a valid applicability result.
