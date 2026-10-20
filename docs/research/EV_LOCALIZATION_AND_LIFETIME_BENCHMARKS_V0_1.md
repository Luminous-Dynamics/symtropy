# EV-LOCALIZATION-001 — Route economics and whole-life benchmark protocol v0.1

**Frozen research date:** 20 October 2026  
**Scope:** public online evidence plus clearly labelled synthetic sensitivity inputs. No supplier, manufacturer, distributor, customs broker or other organization was contacted; no quote was requested.

## Why this is a paradigm shift

Do not frame the engineering choice as “import the car” versus “build it locally”. Compare the transformations required to deliver the same customer service at the same safety and warranty boundary. A complete import can avoid production tooling but incur freight and vehicle duty. CKD assembly can add local labor and investment while still importing most value. Deeper localization can create local capability and reduce some imports, but may initially increase tooling, scrap, supplier qualification and process-control costs. A shared platform can amortize real common tooling, but only across vehicles that truly share that investment.

The new Rust executable is tools/ev-passenger-cost-frontier/src/bin/ev_localization_frontier.rs. Its fixture compares complete imports, CKD assembly, deeper local manufacturing and shared-platform carryover at cumulative-volume sensitivities of 1,000, 10,000, 50,000 and 100,000 units. All component prices, yields, factory conversion, warranty reserves and programme investment in that fixture are **synthetic arithmetic inputs**. They are not quotations, supplier costs, a plant plan or a forecast.

## Economic boundary and calculation

The market baseline is the current public BYD South Africa Dolphin Surf entry-offer page, recorded as R341,900 on 20 October 2026: https://www.byd.co.za/offers/dolphin-surf . The initial stretch target is 10% below the frozen observation, R307,710. Refresh the price and confirm exact trim, included costs, warranty and equipment before head-to-head claims.

Using the existing EV cost-frontier fixture's explicit assumptions (15% retail VAT, an 8% distributor share of ex-VAT retail, and 12% OEM gross margin), the target implies about **R216,628 maximum cost of goods**. The 8% and 12% values are modelling assumptions, not researched market averages or a commercial offer. This is a ceiling to test; it is not evidence that any route can reach it.

For retail price P, undercut fraction u, VAT fraction v, distributor share d and target OEM gross margin g:

- P_target = P × (1 − u)
- if retail includes VAT, P_exVAT = P_target / (1 + v)
- OEM_net_revenue = P_exVAT × (1 − d)
- COGS_max = OEM_net_revenue × (1 − g)

Each import line separately records purchase value, freight/insurance, candidate tariff classification, duty-rate range, classification evidence and origin evidence. The current simplified customs-value basis is explicitly an assumption that purchase value plus freight/insurance is the base to which the candidate duty rate applies. A real customs valuation may have legally defined adjustments; validate it before commercial use. The model intentionally does not assume a preferential origin rate.

At the scenario level it reports, without double-counting:
- imported customs-value proxy, assumed duty, landed import subtotal and a separate import-VAT cash-exposure estimate;
- domestic component spend and factory conversion, adjusted for declared first-pass manufacturing yield;
- warranty/service reserve and per-vehicle tooling/programme amortization;
- both COGS frontiers assuming import VAT is recoverable and assuming it is not recoverable. If recovery status is unknown, there is no single selected frontier.
- the highest cost allowed by the market-price waterfall, tariff/cost evidence gates and explicit evidence gaps.

The VAT cash exposure is a sensitivity, not an assertion of the legally correct import VAT base or of recoverability. Tax treatment, customs classification and trade-preference eligibility must be independently validated. The route model also does not calculate a statutory local-content percentage. Its local-spend proxy is an internal management measure only; it cannot establish compliance with an incentive, localisation requirement or certificate.

## Public anchors and their proper use

- **Finished EV price:** BYD South Africa currently lists the Dolphin Surf from R341,900. The official spec sheet lists Comfort at 30.08 kWh / 232 km WLTP and Dynamic at 38.88 kWh / 295 km WLTP. An entry-price plus 300 km WLTP target is an intentional cross-trim challenge, not a like-for-like comparison. Freeze matched price, trim, seating/payload, warranty, safety evidence and test-cycle boundary before claiming superiority. Sources: https://www.byd.co.za/offers/dolphin-surf and https://www.byd.com/material/byd-site/za/product/dolphin-surf/Dolphin%20Surf%20Spec.pdf
- **Pack cost and supply:** IEA's 2026 outlook reports that average battery-pack prices fell 8% in 2025, China pack prices averaged 30% below North America and 35% below Europe, and global battery production remains highly concentrated. These are market aggregates, not our cost curve: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-batteries
- **Tariff schedule:** the published SARS schedule shows a 25% general-rate entry for finished battery-only passenger EVs under the cited candidate heading and zero duty shown under lithium-ion accumulators. Classification, origin, preference eligibility, exact valuation, VAT and other charges remain route-specific; zero customs duty is not a zero landed cost. Source: https://www.sars.gov.za/wp-content/uploads/Legal/SCEA1964/Legal-LPrim-CE-Sch1P1Chpt1-to-99-Schedule-No-1-Part-1-Chapters-1-to-99.pdf
- **Industrial-policy direction:** Reuters reported on 15 October 2026 that South Africa intends to phase out support for current SKD assembly and move toward deeper CKD manufacture, with a 60% local-content ambition compared with an approximately 38% current average. This is a reported policy announcement, not the complete final legal rulebook or proof of incentive eligibility: https://www.reuters.com/world/africa/south-africa-stop-subsidizing-imported-vehicle-kits-pushing-automakers-build-2026-10-15/
- **Repairability:** a peer-reviewed assessment of six studied EV battery pack designs found up to 75% variation in disassembly costs across its sample. Treat that as evidence that architecture matters—not as a universal savings estimate: https://doi.org/10.1016/j.apenergy.2022.120437

## How to benchmark against industry and try to beat it

A valid “better than state of the art” claim must name the exact competitor trim, market, price boundary, model year, test cycle, equipment and confidence interval. Do not compare a published peak motor efficiency with a drive-cycle result, grid-side energy with pack-side energy, or retail tyre pricing with OEM acquisition cost.

| Domain | Benchmark protocol | Stretch target / release gate |
|---|---|---|
| Purchase affordability | Freeze same-market, same-date, same-trim delivered retail prices and warranty boundary. Compare against the exact Dolphin Surf trim, not an entry-price/range mixture. | Reach at least 10% under the frozen comparable vehicle's retail price without relaxing safety, payload, warranty or equipment requirements. |
| Manufacturing cost | Reconcile line-item low/high COGS to customs and factory gate; publish all-in route costs at 1k/10k/50k/100k cumulative units. | The high-cost case, not only the optimistic low case, must fit the inverse COGS ceiling before claiming robust affordability. Until input evidence is qualified, results stay needs_evidence. |
| Local manufacturing | Maintain an import-line customs record, verified classification/origin and source-backed local process costs for each material category. Report local spend separately from any legally defined local-content method. | Prefer the cheapest fully qualified route at each credible volume. Do not treat deeper localization as cheaper unless its measured cost, yield and volume make it so. |
| Energy and range | Use one declared test cycle, normalized pack-side or wall-side energy boundary, fixed ambient/tyre/load/configuration and an independently repeatable protocol. Add hot-weather HVAC, gradeability, ageing and payload tests. | Target no more than 12 kWh/100 km and at least 300 km only if the same measurement boundary supports both; verify the energy budget with ageing and temperature reserves, not nominal pack arithmetic alone. |
| Charging | Capture complete 10–80% sessions with delivered kWh, elapsed time, losses, battery temperature, charger power/taper, ambient temperature and repeat runs. | Beat the matched trim on full-session performance and compatibility, not advertised peak kW alone. |
| Serviceability and lifetime cost | Measure diagnostic-to-repair time, pack removal/reinstall time, part replacement scope, parts fill rate, warranty claims, degraded-battery performance and recovery yield. Calculate ownership cost per kilometre using transparent local energy/charging, insurance, finance, maintenance, downtime and residual-value assumptions. | Make a common set of high-frequency repairs modular and demonstrably faster/cheaper while preserving isolation, crash, thermal-propagation and post-crash safety gates. Demonstrate lower lifetime cost per kilometre for both home-charging and public/depot-charging use profiles. |
| Manufacturing quality | Measure first-pass yield, defect Pareto, rework hours, end-of-line test coverage and field failures by VIN/lot. Never infer yield from a design sketch. | Track a launch baseline and staged yield improvement; treat yield, safety and warranty inputs as measured evidence before they upgrade the cost result. |
| Circularity | Track battery state-of-health distribution, repairable module/component share, disassembly time, material recovery mass and actual recycler path. | Demonstrate service and recovery outcomes on a physical pack, not just a claim that the design has fewer fasteners or a structural pack. |

These are proposed programme benchmarks, not claims that industry has already been beaten. A genuine superiority statement requires comparable vehicles and independent or measured evidence; the current route fixture remains synthetic by design.

## Paradigm shifts with the largest potential

1. **Design from a cost ceiling backward.** Treat the retailer price as a constraint, invert margins and tax, then allocate a cost budget per subsystem. Re-run the waterfall whenever market price, margin assumptions or tariff treatment changes.
2. **Optimize usable mobility, not battery size.** Minimize cost per reliably delivered kilometre over the customer's actual duty cycle. Compare smaller packs, aerodynamic/rolling-resistance improvements, thermal controls, load/grade limits and charging access against matched energy and reliability requirements.
3. **Make manufacturing topology a design variable.** Evaluate complete import, CKD and deeper production using the same service targets, BOM boundary, warranty policy and realistic volume. A local part must win on verified delivered cost, quality, lead time, resilience or public-value criteria—not a slogan.
4. **Design for maintenance and second life on day one.** Service access, diagnostic coverage, standard fasteners, safe pack isolation and a documented battery-reuse/recycling path can affect downtime and lifetime cost; measure those effects instead of assuming savings.
5. **Treat uncertainty as a first-class output.** Show low/high intervals and why they exist. A missing quote, unknown tariff classification or unverified VAT status is an explicit blocker—not zero cost.
6. **Use Symtropy as an evidence-aware digital thread.** Bind each candidate to a dated input ledger, test protocol, traceable transformations, deterministic report and negative controls. Use Symthaea for sensitivity/robustness reasoning only where its input assumptions and outputs are independently checkable; do not label a model “AI-optimized” as proof.

## Fail-closed rules

- No preferred tariff rate without verified product classification, country of origin and preference eligibility.
- Never apply an import duty twice; distinguish ex-works, FOB, CIF/customs-value, landed, duty-paid and VAT-inclusive boundaries.
- VAT recovery unresolved means report both recovery and non-recovery costs; do not silently select one.
- Tooling is allocated only across the cumulative volume that actually shares it. A platform family does not make unrelated product tooling free.
- Higher local spend does not automatically lower COGS or prove legal local content.
- Cost inputs only become evidence-backed when sourced from qualified quotes, measured production or audited costs. Public retail prices and market aggregates remain context, not supplier offers.
- No route cost result waives safety, crash, battery propagation, electrical protection, homologation, serviceability or warranty gates.

## Claim ceiling and next evidence

The CLI and fixture are a route-economics screen—not a customs ruling, tax opinion, eligibility determination, audited BOM, production-feasibility claim, vehicle certification or recommendation to invest. The next evidence improvements are a complete classification-by-classification public tariff ledger, public tenders/catalogues with genuine production boundaries, measured process/yield studies, and transparent costed teardown/repair data. If a suitable public source cannot be found, the price remains unknown; we will not contact suppliers or invent it.
