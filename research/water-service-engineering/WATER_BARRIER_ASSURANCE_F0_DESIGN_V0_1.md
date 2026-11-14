# WATER-BARRIER-ASSURANCE-001 — F0 Claim-Conditioned Barrier Assurance v0.1

**Tracking issue:** [#5136](https://github.com/Luminous-Dynamics/symtropy/issues/5136)  
**Evidence class:** synthetic decision-contract fixtures only. No filter has been tested or certified.

## Core paradigm

A filter's technology label is not a removal claim. The assurance kernel accepts only explicit contaminant/pathogen claims attached to an exact module configuration, independent evidence identifier, certificate state, service state and operating envelope. Modules contribute only claims while in service, certificate-valid, exact-configuration matched, within their declared flow/pressure/temperature/turbidity range and with remaining capacity.

For microbial claims, bacteria, viruses and protozoa remain separate. The default comprehensive 2-star and 3-star comparison thresholds follow the WHO International Scheme:
- 2-star: bacteria ≥2 log10, viruses ≥3 log10, protozoa ≥2 log10.
- 3-star: bacteria ≥4 log10, viruses ≥5 log10, protozoa ≥4 log10.

WHO's Round IV protocols are the current protocol references for product assessment; F0 does not reproduce a WHO lab challenge or award a star rating. The 2026 Scheme report records 51 products evaluated across Rounds I–IV. Sources: [WHO scheme](https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies), [2026 results](https://www.who.int/westernpacific/publications/i/item/9789240112124), [Round IV protocols](https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies/resources).

Chemical hazards require exact contaminant identifiers and explicit percentage-reduction claims. No generic carbon, ceramic, UF, NF, RO or UV module name provides implied coverage. NSF/ANSI 42, 53 and 401 claims are also contaminant-specific; retain exact edition, model, configuration and certificate scope: [NSF overview](https://www.nsf.org/in/en/knowledge-library/nsf-ansi-42-53-and-401-filtration-systems-standards).

## F0 decision states

- INDETERMINATE — source profile stale/changed, hazard unknown, critical process measurement invalid/unknown, or integrity unknown.
- OUT_OF_ENVELOPE — module expired/capacity exhausted or operating conditions outside the module's declared limits.
- GAP — required contaminant/pathogen claim missing or insufficient, certificate/configuration invalid, bypass open or storage compromised.
- VERIFICATION_REQUIRED — declared claims cover the recorded hazards but there is no current accredited post-treatment laboratory sample.
- COVERED_WITHIN_DECLARED_CLAIMS_REQUIRES_AUTHORITY_REVIEW — claims and operating state pass and a current accredited sample is recorded. This is not a statement that water is safe.

The evaluator always sets release_authorized=false and potability_claim_permitted=false. It never outputs SAFE_TO_DRINK or RELEASE_AUTHORIZED.

## Operational observability versus health evidence

Turbidity, conductivity/TDS, pH, ORP, flow, pressure differential, UV status and disinfectant residual are useful process indicators. They do not independently prove the absence of bacteria, viruses, protozoa, metals or all chemical contaminants. A laboratory result applies to its sample time, location, analyte, method, detection limit and chain of custody. Sensor proxies cannot be promoted to pathogen or chemical results.

WHO guidance for small supplies prioritizes risk-based operational monitoring, sanitary inspection and effective barriers; microbiological tests are an adjunct rather than a substitute for barrier integrity. [WHO small supplies guidance](https://www.who.int/publications-detail-redirect/9789240088740).

For South African Water Services Institutions, align deployments with applicable current SANS 241 prescripts and a water safety plan. Government Gazette 52814 (6 June 2025) requires WSPs, drinking-water monitoring plans, defined sampling points/frequency/parameters, credible ISO/IEC 17025 laboratory methods and reporting. [Gazette 52814](https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf).

## F0 synthetic test population

Eighteen adversarial cases test complete 2-star evidence and higher 3-star requirement; missing virus claim and two-class-only targeted coverage; chemical claim gap; stale or changed source profile; unknown hazard; flow outside the certificate envelope; exhausted capacity; bypass and compromised storage; absent or stale accredited post-treatment sample; invalid/unknown critical measurements; power unavailable for a powered module; and certificate/configuration mismatch.

The one-day source freshness and laboratory-sample-age windows are explicit synthetic policy inputs, not regulatory monitoring frequencies. A deployment must set them from its approved water safety plan and jurisdictional requirements.

## Next benchmark stages

F1 should test household modules across beginning/middle/end-of-life, actual challenge concentrations and storage/recontamination. F2 should test community treatment under hydraulic peaks, power interruption, redundancy and operator maintenance. F3 should measure municipal detection-to-isolation and safe restoration. F4 should measure correct use, replacement access, downtime, affordability and human factors.

Benchmark against WHO-evaluated household products, claim-specific NSF-certified products and conventional point-of-entry/community/municipal treatment trains. Use matched influent profile, flow, capacity, contact time, module age, operating conditions, bypass state and sample point. Do not compare on a single percent-removal number.

## Nonclaims

F0 does not certify a product, treatment train, water quality, potability or field service. No filter performance, chemical/pathogen removal, laboratory result, water saving or health outcome is claimed.
