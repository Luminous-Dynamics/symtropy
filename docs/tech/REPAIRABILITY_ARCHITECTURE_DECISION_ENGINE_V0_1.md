# Repairability Architecture Decision Engine v0.1

**Status:** decision protocol and software design proposal. It does not select a winning washer architecture or certify a physical product.  
**Scope:** Symtropy large-appliance repair programme; first application is W0–W4 washer architecture alternatives.  
**Related work:** [Portfolio issue #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [washer issue #1828](https://github.com/Luminous-Dynamics/symtropy/issues/1828), [portfolio PR #1827](https://github.com/Luminous-Dynamics/symtropy/pull/1827), [stakeholder profile issue #1872](https://github.com/Luminous-Dynamics/symtropy/issues/1872), and [stakeholder-constrained frontier method](STAKEHOLDER_CONSTRAINED_REPAIRABILITY_FRONTIERS_V0_1.md).

## 1. The paradigm shift: from “repairable-looking” designs to evidence-gated architectural decisions

A common design failure is to optimize the visible repair task while moving cost and risk elsewhere. A service panel can reduce access time but weaken stiffness; a replaceable module can lower labour but add seals, connectors, inventory and failure modes; a part can be technically replaceable but unavailable or economically irrational to stock. Conversely, permanent bonding can save initial assembly cost and improve sealing while making a later repair uneconomic.

The unit of design decision is therefore **the module boundary itself**. For every proposed split, record:
- which observed failure or maintenance task it addresses;
- the smallest safe replacement scope justified by evidence;
- the recurring BOM, assembly, connector, seal, stock, documentation and logistics burden the boundary introduces;
- the time/cost/waste saved against a frozen conventional control;
- effects on core function, reliability, energy/water, noise, safety and end-of-life recovery;
- what evidence would reverse the decision.

Do not reward modularity as an end in itself. A module boundary should earn its place by a measured and economically credible improvement in sustained useful service.

## 2. Hard-gate first; Pareto comparison second

The decision engine must never average away a critical safety failure or functional regression.

### Hard gates (non-compensatory)

1. **Safety/protection:** independent hazard review and required product-specific safety verification. A critical failure rejects the variant, regardless of cost.
2. **Core useful function:** preregistered cleaning, rinsing and spin-drying floors are met; failed function blocks efficiency claims.
3. **Durability/reliability:** declared endurance and post-repair verification criteria are met before asserting lifecycle advantage.
4. **Test-phase legal/safety authorization:** applicable market-specific requirements and permissions for the planned lab/service test have been identified and satisfied. This is not a statement that final product conformity has been achieved; final conformity remains a separate release gate.

Gate states are `unknown`, `pass` or `fail`. Unknown is not pass. Fail is a hard rejection. Only a candidate with every gate passing and every required comparison dimension backed by eligible evidence enters a comparative frontier.

### Multi-objective Pareto comparison

Keep separate dimensions: lifecycle cash cost, downtime, hands-on time, energy, water, part availability, discarded material, recurrence and added-interface failures. Do not collapse these to an unvalidated scalar score or hide trade-offs behind arbitrary weights.

For a **minimize** criterion with validated bounds `[lower, upper]`, A robustly dominates B only if A's upper bound is no greater than B's lower bound for every criterion, and is strictly lower for at least one. For a **maximize** criterion the inequalities reverse. If bounds overlap, report the pair as unresolved on that dimension; do not invent a winner. This is intentionally conservative. Bounds must represent stated uncertainty, not convenient ranges chosen after observing results.

A candidate not robustly dominated is on the robust Pareto frontier; it is not automatically safe, best, or commercially optimal. The rates are proportions in the closed interval [0, 1]; bounds for every criterion must be finite and non-negative. If two candidates are incomparable, report the trade-off and select the next measurement that can most efficiently discriminate between them.

## 3. What counts as eligible evidence

Every metric observation carries:
- exact candidate, physical unit/configuration, BOM/part and test-procedure identities;
- metric and unit definitions frozen before scored comparison;
- lower and upper bounds with a documented uncertainty method;
- source receipt and method identifier;
- evidence state and timestamp;
- deviations, exclusions and missingness.

Desk estimates, simulated outputs, catalogue entries and expert guesses may inform test selection but must be labelled distinctly from physical measurements. They must not quietly become numeric observations eligible for physical ranking. This v0.1 implementation deliberately blocks all candidates because there are no measurements yet.

Missing data remain unknown. No imputation of zero for prices, repair times, failures, lead times, waste or energy. If a zero is actually measured, represent it as a measured zero with evidence rather than as a missing value.

## 4. Lifecycle economics of a new service boundary

For design (d), declared service life (T), fault classes (f), and market assumptions (m), estimate the expected present cost of sustained function:

[
LCC(d,m)=C_{purchase}+C_{installation}+
\sum_{t=1}^{T}\frac{C_{energy,t}+C_{water,t}+C_{consumables,t}+C_{maintenance,t}+C_{repair,t}+C_{downtime,t}+C_{end\text{-}of\text{-}life,t}}{(1+r)^t}
]

The model must show the actual terms used; the equation is a framework, not a licence to guess inputs. Keep owner cash outlay separate from monetized downtime/externalities so readers can see which result depends on assumptions. Preserve successful and failed repair attempts, repeat work, part shipping, diagnostic labour, assembly-level replacement and disposal.

A boundary is economically justified only when its incremental lifecycle benefit remains credible after adding:
- more parts and fixtures at manufacture;
- connector, seal, fastener and access-panel count;
- extra electrical/contact or fluid leak failure modes;
- calibration/programming/compatibility costs;
- inventory and obsolete stock risk across regional markets;
- shipping, repair documentation, return logistics and rework;
- performance and durability side effects.

Report sensitivity to uncertain fault frequencies, useful life, local tariffs, labour rates, part delivery, discount rate and repeat-failure assumptions. Present a range and the break-even fault-rate/lifespan at which the new architecture becomes worthwhile. If that break-even point is implausible or highly uncertain, the next task is to collect evidence—not to declare a winner.

## 5. Choose experiments by value of information

The next experiment should reduce decision uncertainty for the least total cost and risk. Examples:
- access panel candidate vs control: timed, reviewed common-task disassembly/reassembly plus stiffness/sealing checks;
- service-spine candidate: repeat assembly trials to estimate misconnection, connector damage and rework;
- bolted vs welded tub: compare bearing-repair labour and component replacement waste against seal integrity, tolerance sensitivity, vibration and long-run fatigue;
- part-source study: written exact-fit verification, delivered lead time and return costs before buying a candidate solely for cheap catalogue parts.

Use Symthaea's existing generic finite-hypothesis EIG/EVSI kernel for test prioritization rather than creating an appliance-specific duplicate scorer. Keep hard safety/feasibility gates upstream of expected information gain. A high-information test that is unsafe or infeasible is not allowed.

Before candidate measurements, freeze controls, test profiles, instrument capability, functional margins, sample plan, exclusions, analysis code and claim language. Start with a control-only repeatability pilot. Use those variance/uncertainty estimates to design the comparative sample size rather than assuming three units prove superiority.

## 6. Required architecture metrics

The v0.1 criterion catalogue is intentionally compact:
- total lifecycle cost per declared useful-service profile (minimize);
- P90 elapsed restore time (minimize);
- first-visit verified restoration rate (maximize);
- energy per successful wash load (minimize after function gate);
- water per successful wash load (minimize after function gate);
- discarded material mass per verified repair (minimize);
- added-interface failure rate (minimize);
- declared common-part delivery-rate within a named target window (maximize).

Do not compare metrics with different definitions, loads, markets or observation windows. Time and money need both hands-on/out-of-pocket and end-to-end measures. Repaired units must receive the same appropriate functional verification, with follow-up recurrence kept separate.

These are candidate dimensions, not a complete regulatory test suite. Add task- and product-specific criteria before any scored study; keep the criteria frozen once data collection begins.

## 7. Benchmark programme and claim ladder

| Evidence phase | Gate | Permitted claim |
|---|---|---|
| Contract/schema audit | Machine-readable content and decision rules are internally coherent | “Protocol/audit contract is valid” only |
| Control-only pilot | Repeatability, instrument uncertainty and invalid-run rules documented | “Measurement procedure is repeatable within the stated range” |
| Matched architecture comparison | Frozen functional gates and uncertainty-aware comparison passed | Bounded comparison on tested units and tasks |
| Endurance and recurrence | Declared cycle/fault and follow-up window completed | Bounded durability/recurrence finding |
| Independent replication | Independent operator/team follows the frozen protocol | Reproducibility claim in the tested envelope |
| Product safety/conformity route | Applicable qualified tests and market obligations satisfied | Only the specific safety/conformity claim supported by the evidence |

A Rust audit pass confirms only the synthetic contract rules. It does not qualify an appliance, prove repair savings or replace conformity assessment.

## 8. Standards and prior work

- [IEC 60456:2024](https://webstore.iec.ch/en/publication/70049) specifies methods for measuring household washing-machine performance, including cleaning, rinsing, low-power and other revised measurements. Use the applicable full text/test protocol when defining a formal performance comparison; this project does not claim conformity to that standard.
- [Commission Regulation (EU) 2019/2023](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32019R2023) contains spare-parts and related resource-efficiency requirements for covered household washing machines/washer-dryers in its jurisdiction. It is not automatically the applicable legal rule in every target market.
- A 2025 open-access paper, [“Design for repair: insights to develop functional modules in household appliances”](https://doi.org/10.1016/j.procir.2024.12.004), applies EN 45554 and physical disassembly tests to several cooking appliances and identifies concrete module-design changes. The present programme should extend that physical-test mindset to matched lifecycle cost, function and uncertainty-aware architecture choices rather than assuming its findings transfer unchanged to washers.

## 9. Implementation invariants

The decision audit must reject or block:
- missing/unknown hard gates being treated as passed;
- critical safety failure being offset by lower cost or energy;
- incomplete metric vectors being compared as if complete;
- missing observations being converted to zeros;
- candidate bounds with lower greater than upper;
- observations without source/method identifiers or frozen metric definitions;
- point estimates presented as robust dominance when bounds overlap;
- desk estimates and physical measurements treated as one evidence class;
- post-hoc criterion, margin, exclusion or weight changes after seeing candidate results;
- outputs described as a product winner where no eligible candidates exist.

The initial washer decision fixture is intentionally unscored. The next real engineering decision is which measurement to fund or obtain access to first, not which W0–W4 option to declare superior.
