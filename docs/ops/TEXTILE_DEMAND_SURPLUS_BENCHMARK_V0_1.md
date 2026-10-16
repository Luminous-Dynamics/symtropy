# Textile Demand and Surplus Benchmark v0.1

Status: proposed benchmark contract; the typed Rust evaluator is implemented, but
its tests have not been executed in this environment. Targets below are goals, not
measured results.

## Paradigm shift: schedule what evidence justifies producing

Factory efficiency is not the same as system efficiency. A line can use energy
efficiently to produce goods that remain unsold. Compare production decisions
against the same frozen demand record and measure *accepted, fulfilled service*
and unsold newly manufactured output together. This is especially important for
rare-use formalwear, couture, ceremonial garments, performance costumes and
deadstock samples: low frequency of use is not a value penalty, but weak demand
evidence should not be converted into an unexamined production forecast.

The typed Rust module \`symtropy-textile-process::demand\` provides five transparent
production policies for each paired scenario:

- **MadeToOrder:** produce the confirmed order quantity.
- **PreorderPlusBuffer:** produce confirmed orders plus an explicitly declared
  safety-stock quantity.
- **SmallBatchFromOrders:** round the confirmed order quantity up to the declared
  batch multiple.
- **ForecastBatch:** round the greater of forecast demand and confirmed orders
  up to the declared batch multiple. A forecast cannot override committed orders.
- **ReuseInventoryFirst:** use eligible accepted inventory against demand first,
  then plan new production only for the remaining confirmed-order quantity.

The same demand, forecast, order book, inventory and assumptions are provided to
all policies in a scenario. Policy planning uses forecasts/orders/inventory, not
realized demand. Realized demand is used only for outcome accounting, avoiding
perfect-information leakage into the production decision.

## Output and accounting boundary

Each policy result preserves:
- planned new production, accepted new output, and rejected new output;
- modeled use of existing accepted inventory and unused remainder;
- fulfilled demand, on-time and late fulfillment, and stockout units;
- unsold accepted newly produced units;
- energy, water and material inputs at the declared per-production-unit boundary;
- service-fill, on-time and unsold-per-fulfilled-unit ratios, expressed as integer
  basis points and undefined when the denominator is zero.

The primary ledgers must close exactly:

1. planned new production = accepted new output + rejected new output;
2. accepted new output = new output fulfilled + unsold new output;
3. existing accepted inventory = inventory used + inventory left unused;
4. total fulfilled = on-time fulfilled + late fulfilled;
5. realized demand = total fulfilled + stockout units.

This evaluator does not sum measures across scenarios with incompatible units or
functional units. Results remain stratified by scenario/profile/service unit.
Any broader aggregate must disclose its weighting and demonstrate compatibility
before results are pooled.

## Semantic safeguards

- **No demand oracle.** Realized demand cannot influence a policy's production
  quantity. The policy receives forecasts and confirmed orders.
- **Orders are protected.** The ForecastBatch rule must plan at least the order
  book, even when the forecast is lower.
- **Existing stock is explicit.** Only ReuseInventoryFirst consumes the declared
  inventory in the counterfactual. The evaluator does not claim that goods were
  physically transferred, resold or used again.
- **Prediction is not receipt.** Inventory condition and forecast/demand
  evidence references are identifiers, not authentication. An actual transfer,
  successful repair, resale or second use requires its own observed receipt.
- **Quality yield is approximate in this v0.1 model.** Accepted output is computed
  by integer-floor multiplication of planned units by the declared acceptance
  rate (basis points). This is a conservative deterministic approximation and
  can bias small lots downward. Physical pilots must use observed integer good
  and rejected counts, not this approximation as a certification or forecast
  of production quality.
- **No environmental score.** Energy/water/material quantities remain separate;
  the module does not calculate carbon, avoided virgin production, transport
  effects or life-cycle benefits.
- **No assumed reuse substitution.** A stock unit used in this counterfactual is
  not automatically proof that a new purchase was displaced. Measure actual
  second use, durability, cleaning, transport and the displaced alternative.

## Proposed target ladder

| Tier | Measure | Initial target |
|---|---|---:|
| A — ledger correctness | Any identity above fails | **0** |
| A — demand leakage | Realized demand affects the planned production calculation | **0** |
| A — order protection | ForecastBatch plans fewer units than confirmed orders | **0** |
| A — provenance | Missing input evidence reference accepted | **0** |
| A — arithmetic | Overflow wraps or becomes an ordinary numeric result | **0** |
| A — comparability | Heterogeneous profiles or functional units silently pooled | **0** |
| B — determinism | Same scenario corpus/version produces a different normalized report | **0** |
| B — paired completeness | Every policy is evaluated on every valid scenario | **100%** |
| C — surplus | Unsold newly manufactured garments per fulfilled unit | Target **≥ 10% relative reduction** versus a frozen baseline |
| C — service | On-time fulfillment | No more than **1 percentage-point regression** |
| C — quality | Observed profile-specific acceptance | No regression beyond a preregistered non-inferiority margin |
| C — shortage | Stockout units | Report every increase; no hidden trade-off |
| C — resource use | Metered energy, water, and material per accepted service unit | Target **≥ 5% relative energy reduction** without hard-gate regression |

The 10% surplus and 5% energy targets are initial pilot hypotheses, not literature
constants. Choose the baseline and acceptance margins before data collection;
report uncertainty intervals, cancellations, markdowns, lead times, stockouts,
and rejected goods. A strategy that reduces surplus by simply failing to serve
demand does not count as a successful result.

## Locked test corpus

The first paired synthetic fixture should include:
- a rare-use decorative or formalwear item with forecast above confirmed orders;
- a workwear or basic-garment scenario with forecast below the confirmed order book;
- a performance or ceremonial service deadline where lead time impacts
  on-time fulfillment;
- an existing accepted sample/inventory case where reuse-first lowers new
  production but can also increase stockouts;
- a no-demand case, a poor acceptance-yield case and arithmetic overflow
  negative controls.

Use stable scenario IDs, revision/version, declared functional units, all unit
IDs, input evidence references, exact commit SHA and a SHA-256 of the exact
manifest bytes. Do not weight synthetic scenarios as if they were observed
market frequencies. Do not compare the total energy of a high-service-volume
policy with a low-service-volume policy without also reporting the demand served,
stockout, accepted output and functional unit.

## Research and interoperability

A 2024 review of 45 textile life-cycle assessment publications finds that service
life extension and reuse can provide important benefits, but the result depends
on recycling yield and material demand, the use phase, the virgin production
assumed to be displaced, substitution factors and transportation—especially in
sharing models. The benchmark therefore retains service and resource vectors and
requires sensitivity analysis rather than a universal “reuse is greener” score.

The European Commission identifies textile apparel as a priority product group
for future ecodesign requirements and a Digital Product Passport under the ESPR
Working Plan 2025–2030. Its current page says the textile-specific delegated act
is planned for Q4 2027 and that timelines can evolve. This work should keep
machine-readable product/profile, composition, service, care and custody links,
but it must not present this draft schema as an adopted regulatory data format
or legal compliance implementation.

References:
- Abagnato, Rigamonti & Grosso (2024), [Life cycle assessment applications to
  reuse, recycling and circular practices for textiles: A review](https://doi.org/10.1016/j.wasman.2024.04.016),
  *Waste Management* 182, 74–90.
- European Commission, [Textile Apparel and the Digital Product Passport](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/textile-apparel_en).
- European Commission, [Sustainable and Circular Textiles Strategy](https://environment.ec.europa.eu/strategy/textiles-strategy_en).
