# LMF-FDM-300 Part-Demand Corpus and Intake Protocol v0.1

**Status:** intake protocol. No demand quantities, part dimensions, use frequencies or economic wins have been measured in this document.
**Owner:** [LMF-FDM-300-ARCH-001 #6787](https://github.com/Luminous-Dynamics/symtropy/issues/6787)

## Purpose

The printer's build volume and architecture must be derived from the actual work the machine is expected to perform. Do not choose a 300 mm, 325 mm or 350 mm cube because it sounds useful. First build a versioned demand corpus from specific parts, tools, fixtures, jigs, packaging and low-risk accessories.

A part enters the corpus only when it has an identifiable owner/use case and an evidence status. Unknown dimensions, frequency, rights or material requirements remain unknown—not zero, not assumed and not inferred from a screenshot.

## Current explicit seed records

1. `FAN-PACKAGING-SPACER-001`: a non-functional packaging/transport spacer candidate. It has been described in #6735 but is not admitted. Its exact CAD, dimensions, packaging loads, material, annual demand, print time and transportation test are not established.
2. `FAN-MFG-ALIGNMENT-JIG-001`: a manufacturing alignment fixture candidate. It is not admitted. Its reference surfaces, drawing, tolerances, fixture repeatability, loads, material and verification method are not established. It must not be used as an unqualified measurement standard.

Other catalogue entries should be added only when the exact product/task is identified. The intake file includes empty slots; those are not demands.

## Required record

For every actual candidate, capture:
- exact part ID, owner, product and product revision/effectivity;
- functional purpose and prohibited use;
- P0–P3 consequence class and proposed H0/N1/R2/Q3 tier (separately adjudicated);
- source CAD/drawing revision and manufacturing rights;
- X/Y/Z bounding dimensions and measurement method/uncertainty;
- critical interfaces, datums, tolerances and fit/function tests;
- material, supplier/lot and conditioning needs;
- orientation/support options, slicer profile and process restrictions;
- environmental exposure: temperature, moisture, UV, chemicals, load, fatigue/creep as applicable;
- annual demand and demand evidence period, split by repair, new production and contingency stock;
- failure consequence, part waiting time and current OEM/job-shop route;
- inspection method, acceptance criteria, witness coupon/test and post-install verification;
- baseline price/quote source/date, lead time and full cost assumptions;
- record confidence, source URL/file and unresolved questions.

A service-ticket count is not automatically annual demand: note observation period, appliance population, duplicate returns and censored/unknown failures. Separate actual measured demand from a forecast or scenario.

## Build-volume analysis

For each exact part geometry and orientation:
- test whether it fits within each candidate machine's usable envelope with clearances, support, brim, purge and toolhead/nozzle restrictions;
- report volume utilization and footprint utilization, not just bounding-box fit;
- compare one-piece manufacture against split-and-join, different process routes and OEM/local shop supply;
- record support waste, estimated time and any dimensional/quality penalties;
- do not count large parts as “covered” if they only fit after a join that changes the part's safety/function or adds an unqualified interface.

Test candidate machine volumes (e.g. 250-class, 300-class, 325-class and other corpus-driven options) against the *same frozen set*. Do not tune the test set after seeing which machine wins.

## Prioritisation

Rank candidates by user value and evidence, not printability alone:
1. pain of unavailability and verified restoration-time opportunity;
2. rights and exact compatibility confidence;
3. part-risk class and consequence of failure;
4. annual demand/volume distribution;
5. process/material suitability and qualification burden;
6. inspection and post-repair verification feasibility;
7. full cost per accepted output versus OEM/job shop;
8. local machine/facility access and transport;
9. environmental/material impact.

A P3 item is not a household candidate even if it is geometrically easy to print. A large part is not a reason to buy a larger machine if demand is one-off and outsourcing is cheaper.

## Gate before FDM architecture selection

Freeze only after:
- a minimum of 20 actual part/task records, or an evidence-backed explanation why a smaller basket represents the initial demand;
- dimensions and rights recorded for every part used in envelope-coverage calculations;
- actual demand or explicitly labelled forecasts with observation periods;
- a baseline OEM/job-shop route for each benchmarked part;
- machine candidates compared on the same corpus;
- risks and qualification burden kept separate from geometry.

No 90th-percentile build-volume claim unless the demand sample and sampling frame support it. For a small pilot, report the finite set and each item's status instead of implying population-level percentiles.

## Claim ceiling

The intake template does not establish demand, dimensions, rights, fit, cost savings or a printer requirement. Only actual source-backed records can contribute to machine selection or SOTA comparison.
