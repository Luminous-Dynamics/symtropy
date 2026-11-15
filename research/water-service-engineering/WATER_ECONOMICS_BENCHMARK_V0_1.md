# Water Economics Benchmark v0.1 — cost per verified useful water service

**Date:** 15 November 2026  
**Parent:** [WATER-ECONOMICS-001 #5425](https://github.com/Luminous-Dynamics/symtropy/issues/5425)  
**Implementation:** WaterLoop physical T0 evaluator, cases 026–033  
**Evidence class:** synthetic-only  
**Claim ceiling:** cost/bill/service arithmetic and admission semantics; no household affordability or lifecycle performance qualification

## 1. The correct denominator

A water system is not cheap because its tank is inexpensive, its treatment train advertises 360 L/day, or a dashboard predicts 45% lower mains use. The household purchases and maintains a system to secure a service, not a nominal output.

The benchmark compares:

`monthly and lifecycle household cost / verified useful m³ delivered to a declared end use`

alongside service continuity, actual utility bill, payer, maintenance burden and resident affordability. It retains separate results for:
- gross local-system cost;
- baseline and actual municipal utility bills;
- incremental household spend against a matched baseline;
- cost per verified useful m³;
- cost per critical person-day actually served;
- water-spend burden, but only where resident-authorised income evidence is admitted.

A lower unit cost is not a win when critical service floors fail. A service-equivalent architecture is not affordable by default. An absent income record remains unknown; no universal affordability ratio is imposed by the evaluator.

## 2. Current South African service and tariff context

Stats SA's General Household Survey 2025 reports 87.4% of households had piped water access in the dwelling, yard or communal tap, but 37.6% reported interruptions lasting more than two days at a time or 15 days in total over the prior year; the Gauteng result was 32.5%. These survey statistics demonstrate that access and reliable service are distinct; they do not describe an individual dwelling or prove service quality at a point of use.

- GHS 2025 publication and tables: https://www.statssa.gov.za/?PPN=P0318&page_id=1854
- GHS 2025 presentation (interruptions): https://www.statssa.gov.za/publications/P0318/GHS%202025%20Presentation.pdf

The 2024 South African water tariff norms require domestic water tariffs to balance financial viability, affordability of basic supply and incentives against wasteful consumption. The norms describe a first block of at least 6 kL/consumer/month at the lowest amount necessary to sustain the service/free-basic-water arrangements. Actual municipality, customer class, service charges, block definitions, rebates and bill periods still matter.

- Government Gazette 50716, 27 May 2024: https://www.dws.gov.za/Documents/Gazettes/Revised%20Norms%20and%20Standard%20in%20repect%20of%20tariffs%20for%20water%20services%20Gazette%20No%2050716%20,%2027%20May%202024.pdf

The Water Research Commission's 2024 position paper estimates 60–80% of municipal water-supply costs are fixed, and notes that fixed tariffs can be regressive for low-income households. This matters in two directions: reducing a household's volumetric draw may not reduce the provider's fixed cost, and eliminating a fixed charge from the household bill is not an honest estimate of savings.

- WRC Report 1250, *Innovations in retail water services tariffs*: https://search.wrc.org.za/reports/9eebffa2-3bfa-4a8c-89d2-1be77e24b671

## 3. Market comparators and interpretation

### Hydraloop H300

The 2026 H300 product page declares 360 L/day potential treatment capacity and 280 kWh/year average electricity consumption; it says the system targets up to four people and claims up to 45% water-use reduction. The July 2026 owner/installation manual lists a 100 L internal volume, 360 L/day nominal capacity, 280 kWh/year average consumption and pump delivery up to 2.4 bar / 8 L/min.

At the full declared throughput, the ratio is:

`280 kWh/year ÷ (0.36 m³/day × 365 days/year) = 2.13 kWh/nominal m³`

This ratio is only a calculation from manufacturer-declared annual energy and full nominal capacity. It is **not** measured energy per actual useful m³. A real comparison requires metered electricity and use-qualified delivered volume over the same time window, including standby, purge, overflow, backup mains use and auxiliary loads.

The 2024 H300 datasheet differs from the July 2026 manual on storage volume/energy declarations; exact document revision is therefore part of the benchmark identity. Do not mix older revisions or select whichever figure makes a comparison look better.

- Current H300 page: https://www.hydraloop.com/h300
- July 2026 installation manual: https://static1.squarespace.com/static/6809d6b186796545157fbf2f/t/6a55b27199175d688f397fb5/Hydraloop%2Binstallation%2Bmanual%2B-%2BJuly-2026.pdf
- Older 2024 product sheet: https://info.hydraloop.com/wp-content/uploads/2024/07/20240708-Product-Data-Sheet-%E2%80%93-Hydraloop-H300.pdf

### Nature-based treatment

A July 2026 full-scale NTSW study reports three installations (household and schools), 2.5 years of water-quality monitoring and a four-year compost accumulation assessment. It reports electricity use of 0.05–0.17 kWh/m³ and 150–1,500 kg/year of stabilized dry matter. This provides an important low-energy passive/nature-based comparator, but it is not directly rankable against a household greywater appliance without matched feed, use, boundary, land, pumping, recirculation, quality and useful-delivery data.

- Halicki et al. 2026, *Journal of Environmental Management* 412, 130241: https://doi.org/10.1016/j.jenvman.2026.130241

### JoJo household water monitoring

JoJo's current South African catalogue lists a tank-level IoT monitor at R4,999 and a water meter at R3,499. Its product page describes tank-level/flow visibility and abnormal-pattern alerts. The warranty page lists 10 years for roto-moulded tanks and one year for pumps/IoT products. These are public catalogue/product and warranty references only; they are not evidence of actual leak-detection precision, avoided losses, service improvement or household savings.

- Product catalogue: https://www.jojo.co.za/product-category/buy-water-monitoring-devices/
- Device functions: https://www.jojo.co.za/jojo-monitor-devices/
- Warranties/certificates: https://www.jojo.co.za/warranties-certificates/

## 4. Frozen T0 cost boundary

This first evaluator does not calculate municipal tariff blocks or price a product. It consumes versioned admitted bills and cost components from their respective owners.

The monthly-equivalent gross local-system cost is:

`C_system = C_capex_equivalent + C_maintenance + (E_kWh × tariff_E) + C_consumables + C_external_water + C_residuals`

The tariff owner supplies the matched baseline and actual municipal bill using the exact tariff revision/customer class/billing period. The evaluator reports the fixed charge before and after separately and keeps it inside the total bill.

`ΔC_household = C_system + Bill_actual - Bill_baseline`

`C_gross_per_m3 = C_system / verified_useful_m3`

`C_incremental_per_m3 = ΔC_household / verified_useful_m3`

`Water_spend_burden = (C_system + Bill_actual) / resident_authorised_disposable_income`

`C_per_critical_person_day = C_system / admitted_critical_person_days_delivered`

If volume or person-days is zero, the corresponding ratio is Undefined. If tariff evidence is unadmitted or its billing-period duration does not match the cost period, bill delta, incremental cost and whole-system cost comparison are Indeterminate. The evaluator does not annualise, prorate or silently resample bills. If cost ledger is unadmitted, total system cost is Indeterminate. If income is unavailable or not authorised, the affordability burden is Indeterminate while an otherwise admitted cost/service comparison and unit-cost metrics remain reportable. Cost efficiency and household affordability are distinct outputs; one is not a proxy for the other.

The evaluator requires:
- tariff billing-period duration to match the system-cost period exactly within the declared numeric tolerance;
- explicit ZAR currency, price date, region and 30-day period;
- cost-use profile identity and service-use profile identity;
- cost-ledger, tariff, income and service evidence admission flags;
- versioned tariff schedule and bill-method IDs;
- fixed-charge values within admitted total bills;
- separately stated cost components;
- service-floor state and actual critical person-days delivered.

T0 treats the supplied monthly capex equivalent as an already calculated result from the whole-life cost owner; it does not select a discount rate or annualisation policy. Future T1 must carry exact installed quote/asset life/replacement/discount/tax/finance assumptions from Sol-Atlas #816.

## 5. T0 corpus cases 026–034

| Case | Scenario | Expected safeguard |
|---|---|---|
| 026 | Complete admitted 30-day cost/tariff/service ledger | Calculate gross cost, bill delta, cost ratios and 6% synthetic spend burden |
| 027 | Rising-block bill delta with fixed charges retained | Use actual bill difference, never average price × reused litres |
| 028 | Tariff evidence not admitted | Keep gross system arithmetic, no bill-savings credit |
| 029 | Zero useful volume | Unit cost remains Undefined; no division by zero |
| 030 | Cost ledger not admitted | Do not understate the system by treating unknown cost as zero |
| 031 | Disposable-income evidence unavailable | No invented affordability ratio |
| 032 | Cost/service intended-use mismatch | No volume credit or cost-per-useful-m³ claim |
| 033 | Lower-cost configuration misses declared essential service floor | No economic-preference eligibility |
| 034 | Utility bill covers a different number of days than the system-cost period | No bill-delta or whole-system cost comparison until periods match |

Cases are synthetic and only verify arithmetic/admission semantics. Their numeric inputs are not product quotes, tariffs, measured households or field observations.

## 6. Industry comparison protocol

At identical household, intended-use class, resident demand, tariff schedule, outage trace, service floor, price date, horizon and payer boundary compare:
1. municipal supply only;
2. supply + gravity storage;
3. supply + booster pump;
4. storage + pump + local backup;
5. greywater appliance, using measured permitted reuse;
6. passive/nature-based reuse, including actual land and maintenance cost;
7. community/alternative-delivery option;
8. WaterLoop hybrid;
9. current no-build/current-practice baseline.

Report component-cost distributions and service outcomes, not one “best product” score. Treat the following as separate:
- manufacturer's declared nameplate capacity;
- commissioned system capacity;
- actual measured useful volume at an end-use boundary;
- volume credited for a declared use after quality admission;
- volume displaced from municipal supply;
- billed reduction under actual tariff blocks;
- reduction in household spend;
- reduction in system/provider cost.

These quantities are not interchangeable.

## 7. Physical T1 evidence gate

Before any installed cost, household affordability, savings or lifecycle superiority claim:
- exact vendor quote and installed/commissioned bill of materials;
- installation labour, professional fees, plumbing/electrical works and commissioning;
- matched municipal tariff schedule, account class, blocks, fixed charges, rebates/subsidies and actual bill;
- admitted electricity meter and use-specific water/flow measurements;
- accepted source/use-quality assessment via Mycelix #6100;
- source, delivered, overflow, purge, reject and residual volumes;
- maintenance labour, replacement parts/media, downtime, repair time and consumable logistics;
- payer/beneficiary identity (owner, tenant, utility, community/public fund);
- local plumbing/electrical/water-quality review and exact site configuration.

No model-generated income, inferred household traits, credit score or automatic subsidy eligibility is part of this benchmark. Disposable income is an optional resident-authorised numeric input; it must not be inferred from protected or sensitive characteristics.

## 8. Nonclaims

This synthetic protocol does not qualify any current tariff, household bill, product price, installation, water-quality result, household affordability, savings, payback, energy intensity, public benefit or lifecycle advantage. It authorises no procurement, tariff, subsidy, loan, tenancy, installation or utility action.


### Certified capacity is not the same as nominal treatment throughput

An additional official comparison check matters here. The NSF official listing, current as of **14 November 2026**, lists **Hydraloop H350/H300 at 52.8 US gallons/day (about 200 L/day), Class R**, with the classification shown as “Laundry & Bathing”; H600 is listed at 260 US gallons/day. The H300 product page separately declares 360 L/day cleaning capacity and up to 45% water-use reduction.

Those two capacity figures are not interchangeable. The 360 L/day is the product-page nominal treatment-capacity declaration; 52.8 US gallons/day is the standard listing's rated capacity for the certified Class R entry. Do not collapse them into one throughput value or infer a 360 L/day NSF certification from the page's separate headline number. Record both exact source, revision, unit, classification and claim scope; seek the manufacturer's underlying NSF test/technical documentation before treating them as matched measures.

Using the annual 280 kWh declaration:
- against the product-page 360 L/day nominal capacity: approximately **2.13 kWh per nominal m³**;
- against the NSF listing's 52.8 US gallons/day (~0.200 m³/day): approximately **3.84 kWh per listed-rated-capacity m³**.

Both are arithmetic normalizations combining different public declarations, **not measured energy per actual useful m³** and not proof that either denominator represents normal in-use throughput. The benchmark's answer is to measure electricity and quality-admitted end-use delivery on the same time boundary.

The 2026 installation manual also explicitly says Hydraloop output is non-potable and warns against drinking use and cross-connections. NSF/ANSI 350 Class R is therefore not a drinking-water claim. The local benchmark must preserve separate use classes and must not compare this output with a potable-water service.

Sources:
- NSF Official Listings, NSF/ANSI 350, current listing: https://info.nsf.org/Certified/Wastewater/Listings.asp?Company=C0427542&Standard=350
- Hydraloop H300 product page: https://www.hydraloop.com/h300
- Hydraloop H300 2026 installation manual: https://static1.squarespace.com/static/6809d6b186796545157fbf2f/t/69e5a64534faab28389f1f69/1776658009052/Hydraloop%2BInstallation%2BPack%2B2026.pdf

### Stats SA interruption denominators

Keep two different GHS 2025 indicators separate:
- **56.8%** reported at least one water interruption during 2025; 43.2% reported none.
- **37.6%** met the report's longer-duration functionality threshold (interruptions lasting at least two days at a time or more than 15 days in total); Gauteng was 32.5%.

The first is any interruption; the second is the longer-duration threshold. Neither is an address-level measured outage series.

Sources:
- Stats SA GHS 2025, Figure 9.5 and 9.6: https://www.statssa.gov.za/publications/P0318/P03182025.pdf
- Stats SA GHS 2025 presentation: https://www.statssa.gov.za/publications/P0318/GHS%202025%20Presentation.pdf
