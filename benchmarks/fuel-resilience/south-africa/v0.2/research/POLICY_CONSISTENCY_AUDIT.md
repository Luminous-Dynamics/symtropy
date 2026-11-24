# Draft Strategic Petroleum Stocks Policy 2026 — source consistency audit

**Source:** Government Gazette 54975, Notice R.7691, 9 July 2026  
**Official source:** https://www.gov.za/sites/default/files/gcis_document/202607/54975rg12021gon7691.pdf  
**Comment notice:** https://www.gov.za/documents/notices/mineral-and-petroleum-resources-publication-draft-strategic-petroleum-stock  
**Consultation deadline:** 9 August 2026 (closed)  
**Research cut:** 24 November 2026  
**Status:** source audit of a draft policy, not legal advice or a determination of the final policy.

## 1. Stockholding targets conflict inside the draft

The same gazetted document gives materially different stock-cover values:

| Location in the draft | State / government | Private industry | Interpretation |
|---|---:|---:|---|
| Executive summary, policy stockholding description | 90 days of net imports | 14 days of refined products | Government reserve primarily crude at Saldanha Bay; private stock described as refined products |
| Detailed policy provisions, section 9 table | 60 days | 21 days | State stock managed by SANPC with 70% crude / 30% key refined products; private licensed wholesalers/importers with 70% crude / 30% finished products |
| Cabinet statement, 1 July 2026 | 60 days of net imports, phased to 90 days long term | Not specified in that statement | Government announcement when approving publication of the draft |

The detailed section 9 table gives a total national cover of "60+ days". The executive summary instead describes a 90-day state reserve plus 14 days of private refined-product stocks. These are different financing, product-mix and resilience designs. SAFRB records each as a separate **reported draft-policy variant**; no target is selected, averaged or treated as current inventory.

The government's official public-comment notice set 9 August 2026 as the deadline, which has passed. This document therefore supports a retrospective source audit, not a live consultation response.

## 2. Crude reserve cover is not immediately usable diesel cover

The detailed table uses a 70% crude / 30% key-refined-product mix. Crude feedstock cannot be counted as immediately usable D50 or D500:
- crude needs a suitable operating refinery, compatible crude slate, available refining capacity and time;
- refined output still requires transfer, storage, quality release and distribution;
- refinery, berth, pipeline or power outages can prevent the conversion and movement;
- the resulting grade and specification must match the demand zone.

SAFRB must therefore maintain separate crude, intermediate/feedstock, D50, D500 and other refined-product states. Crude can contribute to a future supply pathway only through an explicit, constrained conversion chain; it is not an immediate diesel stock.

The draft says about 60% of petroleum-product consumption is in inland regions. This makes coast-to-inland capacity and delivery deadlines relevant, but it does not provide a complete machine-readable topology or evidence-qualified current stock map.

## 3. Import and inland lead-time claims are not calibrated distributions

The policy states:
- 21–42 days for imports to reach South African ports;
- another 10–14 days for offloading, refining and transport from coastal facilities to inland markets.

These figures can be retained as **reported policy assumptions** for scenario design. They are not a historical vessel ETA dataset or a validated probability distribution. Calibration requires dated bills of lading, vessel arrival/departure, berth, discharge, tank receipt, quality release, pipeline and terminal-transfer timestamps with consistent event definitions.

Do not treat the arithmetic sum of those ranges as a guaranteed end-to-end lead time: crude vs finished product, refinery route, vessel/cargo, port, pipeline and inland destination materially change the path.

## 4. Pipeline capacity and replenishment claims require independent qualification

The draft identifies the former 700 km Durban–Johannesburg Pipeline (DJP), says it reached end of economic life in 2018 and was replaced by the Multi-Product Pipeline (MPP), commissioned in 2012 to transport petrol and later diesel. It asserts that the MPP has additional capacity above normal commercial throughput and sufficient capacity for market demand, disruption shortfall and strategic-stock replenishment.

The draft further says strategic stocks should be replenished within 60 days and commercial stocks within 45 days. These are policy-document assertions, not a source-pinned hydraulic capacity curve or operator qualification package.

Before SAFRB turns them into network constraints, it needs:
- grade/batch sequencing and interface/contamination constraints;
- rated and usable flow under named operating conditions;
- pump-station capacity, pressure limits and energy supply;
- line-fill, transit time, leakage/measurement uncertainty and maintenance state;
- competing normal commercial throughput and emergency allocation;
- destination terminal/tank receipt capacity and quality release;
- common-cause outages across power, controls, communications and shared stations.

A claim of "sufficient capacity" is not a capacity value. Do not assign numerical throughput or route resilience from this prose alone.

## 5. Section 7.1 contains material unit and economic arithmetic contradictions

The introduction (section 3) says South Africa uses about **27 billion litres** of petroleum products annually. Section 7.1 instead says approximately **30 million litres** per year. Taken literally, those values differ by a factor of 900.

Section 7.1 then assumes a 20% reduction in availability over two weeks, but describes this as about 4% of annual consumption and 1.2 million litres. The arithmetic does not match the stated 20% shortfall:

- At the section's stated 30 million L/year, 20% of 14/365 annual consumption is approximately **230,137 L**, not 1.2 million L.
- At the introduction's 27 billion L/year, the same 20% / 14-day shortfall is approximately **207.1 million L**.
- 1.2 million L is approximately 4% of 30 million L; it resembles the full two-week consumption at that 30-million-L annual basis, not a 20% shortfall.

The calculation (1.2	ext{ million L} 	imes R20/L = R24	ext{ million}) is arithmetically consistent with the stated 1.2-million-L volume, but that volume is not consistent with the document's stated two-week 20% reduction.

The sectoral loss bullets list approximately R8m logistics, R6m manufacturing, R2m agriculture, R4bn consumer/retail and another R4bn indirect GDP. These items sum to approximately **R8.016 billion**, not the document's stated R24m of sectoral/indirect loss. The document then states total cost as R24m direct fuel value plus R24m sectoral/indirect losses, giving R48m. This does not reconcile to the listed sector figures.

These economic values must be quarantined from SAFRB calibration. Do not repair them by guessing which unit or decimal point was intended.

## 6. Draft operational provisions to model only as labelled policy scenarios

Other draft provisions include:
- refined-product rotation at least every three months;
- a stated 10-million-barrel "current shortfall" to replenish sold or rotated stocks, without enough precision in the cited passage to treat this as a verified diesel deficit;
- Level 1: 20% loss of national refined-product supply for more than 14 days;
- Level 2: 40% loss of national supply with depletion of the 21-day commercial buffer;
- Level 3: severe shock or failure of the import value chain affecting more than 50% of supply.

These can form a policy-scenario track, not factual operational thresholds. The product scope, denominator, baseline period, geography, responsibility, authority and measurement method need precise definitions before they can be evaluated against live data.

## 7. SAFRB requirements arising from this audit

1. Preserve executive-summary and detailed-table stock-cover targets as distinct policy variants.
2. Preserve crude and each finished-product grade separately.
3. Never convert crude cover into usable diesel without an explicit, qualified refining and logistics path.
4. Represent 21–42 day import and 10–14 day downstream lead-time claims as source-labelled scenario ranges until calibrated from event history.
5. Treat MPP capacity and 60/45-day replenishment statements as hypotheses to test, not trusted numerical constraints.
6. Add draft Level 1/2/3 trigger scenarios without labelling them current legal or operational triggers.
7. Quarantine section 7.1 annual-demand and economic-impact values from model training and scorecards.
8. Keep physical quantity, policy target, public statement, scenario assumption and current verified inventory as separate evidence classes.

## Sources

- Government Gazette 54975, Notice R.7691, Draft Strategic Petroleum Stocks Policy of South Africa, 9 July 2026: https://www.gov.za/sites/default/files/gcis_document/202607/54975rg12021gon7691.pdf
- Official public-comment notice and 9 August 2026 deadline: https://www.gov.za/documents/notices/mineral-and-petroleum-resources-publication-draft-strategic-petroleum-stock
- Cabinet statement, 1 July 2026: https://www.gov.za/news/cabinet-statements/full-statement-cabinet-meeting-1-july-2026-03-jul-2026

**Claim ceiling:** this audits internal consistency of a draft policy. It does not estimate actual reserve holdings, independently validate economic losses, qualify MPP hydraulic capacity, or establish the legal status of a final policy.


## 8. Latest public status located by 24 November 2026

S&P Global Energy reported on **15 October 2026** that the public consultation process had concluded, Cabinet approval was still awaited, and CEF leadership expected a decision by year-end. That is the latest policy-status report identified in this research pass; it does not establish that Cabinet has since approved the policy or that a binding rule has commenced. The stock-cover requirements in this audit therefore remain draft-policy claims until a final approved/gazetted instrument is found.

S&P also reported SANPC leadership's attributed estimate of a **300,000 barrels/day** oil-products shortfall following refinery closures. This is a source-attributed industry estimate; it is not a diesel-only deficit, a verified volume of missing stock, or a calibrated annual import requirement. Preserve its source, scope and unit separately.

The South African Reserve Bank's 2026 research note independently reports that refining capacity has halved over the prior decade and imported refined products now supply more than half of domestic fuel demand. This is evidence of **structural import exposure**, not proof of an immediate shortage or a measurement of current stocks.

Conversely, the Minister stated in the National Assembly on 19 May 2026 that supply had been secured and no near-term shortage was expected. This statement is not a stock audit, but it illustrates why SAFRB must distinguish:
- current short-term market availability;
- structural dependence on imported refined products;
- physical reserve adequacy;
- location-specific delivered service under disruption scenarios.

These categories must not be collapsed into one binary "shortage" label.


## 9. New official evidence: strategic stock statements and import structure

The National Assembly Hansard of 25 March 2026 records the Minister's statement that the Strategic Fuel Fund held **eight million barrels** of strategic fuel stock, not being used pending a real crisis. In the same answer, the Minister said approximately **60% of processed products were imported** and 40% processed by domestic refineries and Sasol.

This is an official, dated parliamentary statement, not an independently verified inventory. The answer does not specify crude/refined split, diesel grades, tank locations, quality release, ownership, physical accessibility, measurement date or withdrawal capacity. It must not be turned into diesel litres or days of cover. Keep the 8-million-barrel claim separate from crude imports, refined product stocks, pipeline line-fill and grade-specific usable diesel.

The SARB's 8 September 2026 note later reports that imported refined products supply more than half of domestic fuel demand and that refining capacity has halved over the prior decade. This broadly supports a structural import-dependence claim, but is not directly comparable to the Minister's dated 60/40 statement without harmonising product scope, date and denominator.

## 10. Island View Precinct market-access and capacity-rights evidence

Parliament's 13 August 2026 statement identifies the Island View Precinct (IVP) in Durban as a strategically important liquid-bulk/petroleum terminal complex with 10 port berths and says the majority of South African fuel is imported through it. It reports a supplementary directive for 10% of total IVP capacity to be allocated to new market participants, around 13% of operator capacity committed under existing terminal agreements, and a later-phase objective of approximately 27.8% Black participation through new liquid-bulk terminal development.

These are distinct capacity-access statements, not measured throughput. Do not sum 10%, 13% and 27.8% or treat them as interchangeable quantities. The source itself notes the importance of timely vessel/berth allocation and access to fuel distribution networks, including pipelines.

SAFRB should model legal/contractual entitlement, provider commitment, reservation, scheduling, physical operability and delivered throughput as distinct states. A capacity entitlement is not a physical delivery.

## 11. Domestic capacity restoration is an option, not current production

CEF's 23 September 2026 presentation reports that the Mossel Bay GTL refinery and FA Platform had been offline/on preservation since November 2020. It states the reinstatement business case was completed in August 2026 and describes a phase 1 target of approximately 18,000 barrels/day at an estimated R5.8bn, followed by a phase 2 target of approximately 46,000 barrels/day at an additional estimated R8.5bn. The source wording does not establish whether 46,000 barrels/day is incremental or total phase-2 capacity; SAFRB preserves that ambiguity and does not add the two targets.

CEF also reports material PetroSA liquidity/operational constraints and tighter product-supplier financing requirements. These are project-execution dependencies. No target output enters the current supply balance before funding, feedstock, engineering, construction, commissioning and measured sustained product output are independently evidenced.
