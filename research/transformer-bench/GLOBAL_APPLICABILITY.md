# Global transformer engineering: one physics contract, explicit local profiles

**Status:** requirements and architecture proposal; not a universal compliance claim.

## Core principle

The reusable core is a versioned physical/evidence contract. Frequency, voltage, phase/connection, cooling technology, material grade, operating profile, local tariffs and jurisdiction-specific requirements are explicit profile inputs. An output is qualified only for the stated domain and evidence level.

A universal design score would be misleading: transformer loss, thermal duty, insulation coordination, fault withstand, loading, tariff and regulatory requirements depend on application and geography. Keep the physical objectives (loss, temperature, impedance, mass), economic objectives (local-currency NPV under an explicit price basis), and environmental objectives separate. Publish regional rankings before any justified global aggregation.

## Canonical physical context (minimum useful fields)

A future versioned design manifest should bind:
- rated power in VA/kVA/MVA and winding rated voltages in V/kV;
- phase count, frequency in Hz, winding connection/vector group and tap range;
- dry-type/liquid-immersed technology and cooling class;
- core material grade, winding conductor, material lot/revision, and property-curve source;
- no-load loss, load loss, test method, reference frequency/voltage/waveform and reference temperature;
- geometry and winding configuration revisions;
- declared operating envelope (ambient, altitude, loading, harmonics, overload and duty cycle as applicable);
- model/tool/version and evidence identifiers/digests.

Do not convert unknown units or reference conditions by guesswork. An evidence artifact must not merely have a syntactically valid hash: separate verification must hash the artifact, authenticate its provenance and confirm what the underlying measurement establishes.

## Region and utility profiles

A region profile is a versioned requirements bundle, not just a country name. It should identify the jurisdiction, utility/owner, exact source documents, standard edition, publication/effective dates, source digest, applicable clauses/gates, installation environment, voltage/frequency conventions, local loss/efficiency evaluation basis, and reviewer/status.

Use statuses such as DRAFT, VERIFIED, EXPIRED, and CONFLICTING. The profile cannot be VERIFIED if a required source, edition/effective date, reviewer or clause mapping is missing. Recheck the current edition before a real project: old tender PDFs are historical evidence, not proof of current requirements.

Examples of different kinds of profiles to populate after research (not yet certified here):
- IEC-based utility distribution tender;
- IEEE/ANSI C57-based North American distribution procurement;
- South African SANS/utility-specific distribution procurement;
- a rural mini-grid / solar-plus-storage installation profile;
- dry-type indoor transformer versus liquid-immersed outdoor transformer.

Do not assume all members of one country use the same utility requirements.

## Global operating-profile time series

A profile should carry:
- immutable source identity and content digest;
- timestamps in UTC, an explicit declared coverage interval and a named IANA timezone used only for local interpretation/display;
- interval duration or start/end pairs, ordered without gaps or overlap;
- load/current semantics, units, aggregation window and any missing-data policy;
- separate mean current and mean-square current (or a trace from which both can be independently derived);
- tariff currency (ISO 4217), price year, nominal/real basis, tax treatment and currency-conversion source/date when comparison crosses currencies;
- carbon-intensity source/year if environmental comparisons are reported.

Calendar years may contain leap days; local-time profiles may contain daylight-saving transitions. A total of 8,760 hours does not prove temporal coverage: a gap and overlap can cancel arithmetically. Canonical ingestion should validate timestamps and interval continuity, not only summed hours. Do not silently pad missing data, interpolate gaps or map absent tariffs/carbon data to zero.

## Evidence ladder

1. Analytic arithmetic with hand-checkable reference cases.
2. Analytical transformer calculations with pinned reference conditions.
3. Independently qualified FE models, including mesh/convergence studies and held-out geometry.
4. Manufacturer/laboratory test measurements with traceable test conditions and calibrated instrumentation.
5. Applicable regional standards and utility requirements reviewed against exact editions and clauses.
6. Field/site performance and service evidence.

These are distinct claims. Synthetic fixtures verify metric code; FE output verifies a declared numerical model within its qualified domain; physical tests validate specific hardware under recorded conditions; only an applicable compliance process can establish product conformity. Never infer a later level from an earlier one.

## Benchmark matrix for global readiness

At minimum, keep separate test strata for:
- geometry family and physical manufacturer/test lot;
- core material grade and material-lot variation;
- 50 Hz versus 60 Hz (and any additional frequency explicitly supported);
- sinusoidal versus explicitly modeled distorted-waveform/harmonic conditions;
- declared reference/operating temperature and voltage;
- dry-type versus liquid-immersed and the relevant cooling/installation class;
- climate/ambient/altitude conditions where the claimed model uses them;
- regional utility/specification profile for compliance and economic evaluations.

Random row splits alone are not enough. Hold out full geometry families, material lots, manufacturers and temporal test campaigns when the data allow. Report results by stratum and abstain outside the validated domain. Region name is not a substitute for physical features such as ambient, frequency or material behavior.

## Cost and global comparison policy

Within one lifecycle-cost run, use a coherent currency and price basis. For cross-region comparisons bind FX source/date, inflation index, real/nominal conversion and discount rate. Report local-currency outputs alongside any converted comparison. Keep capital cost, energy-loss cost, worst-case cost, carbon and service risk as separate axes; do not hide unit or policy differences in a single opaque score.

Unknown local tariff, carbon factor, legal requirement or failure rate remains unknown. A global ranking is permitted only after comparability rules and aggregation policy are preregistered.

## Regional starting-point matrix (checked 2026-10-29; none verified as a product profile)

| Region / context | Primary starting point | What it teaches us | Current profile status |
|---|---|---|---|
| European Union | [European Commission power-transformer ecodesign page](https://energy-efficient-products.ec.europa.eu/product-list/power-transformers_en) and [Regulation (EU) 2019/1783](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A32019R1783) | The published product rules are scoped to 50 Hz networks and use class-specific loss limits or peak-efficiency-index rules; applicability/exemptions must be checked against the consolidated legal text. | DRAFT; source pointers added, legal scope and clauses not independently qualified |
| United States | [U.S. DOE distribution-transformer page](https://www.energy.gov/cmei/buildings/distribution-transformers) | DOE's product definition explicitly includes 60 Hz and specific voltage/kVA bounds, with listed exclusions; the DOE page says amended-standard compliance is required beginning 2029-04-23. | DRAFT; regulatory scope and exact CFR clauses still require review |
| India | [BIS publication on distribution-transformer ratings and loss context](https://www.bis.gov.in/wp-content/uploads/2025/03/news_2024-10-24_11.pdf) and [IndiaCode source lead](https://indiacode.ecourtsindia.com/rules/specification-of-energy-consumption-standards-for-star-labelled-e284ff7e/) | The referenced material distinguishes no-load and load losses and describes total-loss rules at specified load points; current IS 1180 editions, amendments and mandatory BEE applicability need primary-source verification. | DRAFT; current normative edition and legal status not verified |
| South Africa | [SANS 780:2019 candidate historical copy](https://irp.cdn-website.com/4e0aecb6/files/uploaded/SANS780_2019_Ed5.pdf) and [City Power ground-mounted transformer tender specification](https://www.etenders.gov.za/home/Download/?blobName=47fa128d-b1d1-410c-a0fe-0bdee16b71f9.pdf&downloadedFileName=CP_TSSPEC_164+Rev+3+-+Ground+Mounted+Distribution+Transformers.pdf) | A historical SANS copy and a particular utility tender are research leads, not proof of the current SANS edition or universal South African requirements. The named utility/owner and its current controlled specification are essential. | DRAFT; SABS current status and exact utility requirements not verified |

The U.S. DOE scope reference excludes some transformer categories, so its kVA/frequency summary must not be treated as a universal definition of every transformer bought in the United States. The EU profile must distinguish applicable product regulation from IEC test standards and any national/utility procurement terms. India's star-labeling context must be tied to the current primary legal rules and exact IS 1180 part/amendments, not a mirror or old summary. The South African profile must not infer current applicability from the hosted SANS PDF.

This matrix is intentionally a **research queue**, not a compliance database. Each source record must be captured, hashed and independently reviewed before its region profile can be promoted to VERIFIED. See the initial profile records in `profiles/` and their schema, `profiles/region-profile.schema.json`.

## Standards starting points

These sources are indexes or scope references, not a completed country-by-country compliance review:
- IEC 60076 series: https://webstore.iec.ch/en/publication/588
- U.S. DOE distribution-transformer scope and U.S.-specific efficiency rules: https://www.energy.gov/cmei/buildings/distribution-transformers
- IEEE C57 transformer standards collection: https://standards.ieee.org/wp-content/uploads/import/documents/tocs/powerdistribution.pdf
- Published geometry-aware core-loss surrogate and its stated scope: https://www.sciencedirect.com/science/article/pii/S0378779626010060

For every deployment, confirm applicable current national adoption, utility procurement clauses, edition, amendments, effective dates and test conditions with qualified reviewers. This document does not certify regional profiles or any transformer.

## Immediate implementation sequence

1. Keep the present evaluator marked as an analytic/core-loss screening tool.
2. Extend its reference table to include explicit frequency and excitation class, then report metrics independently by those domains.
3. Create fixtures for at least 50 Hz sinusoidal and 60 Hz sinusoidal domains; no transfer claim unless the held-out domain passes.
4. Version the timestamp/profile and currency contracts before admitting field load/tariff traces.
5. Add a separately reviewed regional profile catalog with evidence-bound editions and explicit qualification statuses.
6. Acquire a named incumbent with traceable loss-test data; then qualify the analytical and FE baselines before training or judging surrogates.
