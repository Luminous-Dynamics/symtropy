# Maritime UKC Physical Benchmark v0.1

**Status:** synthetic contract and benchmark-corpus seed. Not a UKC solver, an S-129 validator, a physical validation result, or navigation authority.

## Purpose and ownership

This package seeds [MARITIME-UKC-PHYSICAL-VALIDATION-001 #4390](https://github.com/Luminous-Dynamics/symtropy/issues/4390).

- S-129 product identity, dataset lifecycle, freshness and whole-dataset replacement belong to [#4388](https://github.com/Luminous-Dynamics/symtropy/issues/4388).
- Ship-handling and manoeuvring envelopes belong to [#3865](https://github.com/Luminous-Dynamics/symtropy/issues/3865).
- Decision-time forecast applicability belongs to [#1650](https://github.com/Luminous-Dynamics/symtropy/issues/1650).
- The end-to-end service outcome belongs to [#3683](https://github.com/Luminous-Dynamics/symtropy/issues/3683).

This package defines an input contract and frozen synthetic adversarial cases. The expected dispositions are **contract-validation outcomes only**. They are not measured physical outcomes, and no expected numeric UKC/squat value is asserted as a reference truth.

## Paradigm: clearance as an uncertainty budget

Move from a single, unqualified squat number to a vessel-, sensor-, route-, location-, time- and regime-specific evidence chain.

A bookkeeping relation for a fully declared reference frame is:

`UKC = referenced\_water\_depth - vessel\_draught - dynamic\_response\_at\_critical\_point`

It is not a universal solver. Every term requires:
- a vertical datum, sign convention, spatial location and time;
- explicit static/dynamic draught and trim conventions;
- a source or model identity, revision and applicability domain;
- measurement/model uncertainty, resolution and any known covariance;
- evidence class: measured, forecast, empirical, simulated or prescribed;
- a double-counting policy for squat, trim and sensor-derived UKC.

Never silently set missing terms to zero, assume independent errors from correlated sources, or transfer midship sinkage to a bow-mounted sensor without a validated trim profile.

## Research anchors and what they actually establish

1. **Ma et al. (2025), Ocean Engineering 323, 120665.** The paper compares self-propelled and towing URANS for a 10,000-DWT ship in an intermediate channel (reported W/B = 2, h/T = 1.45), with physical experiment comparison for resistance and vertical force. The authors report that propeller-related effects materially influence sinkage and trim and that the towing model gives lower-accuracy squat predictions in the studied case. This is a regime-specific finding, not a universal correction factor. DOI: https://doi.org/10.1016/j.oceaneng.2025.120665

2. **Kok et al. (2021), empirical methods vs self-propelled URANS.** The study compares squat predictions against benchmark data for finite-depth, high-speed cases. In its investigated Fr_h >= 0.5 and 1.1 <= h/T <= 1.3 regime, most empirical methods substantially underestimated squat (reported 7–49%); the ranking differed in the very shallow/lower-Froude cases. Do not generalize the 7–49% result outside that study's tested cases. DOI: https://doi.org/10.5750/ijme.v162iA2.1131

3. **MARIN CoVadem formula validation (2019 publication).** The report describes operational inland-vessel measurements and model-test comparisons of Barrass, Ankudinov, Tuck/Stocks and Römisch methods. It emphasizes dynamic trim when UKC sensors are near the bow. The published 2019 snapshot reported more than 80 participating vessels; this is a dated report, not a statement about the current fleet or public data availability. Any raw data use needs separate permission, rights and context-of-use review. https://www.marin.nl/en/publications/validation-of-squat-formulas-for-inland-vessels

4. **Tjeldsundet S-129 commercial-voyage demonstration.** The S-100 Demonstrator report documents a combined S-101/S-102/S-111/S-129 workflow and a route about 65 NM shorter through Tjeldsundet. It explicitly says high-water conditions meant increased navigable space from S-129 was not demonstrated during the voyage; additional cargo was theoretical, not demonstrated. Separate interoperability success, modeled opportunity and observed physical benefit. https://s-100.no/operational-test-s-129-under-keel-clearance-management-tested-in-tjeldsundet-norway/

5. ITTC's benchmark repository is a discovery and reference location; each dataset still needs exact artifact, rights, metadata and context-of-use admission. https://www.ittc.info/benchmark-repository

## Contract v0.1

A case binds:
- exact vessel configuration, length/beam/draught/trim and propulsion mode;
- waterway geometry, horizontal CRS, vertical datum, depth and survey provenance;
- speed-through-water, depth Froude, h/T, W/B, water level/current/density/waves and validity/cutoff;
- sensor position relative to the vessel reference point, vertical datum, measured UKC, calibration, uncertainty and source rights;
- model method, purpose (primary prediction vs comparator), propeller representation, output reference point, trim representation, convergence state, domain, config digest and evaluation split;
- source-lineage groups and whether any independence claim is made;
- unit and component-accounting conventions to detect mismatch and double counting.

The profile is not a complete physical solver input standard. Future revisions may add hull form, loading/LCG/VCG, rudder, propeller curves, bank geometry, channel cross-section, wave response, turbulence/mesh, numerical settings and full raw trajectory/force outputs. Do not add undocumented precision to v0.1.

## Model comparison arms

Implement/reproduce only where the exact definition and applicability are established:
- empirical formulae: Barrass/Barrass II, ICORELS, Millward, Ankudinov/Briggs, Tuck/Stocks, Römisch;
- towing URANS;
- self-propelled URANS with declared propeller representation;
- physical model tests and independent measurements;
- permissioned in-service UKC observations.

No arm is universally best. Stratify by h/T, Fr_h, W/B, hull/vessel class, speed, propeller state, waterway geometry and sensor position. A towing model may be a legitimate benchmark comparator but must not silently stand in for a self-propelled primary prediction when propeller effects are material.

## Benchmark metrics

Report separately:
- sinkage, trim and sensor-point squat bias/MAE/RMSE by regime;
- vertical-force and resistance comparison as different observables;
- interval coverage and width; model discrepancy and out-of-domain rate;
- UKC error at the declared critical point/time;
- false-safe, false-no-go and abstention/unknown rates;
- sensitivity to bathymetry, water level, draught, trim, current, sensor location and correlated uncertainty;
- solver runtime, memory, mesh/time-step sensitivity and configuration reproducibility.

Hard gates are not weighted-score components. A false-safe case cannot be compensated by lower runtime or better mean error.

## Frozen synthetic corpus

The accompanying corpus contains 31 deterministic contract cases:
- two positive controls (self-propelled URANS and trim-aware empirical method), both expected to remain `CASE_CONTRACT_VALID_NOT_PHYSICALLY_QUALIFIED`;
- negative controls for datum, sensor location, trim, propulsion representation, model applicability, convergence, digest, density, source age, forcing validity, calibration, uncertainty, dependency correlation, units, double counting, vessel draught, speed, rights, split leakage and independent validation reference.

The mutation runner and independent Rust oracle are follow-on implementation work. Until that exists and passes exact-head checks, the corpus is a frozen specification/fixture seed—not an executed benchmark result.

## Promotion ladder

1. Contract validity on synthetic fixtures.
2. Formula implementation fidelity and exact documented applicability handling.
3. Reproduction of rights-reviewed model-test reference cases.
4. Matched towing/self-propelled CFD comparison with convergence and physical observables.
5. Permissioned in-service validation at the actual sensor location, including trim.
6. Operational S-129 composition with vessel-/route-/time-specific inputs.

A solver convergence result is not experimental validation. Model-test agreement is not full-scale field validation. S-129 product conformance is not UKC physics validation. None of these results grants permission to transit.
