# WaterLoop source-characterisation campaign protocol v0.1

**Tracking:** [WATERLOOP-FEED-CHAR-001 #5545](https://github.com/Luminous-Dynamics/symtropy/issues/5545)  
**Parent:** [WATERLOOP-MODULAR-TRAIN-001 #5475](https://github.com/Luminous-Dynamics/symtropy/issues/5475)  
**Observation/assessment authority:** [Mycelix #6100](https://github.com/Luminous-Dynamics/mycelix/issues/6100)  
**Status:** preregistration design; no samples acquired  
**Evidence class:** design_only  
**Initial stream:** separately captured bath/shower light greywater from a permitted, fully serviced, managed site  
**End-use boundary:** no reclaimed-water end use during this source-characterisation campaign

## 1. Decision to be informed

The campaign is designed to answer:

> What distribution of source composition, mass loading, flow, time variation and storage transformation does the proposed first feed actually present to the WaterLoop process bay, and what feed envelope can each candidate treatment module defensibly accept?

It does not answer which train is safe or best by itself. It supplies an empirically described feed challenge population for the matched treatment-module tournament in #5475.

Do not select a filter, membrane, wetland, disinfection method or process-control threshold from:
- a single grab sample;
- one household/day;
- a literature average;
- a vendor's nominal design envelope;
- an online proxy such as turbidity or conductivity alone;
- a synthetic challenge feed alone.

## 2. Research basis and the reason to measure distributions

Ninsiima et al. (2026) review 131 empirical studies and show wide source variability (reported COD 100–2,000 mg/L, TSS 30–1,000 mg/L, laundry surfactants frequently above 100 mg/L), linked to source mixing, household behaviour, temporal variation and storage transformations. Their review argues for property-driven treatment screening rather than technology selection by headline removal. DOI: https://doi.org/10.1007/s42452-026-09559-0

South African WRC TT 746/17 states that greywater quality and risk depend on source and intended use. It also distinguishes grab/source classes, discusses risk/uncertainty and QA/QC, and states that monitoring is required to check selected process performance. The WRC describes its own report as guidance, not a prescriptive rulebook; it does not define a universal feed specification. Source: https://www.wrc.org.za/wp-content/uploads/mdocs/TT%20746-17.pdf

A 2026 residential bench comparison uses synthetic influent and reports different trade-offs for UF/UV and ozone/GAC/UV. Because its feed was synthetic and the study bench-scale, it is a reason to test against real local source batches, not a basis to claim either train will win locally. DOI: https://doi.org/10.1111/1752-1688.70162

## 3. Phase gates

### Phase A — clean-water measurement-system shakedown

Use the clean-water-only unoccupied bench first. Verify flow, volume, level/geometry, elapsed time, energy, sample-port access, reject routing, containment, calibration and timebase. No greywater is introduced in this phase.

### Phase B — source characterisation, no reuse

Begin only when site permission, source-collection design, plumbing/process review, health and safety plan, laboratory instructions, preservation/transport arrangements and approved disposal route are admitted.

The first feed is bathroom bath/shower only, held as a dedicated source class. This phase characterises the source; no treatment outlet or end-use fixture receives the collected water.

### Phase C — contained storage transformation study (optional, separately approved)

If the process design needs storage-age data, test separately in a closed, approved, unoccupied test cell with a preapproved protocol. Distinguish:
- greywater process-residence time (age of the feed before treatment);
- laboratory sample-preservation and transport time (method-specific);
- the age of an aliquot after it has been collected for analysis.

The WRC's 24-hour guidance for untreated greywater is an outer risk boundary, not a design target. The campaign must not hold untreated feed beyond the approved limit. Any samples at a time point are collected and handled according to the laboratory's instructions.

### Phase D — matched treatment-feed batches

Only after Phase B has a provisional source envelope and Phase C is completed or explicitly excluded. Use aliquots from the same identified source batch for competing trains, with predeclared random assignment, equal hydraulic/load conditions and exact configuration receipts. Retain batch/event identity throughout.

Synthetic feeds remain a separate screening population; do not combine synthetic and actual greywater observations in one empirical influent distribution.

## 4. Source classes and exclusions

| ID | Source | V0.1 campaign status | Reason / future gate |
|---|---|---|---|
| SRC-BATH-01 | Bath and shower | First candidate feed after approvals | Keep batch/source identity; retain bath vs shower sub-source if separately measurable |
| SRC-BASIN-01 | Hand basin | Excluded from initial campaign; separate future stratum | Characterise separately; do not silently merge into bathroom profile |
| SRC-LAUNDRY-01 | Laundry wash/rinse | Excluded; separate future profile | Surfactants, sodium, boron, pH/salinity and solids can change treatment feasibility |
| SRC-KITCHEN-01 | Kitchen/dishwasher | Excluded | Grease, food solids, organic and microbial load need a distinct design |
| SRC-BLACKWATER-01 | Toilet/urinal/bidet/faecal contamination | Excluded | Not greywater for this campaign |
| SRC-CHEM-01 | Paint, solvent, vehicle, workshop or strong chemical cleaning water | Excluded | Not an admitted household greywater input |
| SRC-RAIN-01 | Roof rainwater | Separate future programme | Different source, collection and risk envelope |

No source mixing is allowed. A wrong-source event is preserved as a negative-control event and rejected from the SRC-BATH-01 population.

The first campaign is limited to a fully serviced, managed site. It is not a collection programme for informal or unserviced dense settlements.

## 5. Sampling design principles

This protocol specifies data-design requirements, not field sampling instructions. The receiving laboratory and qualified professionals must specify sample containers, volumes, preservatives, temperature, holding time, transport, PPE, cleaning/decontamination, safety and disposal.

### 5.1 Unit of observation and hierarchy

The experimental hierarchy is:

`site → source point → source event/batch → collection method → aliquot/sample → analyte result`

A shower event, an aliquot, a laboratory replicate and a household are not interchangeable statistical units. Repeated samples from one household do not count as independent households.

Every event gets a coded event identity. Each split aliquot gets its own sample identity and links back to the parent event/batch. Never create a new batch identity merely because the sample is split for different labs or treatment trains.

### 5.2 Representative sample method

For each measurand and decision, the laboratory/professional plan chooses whether a grab, time-composite, flow-proportional composite or another method is fit.

Retain the exact sample collection mode:
- grab observations represent a specific point/time;
- time-composites represent a stated time-weighting rule;
- flow-proportional composites represent a stated flow-weighting rule;
- aliquots from a composite are not independent source events.

Record the collection interval and the aggregation rule. Do not label a spot sample as an event average or compare two differently constructed composites as if they had identical representativeness.

### 5.3 Stratification

The preregistration must define which strata matter to the process and intended source envelope. Candidate non-identifying context includes:
- source subtype (bath or shower);
- event duration and measured event volume/flow profile;
- time-of-day or season band;
- water/source temperature;
- coded cleaning/personal-care product class, only where consented and materially relevant;
- occupancy band or number of use events, only where justified and consented;
- system idle/storage state;
- unusual cleaning/product event or excluded-source suspicion.

Do not capture resident names, exact addresses, sensitive personal-care behaviour or data unnecessary for the treatment question. Site identity is coded and controlled by the site/spatial-data owner.

### 5.4 Replication and sample count

There is no universal minimum sample count in this document.

Use a pilot phase to estimate within-site, between-site and event-to-event variability. The statistician/process designer preregisters:
- primary feed parameters and quantiles of interest;
- desired confidence/uncertainty interval precision;
- site-cluster and repeated-event design;
- handling of seasonal/operating strata;
- stopping rule and treatment of missing/censored results;
- sample count and power calculation.

The main campaign proceeds only after this design is frozen. It must span the declared source strata; it must not simply continue until a preferred treatment train looks good.

## 6. Candidate analytical panel

The exact analytes, method, detection/quantification limit, preservation, holding time, lab, decision rule and acceptance criteria are set by the laboratory and qualified water-quality assessor. No numeric toilet-flushing acceptance limit is created by this protocol.

### Core screening candidates

- temperature;
- pH;
- electrical conductivity;
- turbidity;
- total suspended solids;
- COD and/or BOD, as method-appropriate;
- microbiological indicators selected by the risk assessor (e.g. E. coli and/or total coliform);
- source flow and event volume.

### Source-specific follow-up

For shower/bath, include surfactant/personal-care product indicators where the professional screening shows they are relevant to downstream treatment, fouling or reuse risk.

For a future separately approved laundry campaign, consider surfactants, sodium, boron, conductivity/salinity, pH, COD/TSS and microbial indicators.

For future irrigation studies only, WRC TT 746/17 discusses EC, SAR, E. coli and pH as a minimum analysis, with boron, COD, oil/grease, suspended solids, total inorganic nitrogen and total phosphorus in a fuller irrigation analysis. Those are irrigation-specific study inputs; they are not toilet-flush acceptance criteria and must not be copied into that use profile.

### Process indicators vs quality assessments

Temperature, pH, conductivity and turbidity are useful process variables. None by itself establishes microbial safety or use-specific compliance. The control system may use an admitted process signal for an interlock, but Mycelix #6100 binds raw results to the exact method/profile and records the derived assessment separately.

## 7. QA/QC, method and custody

The qualified lab establishes the QA/QC plan. Consider field/equipment blanks, duplicates, control/reference material and blind sample identities as appropriate for the method. Their frequency, acceptance criteria and corrective action are method-specific.

Each sample record must preserve:
- unique sample ID and parent event/batch ID;
- source profile and source point code;
- collection mode and time window;
- sampler identity/competence evidence as required;
- chain-of-custody and transport/receipt;
- method ID and revision;
- laboratory and accreditation scope where relevant;
- analyte/measurand ID, value, canonical unit and raw/processed status;
- detection/quantification limit and qualifier;
- instrument/channel, calibration/check record, applicability and uncertainty when relevant;
- collection, acquisition, valid and receipt time semantics separately;
- preservation, temperature and holding-time status as specified by lab;
- QC flags and rights/consent/retention scope.

Missing method/custody/calibration information remains missing. Below-detection/censored values are not converted to zero. Rejected samples remain in the campaign audit trail, with an exclusion reason; they are not silently removed.

Mycelix owns the sample/observation and derived assessment records. Symtropy records reference the exact Mycelix observation IDs/digests and the study-event identity; no parallel laboratory record authority is introduced.

## 8. Concentration, hydraulic load and joint envelope

Treatment design needs both concentration and mass loading.

For each analyte with a suitable method, derive load only when concentration, event volume/flow and time alignment are admitted:

`event_load = concentration × measured_event_volume`

The derived value must preserve units, transformation/method revision, uncertainty and source observation digests.

Report:
- per-event concentration and mass load;
- event flow/volume and duration;
- median and predeclared quantiles with uncertainty intervals;
- within-site and between-site variability;
- maximum/outlier values with investigation status;
- joint covariance/dependence for critical feed variables;
- temperature and time-dependent changes;
- censored, missing and QC-rejected observations.

Do not construct an impossible synthetic worst case by combining the maximum of every independent variable. Use observed joint batches and clearly marked synthetic challenge profiles.

## 9. Feed-envelope construction and treatment selection

A provisional feed envelope is source/profile/configuration specific. It records:
- sample/observation population and digests;
- sample/cluster/event counts and represented strata;
- quantile/uncertainty method;
- joint load/flow/temperature conditions;
- excluded/missing/censored population;
- out-of-envelope trigger;
- evidence class and review status.

The first feed envelope is **provisional** until the site/source strata and QA/QC requirements are met and the intended process owner reviews it. Each treatment module claims only the feed envelope actually tested. A module outside that envelope must reject/divert or obtain new qualification.

A candidate train is compared on matched aliquots from the same batch, with process settings/configuration frozen. Compare:
- quality at each stage and final use point;
- absolute outlet values and distributions, not only percentage removal;
- microbial and chemical results separately;
- useful-volume recovery, reject and residual load;
- energy, pressure/headloss, cleaning, fouling/breakthrough and restart;
- maintenance hours, consumables and cost per independently verified useful m³;
- out-of-envelope frequency and safe rejection.

A higher mean COD removal does not compensate for microbial failure, high reject volume, frequent maintenance or loss of service.

## 10. Predeclared statistical stopping rule

The pilot must produce variance estimates before the main campaign sample count is fixed. Main campaign stopping is allowed only when:
1. all predeclared source/site/season strata are represented;
2. sample count follows the preregistered cluster-aware design;
3. primary feed quantiles reach their predeclared uncertainty/precision target;
4. QA/QC and method coverage meet the lab's acceptance criteria;
5. no unrepresented high-impact source/operating stratum remains;
6. any exclusions, failures and censoring are reported.

If not achieved, the envelope is provisional or Indeterminate. Do not call the sample set representative merely because it has a large row count.

## 11. Deliverables

1. Qualified and signed sampling/analysis plan and site permission record.
2. Coded source/strata register with inclusions/exclusions and privacy review.
3. Laboratory analyte/method matrix and QA/QC/custody plan.
4. Pilot variance and cluster-aware sample-size/quantile-precision analysis.
5. Source event/sample manifest with exact Mycelix observation references.
6. Reproducible distribution/load report with uncertainty, dependence and missingness.
7. Versioned provisional/admitted feed-envelope object for each source profile.
8. Matched-batch treatment-train dataset and frozen input manifest for #5475.

## 12. Nonclaims

No greywater has been sampled or collected by this protocol. No site, household, laboratory, sampling procedure or sample count is approved. No numeric quality threshold, treatment train, pathogen barrier, installation, reuse, potable claim, SANS compliance, public-health determination or household deployment is qualified.