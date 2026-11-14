# Filtration Industry Comparator Crosswalk v0.1

**Snapshot:** 14 November 2026  
**Tracking issue:** [#5090](https://github.com/Luminous-Dynamics/symtropy/issues/5090)  
**Parent:** [#4958 whole-train benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/4958)  
**Scope:** internal research baseline design only.

## 1. How to compare the industry fairly

There is no single leaderboard spanning all filters. WHO microbial performance classes, NSF contaminant-reduction claims, UV device flow listings, membrane transport, local regulatory compliance and whole-life service outcomes are different evidence dimensions.

The correct comparison key is the exact product/configuration, named claim, source-water profile, flow/pressure, test protocol and measured lifecycle point.

A product's standards badge cannot be turned into a universal removal claim. A WHO microbial star level cannot be compared numerically to an NSF lead/PFAS/glyphosate claim. A listed UV flow rate is not equivalent to a cartridge's tested service capacity.

## 2. WHO household treatment reference set

The WHO products-evaluated page currently identifies membrane, ceramic, UV, flocculation/disinfection, chemical disinfection and solar treatment technologies. Example membrane/combination comparators include:
- **3-star:** LifeStraw Community and ORISA (membrane filtration).
- **2-star:** Drop2Drink Unit (UV + membrane filtration) and Grifaid Family Filter (membrane filtration).
- **1-star targeted:** Katadyn BeFree (membrane) and Nazava (ceramic), with targeted pathogen-class scope.

WHO star requirements:
- 3-star: at least 4-log bacteria, 5-log viruses and 4-log protozoa.
- 2-star: at least 2-log bacteria, 3-log viruses and 2-log protozoa.
- 1-star: at least the 2-star target for two pathogen classes.

The page says performance reflects the time of testing, correct/consistent use matters, and inclusion is not WHO approval or endorsement. Crucially, the WHO page states the list must not be used for commercial or promotional purposes. The product names above are for internal scientific baseline design only. The list does not replace country-specific procurement or regulatory qualification.

Primary list: https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies/products-evaluated

## 3. NSF standards crosswalk

As of this snapshot, the current editions identified in NSF/ANSI standards listings are:
- 42-2025 — aesthetic-effect claims such as chlorine, taste/odour and particulate.
- 53-2025 — specific health-effect contaminant claims.
- 55-2026 — UV microbiological treatment.
- 58-2025 — RO drinking-water systems.
- 244-2026 — supplemental microbial filtration for otherwise microbiologically safe supplies; includes end-of-life-indicator requirements.
- 401-2026 — specified emerging/incidental contaminant claims.

Freeze the exact edition, applicable section, certification and listing for every product before a benchmark run. A standard number is not a product ranking and does not mean every product certified to that standard makes every available claim.

NSF standards overview: https://www.nsf.org/consumer-resources/articles/standards-water-treatment-systems

## 4. New industry frontier: NSF Protocol 549

NSF launched Protocol 549 on 14 October 2026 for independent verification of glyphosate-reduction claims. NSF's announcement states the treated water must contain glyphosate at or below 0.28 mg/L under the protocol and identifies activated-carbon adsorption and reverse osmosis as possible treatment methods. It is aligned with NSF/ANSI 53 and 58.

This is a recent specific-chemical claim benchmark. It should be a named T2 comparator, not a generic filter score.

Before exact replication, obtain the protocol itself and freeze the challenge-water recipe, influent level, sample schedule, claimed flow/capacity, end-of-life conditions, lab method, precision/uncertainty and acceptance rule. The public announcement alone does not establish AMPA co-removal, local legal limits or product performance.

Source: https://www.nsf.org/in/en/news/nsf-launches-protocol-549-for-glyphosate-reduction-claims

## 5. Example NSF UV point-of-entry flow baselines

The NSF official POU/POE listing snapshot dated 1 November 2026 includes Applied Membranes models such as:
- UV-SPNSFA-5.4M-HP — 5.4 US gpm, Class A (about 20.44 L/min).
- UV-SPNSFA-18M-HP — 18 US gpm, Class A (about 68.14 L/min).
- UV-SPNSFB-13 — 13 US gpm, Class B (about 49.21 L/min).

These exact listings provide useful community/POE flow points, but Class A and Class B are not equivalent. The listing is for UV system claims; it does not establish mechanical-filter pressure loss, cartridge ELI, household low-flow performance or chemical removal.

Live listing: https://info.nsf.org/Certified/DWTU/Listings.asp?ProductType=Point-of-Entry&Standard=055

## 6. South African requirements as a separate axis

The 6 June 2025 revised compulsory national water and sanitation services standards require fit-for-consumption drinking water under applicable SANS 241 prescripts, water safety plans, defined monitoring programmes, credible lab analytical methods, reporting to DWS IRIS within 30 days and retention of records for at least five years.

This is an institutional water-service obligation, not a claim that a filter product is certified. Local benchmark profiles must freeze the applicable SANS 241 edition, determinants, sample points, frequency, method and incident-response requirements.

Government Gazette No. 52814: https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf

## 7. How Luminous can exceed current practice

The high-value gap is not merely adding another adsorbent or a more complex membrane. It is **claim-bounded lifetime qualification**:
1. Reproduce the exact comparator protocol and challenge profile first.
2. Use multiple production units and batches with blinded independent laboratory samples.
3. Measure breakthrough across the complete rated life, including end-of-life and post-capacity behaviour.
4. Add intermittent use, stagnation/restart, variable turbidity, pressure/flow excursions and source-profile shifts as separately labelled robustness tracks.
5. Pair an ELI with actual claim-specific sample evidence during qualification; a throughput counter or pressure sensor is not contaminant evidence.
6. Measure delivered service volume, downtime, replacement burden, reject/backwash, energy and operating labour.
7. Compare by source profile and intended use, not with one universal ranking.
8. Publish uncertainty, failures, no-results and out-of-domain states.

Proposed goals (not achieved): at least 20% more verified service volume or 20% fewer premature replacements at equal or better false-safe/missed-breakthrough risk in at least one matched profile; lower lifecycle cost per 1,000 L of profile-conforming delivered water; and zero false-safe results in the negative-control corpus.

## 8. Nonclaims

This is a dated public-source crosswalk, not product testing. WHO inclusion is not endorsement; NSF listing applies to exact models and claims; P549 is not a South African statutory limit; no Luminous filter, membrane, water sample or treatment train is certified or qualified by this document.
