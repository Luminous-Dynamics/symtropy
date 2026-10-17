# Washer Acquisition and Parts-Resilience Plan v0.1

**Status:** desk-research and procurement protocol only. No appliance has been bought, borrowed, opened, tested or safety-qualified by this plan. Prices and availability below are a dated sourcing snapshot, not offers, purchase authorization or verified parts quotations.  
**Related work:** [WASHER-001 #1828](https://github.com/Luminous-Dynamics/symtropy/issues/1828), [large-appliance architecture PR #1827](https://github.com/Luminous-Dynamics/symtropy/pull/1827).  
**Sourcing snapshot:** 17 October 2026; South African retail market used as an initial acquisition context, not as a universal market assumption.

## 1. Why this is the next engineering step

A service-friendly machine can still be expensive or effectively unrepairable when its exact part is unavailable, compatibility is uncertain, shipping takes weeks, diagnostic information is locked away, or a low-price part requires replacing a costly assembly. Repairability therefore needs two linked but distinct experiments:

1. **Architecture/serviceability:** how safely, accurately and quickly can a known fault be diagnosed, repaired and verified?
2. **Supply resilience:** can the correct part or a qualified substitute be identified, sourced and delivered at a defensible total cost?

Do not blend either result into a single score that hides a safety failure, a function regression or an unknown. The product should optimize *verified restoration of useful function per lifecycle cost*, not the number of modules, the cheapest BOM or the lowest quoted part price alone.

## 2. Preserve experimental validity while keeping the first spend low

Use the following evidence ladder. Each level answers a different question and cannot stand in for the next.

| Level | Activity | What it can support | What it cannot support |
|---|---|---|---|
| A — zero-cost desk study | Freeze candidate criteria; collect public manuals/specs, exact model identities, current price/stock snapshots, parts listings, shipping and service terms | A documented acquisition decision and list of evidence gaps | Physical accessibility, repair time, real performance, durability or a repair-cost claim |
| B — donor teardown | Inspect one de-energized, drained, appropriately handled donor; photograph/measure access, fasteners, part boundaries and discarded mass | Descriptive teardown findings for that exact unit | A matched performance comparison, population-level failure rate or comparative causal claim |
| C — control-only pilot | Test at least three physical units of one frozen control design/revision with a frozen workload and calibrated measurement setup | Initial estimate of within-control variability, instrumentation repeatability and test-operability | Strong effect claims, service-life claims or reliability certification |
| D — candidate comparison | Test candidate design(s) against the frozen control under the preregistered protocol; sample plan informed by the pilot | Bounded matched comparisons with uncertainty and exclusions disclosed | Universal superiority or untested market/customer claims |
| E — independent replication | Independent builder/operator repeats the specified test with traceable configuration and instruments | Stronger reproducibility evidence | Regulatory conformity unless required conformity testing has separately passed |

**Unit-count rule:** three different models are not a three-unit repeatability set. The initial control pilot uses at least three physical units of the same selected design/revision, recording serial or production-lot identity privately where appropriate. Candidate comparisons need a separately justified candidate sample plan; three control units do not automatically make a candidate comparison adequately powered. If funding allows only one physical unit per design, report a case study and measurement traces—do not label it a statistically supported design win.

A used donor can be excellent for reverse-engineering access and fastener patterns, but it is not a substitute for a matched, functioning control. Do not buy the six-unit comparison set before a control-only pilot demonstrates that the protocol and equipment produce repeatable measurements.

## 3. South African 8 kg front-loader sourcing shortlist

The goal is to identify a feasible control; **no model is selected yet**. Specs, SKU suffixes and retailer listings must be reconciled before purchase. Prices below are discovery values captured from the linked public pages around the snapshot date.

| Candidate lead | Publicly listed information | Evidence available | Key unresolved check |
|---|---|---|---|
| **Defy DAW 408** | Official product page states 8 kg, 1200 rpm, 230 V / 50 Hz and lists a user manual. Retail comparison pages have shown offers around R5,499–R5,899. | [Defy official model page](https://www.defy.co.za/products/freestanding-washing-machines/freestanding-washing-machines-front-loader-daw-408); [price snapshot](https://www.pricecheck.co.za/offers/254915254/Defy%2BDAW408%2B8KG%2BFront%2BLoader%2BWashing%2BMachine%2BManhattan%2BGrey) | Defy’s official page did not itself show an online retailer when checked. Confirm exact DAW 408 versus DAW408B suffix, current stock, revision and parts compatibility. |
| **Hisense WF1I8022BT** | Retailer page lists 8 kg, 1200 rpm; R5,199 sale price / R5,999 regular price, with the sale advertised through 31 October 2026. | [HiFi Corp listing](https://www.hificorp.co.za/hisense-8kg-front-load-washer-titanium-grey-wf1i8022bt) | Reconfirm stock and delivery at acquisition time; obtain model-specific manual, service access terms and itemized exact-SKU parts availability. |
| **Samsung WW80T3040BS/FA** | Samsung South Africa’s product page states 8 kg and publishes a user manual; its specs list 1400 rpm. A price aggregator has listed this model around R5,499. | [Samsung South Africa](https://www.samsung.com/za/washers-and-dryers/washing-machines/ww3000tm-front-loading-quick-wash-drum-clean-delay-end-8kg-silver-ww80t3040bs-fa/); [price discovery](https://www.pricesite.co.za/washing-machines/?type=front-loader) | Confirm suffix, current local stock and exact revision; its 1400 rpm maximum differs from the 1200 rpm candidates and must be treated as a matching variable, not ignored. |
| **Midea MF110W80B-T** | South African retailer listings have shown 8 kg units around R4,999.99; a South African-hosted owner manual is publicly accessible. | [manual PDF](https://retail.mideasouthafrica.com/wp-content/uploads/MF110W80B-T-Manual.pdf); [Midea South Africa support](https://www.midea.com/za/support-southafrica) | Verify the current local retail quotation and service-parts pathway for the exact SKU; an owner manual is not a service manual or proof of part availability. |

These four leads are **not a matched benchmark group**. Capacity alone does not match drum volume, control software, spin speed, heating behaviour, cycle definition, condition, load profile or manufacturing revision. Do not average their advertised labels and call that a control baseline.

### Control selection rule

Choose one model only after a documented selection review that includes:

- at least three physically obtainable units of the same model/revision for the control-only repeatability pilot, or an explicit, lower-claim plan if this is not affordable;
- comparable installation and electrical/water requirements at the actual test location;
- manuals, rating labels, approved service information and exact model/part identifiers;
- current delivered price and terms from more than one seller where practicable;
- a parts enquiry covering common service items and at least one high-cost assembly, with written compatibility confirmation;
- a record of stock, delivery estimate, freight/return costs, warranty, service restrictions and unanswered enquiries;
- independent confirmation that the selected workload and test method can be run without changing safety protections.

If three identical new units are financially infeasible, first complete Level A and seek lab access, a loan, a manufacturer-independent repair partner or suitable same-model used units. Do not silently downgrade a repeatability study into a cross-model comparison. No purchase is authorized by this document.

## 4. Parts-resilience experiment

For each chosen model, request a dated quote for the following **fault/repair classes**, not merely a generic brand catalogue:

1. drain filter and pump assembly;
2. inlet valve and inlet filter;
3. door lock and door seal;
4. pressure/water-level sensor and harness;
5. heater and temperature sensor;
6. shock absorbers/springs;
7. control board and display;
8. motor/drive component;
9. drum bearing/seal assembly or tub assembly;
10. hoses, clamps, seals and other recurring consumables.

The list is a comparison curriculum, not a claim that these items fail most often. Update the ranking when valid repair-history data become available.

For every query record: exact model/revision fit; part number and supersessions; OEM or compatible status; proof of compatibility; price and quote date; stock status; dispatch time; delivered time; delivery/return terms; warranty; seller identity; geographic origin; whether a second independent lawful source exists; whether the source requires technician credentials or an account; and what documentation remains inaccessible. Distinguish **listed**, **quoted**, **in stock**, **ordered**, **delivered**, **installed** and **verified**. These are not synonyms.

A catalogue entry or a phone assurance is not proof that the item fits the test unit. Do not treat a visually similar component as compatible without dimensional, electrical, thermal, material, pressure/flow and interface evidence appropriate to that part. Any substitute used in a physical test must have a testable identity and a declared qualification boundary.

### Proposed parts-resilience metrics

| Metric | Operational definition | Reporting rule |
|---|---|---|
| Exact-fit evidence rate | Requested part classes with written model/revision compatibility evidence ÷ all part classes queried | Missing evidence remains unknown; do not count a generic catalogue hit as a match |
| Local-source coverage | Part classes with at least one verified supplier in the target market ÷ part classes queried | Report both all classes and the declared common-repair subset |
| Dual-source coverage | Classes with two independent, verified lawful sources ÷ classes queried | Sellers that merely resell the same sole distributor do not necessarily count as independent |
| Delivered lead time | Calendar/work days from dated order acceptance to physical delivery | Report median/P90 only with adequate complete observations; otherwise show each observation and missing values |
| Fulfilment rate | Orders delivered as quoted ÷ orders accepted | Keep cancellation, substitution and incomplete delivery separate |
| Repair affordability ratio | Parts + shipping + realistic labour/diagnosis + expected rework for verified restoration ÷ current like-for-like replacement price | Show parts-only and all-in ratios separately; include uncertainty and market date |
| Assembly penalty | Extra spend and discarded mass when a small faulty component is sold only in a larger assembly | Do not assume the smaller part is independently repairable or safe to replace |
| Documentation access | Required diagnostic/service steps possible from legally available model-specific information and permitted tools | Record each unavailable/credential-gated item and any lawful access path |
| End-to-end restoration time | First triage to post-repair verified function, including sourcing waits, repeat trips and failed attempts | Report hands-on repair time separately from elapsed outage |
| Repair outcome | Post-repair function, relevant safety checks and follow-up recurrence status | A powered-on display alone is not a restored-function receipt |

### Initial open-design targets (hypotheses, not measured achievements)

- For each of the ten repair classes, publish model-independent interface requirements, service procedure, compatible-part evidence and the reason for any single-source dependency.
- Aim for **two independently verified supply paths** for the most consequential/common repair classes; report all exceptions instead of gaming coverage with low-value consumables.
- Aim for **90% of declared routine-repair parts delivered locally within five business days** in the named launch market, measured from dated orders and reported with P50/P90. This may prove too expensive in the first region; preserve the actual result and the cost of meeting it.
- Aim for the most common, qualified repair classes to have an all-in restoration cost below **35% of a like-for-like replacement price**, but only where safety, functional verification and expected recurrence remain acceptable. Publish exceptions and confidence bands; a low-cost but short-lived repair is not a win.
- Publish a lawful offline documentation bundle and a versioned parts-compatibility manifest. Vendor cloud access must not be the sole source of essential user maintenance instructions or basic ownership records.
- Evaluate a 10-year parts-support target for the open design as a project commitment to cost and inventory-model, not a claim that all parts are available today or that one rule automatically applies in every market.

These goals should be revised only before the scored candidate phase, based on a declared affordability model, market feedback and pilot repeatability—not after seeing which design scores best.

## 5. Cost model: optimize restoration, not the checkout bill

For fault class (f), market (m) and design (d), estimate:

[
C_{mathrm{restore}}(d,f,m)=C_{mathrm{diagnosis}}+C_{mathrm{parts}}+C_{mathrm{freight}}+C_{mathrm{labour}}+C_{mathrm{waiting}}+C_{mathrm{rework}}+C_{mathrm{disposal}}
]

Keep waiting/outage cost separate from cash outlay and show alternative assumptions for households that cannot monetize downtime. For the product life-cycle view, add purchase/installation, maintenance, energy, water, detergent/consumables, expected repair events, failed repairs and end-of-life handling. Do not double-count parts or labour, and do not set unknown repair frequency or downtime to zero.

Evaluate sensitivity over local tariffs, use intensity, technician access, import lead times, exchange-rate exposure, spare-part prices and product lifetime. Compare service-spine designs against conventional designs on *the same declared fault curriculum and workload*. An extra connector or independently replaceable module is justified only if the expected reduction in total restoration/resource burden exceeds the added BOM, assembly, sealing, inventory, failure and test costs.

A responsible first economic result is a transparent range with its assumptions—not a single ROI point estimate unsupported by field data.

## 6. Safe sequence and stop gates

1. **Desk gate:** freeze the selected design/revision, candidate rationale, parts-query template, source log and intended claims before any purchase.
2. **Physical safety gate:** obtain a qualified hazard review for lifting, water containment, mains isolation, stored energy, heater, rotating assembly, pinch/crush hazards and post-repair electrical/leak checks.
3. **Control-only gate:** freeze the test workload, instruments/calibration evidence, cleaning/rinsing method, invalid-run rules and data schema; establish repeatability before candidate testing.
4. **Candidate freeze:** before the first candidate result, freeze functional non-inferiority margins, economic thresholds, exclusions, sample plan, analysis and stopping rules.
5. **Service-task gate:** only manufacturer-intended, reviewed user tasks may enter user-maintenance trials. Internal mains/drive/heater work, heavy tub/bearing work and any required protective-electrical check stay with qualified personnel and the approved procedure.
6. **Claim gate:** preserve all raw observations, exact unit/BOM/part/procedure revisions, prices/quotes, calibration records, failed repair attempts, deviations, uncertainty and digests. A safety-critical finding cannot be averaged away by a low cost or short repair time.

No live work with guards removed, no defeat of protective devices, no amateur opening or charging of refrigerant circuits, and no assumption that easier access makes a hazardous task user-safe.

## 7. Source notes and interpretation limits

- [IEC 60456:2024](https://webstore.iec.ch/en/publication/70049) specifies performance measurement methods for household washing machines. Use the standard’s licensed/current text and the applicable local requirements when selecting or claiming conformity to a method; this document does not reproduce or certify against it.
- [EU Regulation 2019/2023](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32019R2023) contains model-specific ecodesign resource-efficiency and spare-parts requirements for covered washers/washer-dryers in its jurisdiction. It is a useful design benchmark, not proof that the same obligations apply in South Africa or that a listed part is available to us.
- [Open Repair Alliance data-production notes](https://openrepair.org/open-data/producing-the-open-repair-alliance-datasets/) describe partner collection, mapping and cleaning processes. The ORDS system primarily aggregates community repair-event data and is not automatically a representative sample of large-appliance service failures; use it to improve data collection, not to invent a washer failure-rate prior.
- [Defy Service Centre spares catalogue](https://defyservice.co.za/product-category/spares/) demonstrates that public online spare-part listings exist. It does not, by itself, establish fit for DAW 408/DAW408B or prove that a quoted part will be delivered.
- [NIST measurement uncertainty](https://www.nist.gov/itl/sed/topic-areas/measurement-uncertainty) is a useful reference for treating instrument limits and uncertainty as part of the evidence, not as an afterthought.

## 8. Decision requested from evidence—not preference

The desk phase should return a comparison table, exact-model documentation bundle, parts-source/quote log, uncertain/unavailable field list, and a recommendation with reasons. If it cannot identify three comparable physical units of a suitable control, it should explicitly recommend the next-lowest-cost valid path (loan, lab collaboration, or a descriptive donor teardown) and lower the claim ceiling accordingly.

**Claim ceiling:** this plan does not show that any shortlisted washer is more repairable, that its parts are reliably available, that a target can be met, or that a proposed appliance is cheaper, safer, more efficient or compliant. Such claims require the physical evidence and independent checks stated above.
