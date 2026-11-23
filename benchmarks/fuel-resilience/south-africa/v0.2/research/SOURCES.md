# SAFRB source register and evidence caveats

Research cut: **23 November 2026**. Values below are public-source claims, not certified current stocks or an operator-approved national balance.

| Source ID | Source | Public observation used | Limitation |
|---|---|---|---|
| DEE-2026-MARKET-REPORT | [Department of Electricity and Energy — Overview of the Petrol and Diesel Market 2015–2024, fourth edition](https://www.dmre.gov.za/LinkClick.aspx?fileticket=ExxZyYuQywM%3D&portalid=0) | 2024 D50 approximately 12.0bn L and D500 approximately 273m L; the report discusses domestic diesel supply shortfall and imports. | The report says the energy-statistics series has not yet received Stats SA SASQAF accreditation. Grade values are rounded and do not reconcile exactly to the national aggregate figure shown in the report. |
| DEE-2026-NATIONAL-SUPPLY-DEMAND-GRAPH | Same DEE report, national supply/demand chart | Approximate 2024 diesel consumption around 11.8bn L. | Chart read/rounded value; not a substitute for an underlying machine-readable series. |
| STATSSA-MBALO-MAY-2026 | [Stats SA Mbalo Brief, May 2026](https://www.statssa.gov.za/publications/MbaloBrief/MbaloBriefMay2026.pdf) | Repeats 11.8bn L diesel consumption in 2024. | The narrative cites a secondary source; treat as an aggregate comparator, not a grade allocation. |
| SA-ENERGY-TRADE-Q2-2026 | [South African Energy Trade Report — Quarter 2, 2026](https://fuelsindustry.org.za/wp-content/uploads/2026/10/SOUTH-AFRICAN-ENERGY-TRADE-REPORT-QUARTER-2-2026.pdf) | Diesel imports 3.3bn L in Q2 2026; origin share: India 27%, US 21%, Oman 15%, Togo 9%, Netherlands 5%; source-reported origin litres sum to 2.569bn L. | Customs trade flow, not net usable stock, net imports, inventory, consumption or a stable annual mix. The five origin litres differ by 28m L from 77% of the rounded 3.3bn L total; preserve reported shares and litres separately. |
| SA-ENERGY-TRADE-2024 | [South African Energy Trade Report 2024](https://www.dmre.gov.za/Portals/0/Resources/Publications/Reports/Energy%20Sector%20Reports/SA%20Energy%20Trade%20Report/2024-South-African-Energy-Trade-Report.pdf) | Report sections show different diesel export totals: 790m L in one aggregate table and 276m L in a destination discussion. | Explicit unresolved source conflict. Preserve both statements and request the underlying SARS data / definition reconciliation. |
| CABINET-STOCKS-JUL-2026 | [Cabinet statement, 1 July 2026](https://www.gov.za/news/cabinet-statements/statement-cabinet-meeting-1-july-2026-02-jul-2026) | Draft policy proposes 60 days of net-import cover, rising to 90 days. | Policy proposal / consultation target, not evidence of actual reserve holdings. |
| IEA-OMR-NOV-2026 | [IEA Oil Market Report, November 2026](https://www.iea.org/reports/oil-market-report-november-2026) | Global oil market projected surplus in Q4 2026 while diesel and other refined product supply remains tight; on-land inventories remain low. | Global aggregate oil balance is not a diesel-product balance and is not a South African inventory measurement. |
| NIST-OT-IPD-2026 | [NIST SP 800-82 Rev. 4 Initial Public Draft](https://csrc.nist.gov/pubs/sp/800/82/r4/ipd) | Draft OT security guidance; published 21 September 2026; comments due 30 November 2026. | Draft, not final standard. |

## National usable-stock coverage is not established

The public sources reviewed for this benchmark do **not** establish a single consolidated, independently verified national total of immediately usable South African diesel. In particular:

- Strategic crude inventory is crude feedstock, not refined diesel available to end users.
- Pipeline line-fill is not automatically a withdrawable reserve or local depot stock.
- Commercial stock reports are not a complete, independently reconciled national inventory.
- Cargo nominations, vessels in transit, customs imports and future commitments are not current usable stock.
- A draft 60/90-day stock-cover policy target is not evidence that such stock is currently held.

Therefore the benchmark must not calculate or publish national "days of diesel remaining" from these incomplete categories. That calculation requires a time-aligned opening stock by grade, terminal/tank boundary, quality release and physical accessibility, plus domestic production, discharges, transfers, losses and demand on a consistent accounting basis. Until that evidence exists, report the stock total as **unknown / not established**, not as zero and not as an imputed estimate.

## Mandatory evidence semantics

- **Reported:** a source states a value. No measurement claim is implied.
- **Measured:** an instrument or verified measurement process captured the quantity. Attach raw artefact digest, instrument identity, calibration status, time, location, grade and measurement uncertainty.
- **Derived:** computed from identified source observations and a frozen transformation.
- **Forecast:** model output; never silently upgraded to observed.
- **Scenario:** synthetic stress input; never presented as an actual national state.
- **Verified usable inventory:** measured, digest-bound, quality released, physically accessible and within a named tank/terminal boundary at a named time.
- **Committed cargo:** contract or nomination state, not physical receipt.
- **Cargo in transit:** vessel/transport status, not tank inventory.
- **Discharged:** cargo transfer to a named receiving boundary supported by evidence; quality release and accessibility remain separate gates.
- **Pipeline line-fill:** material occupying the pipeline; not automatically recoverable reserve or local depot inventory.

When sources conflict, preserve source identity, publication version, date, quantity, unit, geographic boundary, product grade, accounting basis and uncertainty. Reconciliation must be a documented transformation with provenance, not a quiet overwrite.
