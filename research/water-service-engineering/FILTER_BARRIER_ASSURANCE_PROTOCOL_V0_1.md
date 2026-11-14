# FILTER-BARRIER-ASSURANCE-001 — Barrier Integrity Contract v0.1

**Tracking issue:** [#5104](https://github.com/Luminous-Dynamics/symtropy/issues/5104)  
**Programme:** water service engineering  
**Status:** synthetic barrier-state contract only; no filter/product/health claim qualified.

## 1. Objective

A filter is not safe because water looks clear, differential pressure is normal, a UV lamp is lit or conductivity is stable. The system must retain the exact claim being evaluated, independent challenge evidence for that claim, the operating envelope, bypass/media/service state, sensor health, maintenance history and separate laboratory sample evidence.

The assurance object has separate outputs:
- Product/barrier claim qualification — does independent challenge evidence cover the requested claim and stated influent/operating envelope?
- Current operating observation — are available sensors healthy and are current measured conditions inside that declared envelope?
- Sample evidence — what does a particular laboratory sample say at its own time/location/analyte/method?
- Compliance authority — reserved to the applicable competent authority and licensed standards, not the software.

The evaluator always sets potability/compliance authorization to false.

## 2. South African and international context

The revised compulsory National Water and Sanitation Services Standards were promulgated in Government Gazette 52814 on 6 June 2025. Regulation 5 requires drinking water to meet applicable SANS 241 prescriptions, water safety plans for supply systems, annual/risk-triggered review, monitoring programmes specifying sampling points/frequency/parameters, credible accredited laboratory methods, IRIS reporting and incident management. The 19 December 2025 Standards Matters notice lists SANS 241:2025 Edition 7. Exact requirements and limits must be checked against the licensed standard and system-specific water safety plan.

WHO's 18 March 2026 report covers 21 newly evaluated household water-treatment products in rounds III and IV and 51 evaluated products overall. WHO's scheme uses independent laboratory challenge protocols and microbiological performance criteria for bacteria, viruses and protozoa. NSF/ANSI 42, 53, 55 and 58 represent different claims (aesthetic, health-related contaminants, UV and reverse osmosis); no single certification is a blanket microbial or chemical safety statement.

Sources:
- Government Gazette 52814, 6 June 2025: https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf
- Government Gazette Standards Matters, 19 December 2025: https://www.gov.za/sites/default/files/gcis_document/202512/53873gen3708.pdf
- WHO HWT Scheme Rounds III and IV (2026): https://www.who.int/westernpacific/publications/i/item/9789240112124
- WHO harmonized testing protocols: https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies/resources
- NSF water-treatment standards overview: https://www.nsf.org/consumer-resources/articles/standards-water-treatment-systems

## 3. Paradigm shift: claim-scoped assurance

Represent the treatment asset as a claim-scoped barrier:
1. What exact contaminant or pathogen class is claimed?
2. Which independent challenge protocol and edition tested it?
3. What influent challenge envelope, flow, temperature, pressure, pH, turbidity/UVT/salinity and cumulative treated volume were validated?
4. Does the current process remain within that envelope?
5. Is the media/membrane/UV source correctly identified, installed, sealed, not expired and not bypassed?
6. Are sensors current, calibrated and located on the right side of the barrier?
7. Has post-maintenance requalification been completed?
8. What independent samples exist, and what is their exact time/location/analyte/method scope?

Missing evidence means unknown or not qualified; it does not mean safe.

## 4. Assurance states

The synthetic evaluator uses separate dimensions, not one green/red safety flag.

Qualification disposition:
- QUALIFIED_FOR_DECLARED_ENVELOPE
- NOT_QUALIFIED
- CLAIM_SCOPE_MISMATCH
- OUT_OF_DOMAIN
- BYPASSED
- OUT_OF_SERVICE
- DEGRADED
- UNKNOWN

Current operating disposition:
- OBSERVED_IN_PROFILE
- OUT_OF_PROFILE
- NOT_OBSERVED
- BYPASSED
- OUT_OF_SERVICE
- UNKNOWN

Independent lab evidence is reported separately as NOT_AVAILABLE, SAMPLE_RESULT_PRESENT or SAMPLE_FLAGGED_FOR_AUTHORITY_REVIEW. It is never copied forward to other times or locations. A flagged result is not automatically adjudicated against a regulatory limit by this evaluator.

## 5. Precedence and fail-closed behavior

1. Known open bypass => BYPASSED.
2. Known expired/retired media or cartridge => OUT_OF_SERVICE.
3. Unknown bypass or media identity/service state => UNKNOWN.
4. Offline critical sensor => UNKNOWN; degraded sensor => DEGRADED.
5. Maintenance without required requalification => UNKNOWN.
6. Missing/expired/failed independent challenge qualification => NOT_QUALIFIED.
7. Requested claim not contained in tested claim scopes => CLAIM_SCOPE_MISMATCH.
8. Current process outside the independently qualified envelope => OUT_OF_DOMAIN.
9. Only if all these gates pass can the claim be QUALIFIED_FOR_DECLARED_ENVELOPE and the current observation be OBSERVED_IN_PROFILE.

## 6. Three deployment levels

### Household / POU
- Locally understandable, fail-visible indicator; no smartphone or network required.
- Flow/volume, pressure differential, cartridge identity/age and tamper/bypass status where relevant.
- Clear instructions for unknown, expiry, bypass and required lab verification.
- Safe storage and post-filter contamination control.
- Never show safe based only on turbidity, pressure or conductivity.

### Community / communal / package plant
- Duty/standby modular barriers and service without loss of all treatment.
- Raw/treated flow, differential pressure, turbidity and disinfectant/UV indicators where process-relevant.
- Operator sampling plan, accredited lab access, spares, calibration, sanitation, inventory and documented maintenance.
- Power loss, dry-run, storage stagnation, contamination ingress and restart tests.

### Municipal / treatment works / distribution
- WSP from source to consumer and risk-based monitoring.
- Process-critical control points, sampling locations/frequencies and laboratory evidence.
- Asset train identity, bypass/cross-connection, redundancy, distribution pressure and disinfectant residual where relevant.
- Incident-management and public reporting remain under Water Services Institution/Authority processes.
- SCADA integration is read-only by default; no unapproved control writes.

## 7. Benchmarks

- WHO microbial log reduction per bacteria/virus/protozoa class where claimed; use applicable harmonized protocol and independent lab.
- Contaminant-specific challenge and breakthrough curves with mass balance.
- Membrane integrity / challenge tests, adsorption breakthrough, particle-size removal or UV validation as appropriate to the process.
- Begin-, mid- and end-of-life performance.
- Operating envelope including flow, pressure, temperature, pH, turbidity/UVT, salinity, source chemistry and cumulative treated volume.
- Bypass/tamper detection, media expiry, sensor outage/drift, maintenance and restart recovery.
- False-green state rate, time to detect degradation, abstention rate, calibration error and detection coverage.
- Cost per litre of water meeting the declared quality profile; energy, chemical use, waste and downtime.

Non-compensable: microbial/chemical challenge claims, integrity and regulatory quality cannot be traded against throughput, energy or cost.

## 8. Negative controls

The benchmark must prove the system does not incorrectly show qualified/in-profile when:
- bypass is open or unknown;
- critical sensor is offline or degraded;
- media is expired, unknown or replaced without requalification;
- independent challenge evidence is missing, expired or for a different claim;
- operating conditions exceed the validated envelope;
- a lab result is flagged but process proxies look normal;
- only turbidity/pressure/conductivity proxies exist without a relevant independent challenge claim.

## 9. Safety / regulatory boundary

No process proxy, software status, filter certificate outside its scope, or synthetic fixture is a drinking-water compliance certificate. Independent challenge testing and accredited sample evidence must be evaluated under the applicable licensed standard and competent authority process. No autonomous chemical dosing, UV setpoint, pressure, valve, bypass, release or public advisory is authorized. Household filter recommendations must not override an official Boil Water or Do Not Use notice.

## 10. Nonclaims

No product, barrier, pathogen log reduction, contaminant reduction, potability, SANS compliance, certification, municipal compliance, treatment life or safe-use claim is established here. All fixtures are synthetic contract tests only.
