# Manufacturing Accepted-Output Resilience Scorecard v0.1

**Owner:** Symtropy issue [#7223](https://github.com/Luminous-Dynamics/symtropy/issues/7223)  
**Implementation:** draft research branch `research/mfg-accepted-output-scorecard-v0-1`  
**Evidence class:** synthetic, offline  
**Claim ceiling:** deterministic arithmetic over frozen synthetic episode traces; no physical acceptance, product conformity, production release or industrial resilience claim.

## Why this metric exists

A production schedule can finish an operation without creating an accepted product. A machine can report cycle completion while a first-article inspection is pending, a measurement is conflicted, a genealogy link is broken, or a product is rejected. Makespan and cycle-completion metrics can therefore report apparent recovery before the production service has actually recovered.

This scorecard follows the boundary already owned by [#5411](https://github.com/Luminous-Dynamics/symtropy/issues/5411): the meaningful reconfiguration interval ends at the first independently accepted output under the target configuration, not at the first machine cycle. It composes with [#5758](https://github.com/Luminous-Dynamics/symtropy/issues/5758) intended/recorded/observed reconciliation and [#7190](https://github.com/Luminous-Dynamics/symtropy/issues/7190) decision-cut visibility. It does not implement those systems or authenticate source evidence.

## Metric definitions

Each frozen episode declares:
- disturbance time (t_0);
- evaluation horizon (T);
- demand in units (D > 0);
- recovery target (R), with (0 < R \le D);
- product-output records produced in the interval ([t_0,T]).

A unit is credited as independently accepted only when all three conditions hold:
1. its disposition is `ACCEPTED`;
2. verifier class is `INDEPENDENT`;
3. evidence disposition is `ADMITTED`.

A unit is credited as independently rejected only when disposition is `REJECTED` with the same independent/admitted conditions. Everything else remains unresolved, including producer-only acceptance claims, unknowns and conflicts. No inference is made that unresolved output is physically defective; it is simply not credited as accepted.

For an episode:
- **Produced units:** all output units in the declared episode.
- **Independently accepted units:** the qualified subset above.
- **Independently rejected units:** independently verified, admitted rejections.
- **Unresolved units:** produced minus independently accepted minus independently rejected.
- **Unqualified acceptance claims:** units marked accepted but lacking independent/admitted support.
- **Time to first accepted output:** earliest qualified acceptance time minus (t_0), nullable if none.
- **Time to recovery target:** first time cumulative qualified accepted units reach the predeclared (R), minus (t_0), nullable if never reached.
- **Accepted demand fill:** (min(A,D)/D), reported in parts per million.
- **Accepted quality yield:** (A/P), where (A) is accepted units and (P) produced units; nullable if no output was produced.

All ratios use integer arithmetic and floor rounding. The report keeps episode-level vectors and family-level totals/counts; it intentionally does not collapse quality, evidence and service into a single weighted score.

## Frozen v0.1 synthetic cases

The four fixture cases exercise:
- nominal recovery with independently accepted, rejected, producer-only and evidence-unknown output;
- correlated disruption with conflicted acceptance evidence;
- no independently accepted output despite a producer-only acceptance assertion;
- complete outage with no produced units.

Golden answers are independently written in the known-answer fixture. The Rust evaluator does not derive the expected answers from its own output.

## Adversarial invariants

- Duplicate case IDs and duplicate output IDs fail closed.
- Outputs outside the declared disturbance/horizon interval fail closed.
- Disposition cannot predate production; accepted/rejected outcomes require a disposition timestamp.
- Producer-only, Unknown or Conflicted acceptance claims never count as accepted.
- Input and output serialization order cannot alter the scorecard.
- Zero accepted output yields null time-to-first-accepted and null time-to-recovery-target—not zero.
- Zero produced units yields null quality-yield, not an invented 0% sample yield.
- The evaluator is offline and read-only. It creates no work order, resource reservation, machine command, product release or physical acceptance.

## Comparison protocol

This scorecard is an outcome layer, not a replacement scheduler. On a future matched policy tournament:
1. freeze the same case, disturbance stream, decision-visible evidence prefix and output-acceptance evaluator for every policy;
2. compare a common baseline portfolio (SPT, EDD, least slack, scarcity/deadline dispatch, event-triggered repair, fixed-interval rolling horizon, full re-optimization and selective repair);
3. separately report makespan, tardiness, schedule changes, time-to-first-accepted-output, time-to-recovery-target, cumulative accepted throughput, rejected/unresolved output, and P95/CVaR recovery times;
4. publish per-instance paired differences and family-stratified uncertainty intervals;
5. retain runs with no recovery as censored / unrecovered outcomes rather than excluding them from successful-case averages.

For a reconfiguration study, keep source-configuration last accepted output and target-configuration first accepted output distinct. The exact A→B interval remains with #5411; this scorecard measures accepted output within a defined episode.

## Relation to public benchmarks and industrial products

FJSPLib is the static deterministic comparison track: its public library includes flexible-job-shop, machine-independent and flexible-assembly instances and documents a standardized 10-minute solver comparison. Do not numerically compare this synthetic recovery scorecard to those static scheduling scores.

Dynamic methods such as incumbent-aware selective-repair (e.g. SLR2) are valid future comparators only after compatible event traces, objectives, compute budgets and acceptance semantics are aligned. Likewise, document-driven masked scheduling is a useful comparator for precedence/capability/availability semantics, not a substitute for independent output acceptance.

SAP PP/DS already provides detailed resource scheduling and availability-aware planning. This scorecard is intended as an assurance/outcome overlay around such schedulers, not a claim to replace APS/MES products.

## Next versions

- Add event-linked acceptance records by composing #7191's decision-cut projection rather than reimplementing an event ledger.
- Compose accepted output with #5411 reconfiguration chronology and #5758 discrepancy/traceability disposition.
- Add operations, WIP, setup transitions, rework loops and inspection capacity only through their existing owners.
- In #7189 v0.2, evaluate this scorecard across held-out regimes and compare policies on matched episode traces.
- Only consider physical trials after product, metrology, source rights, safety, process and competent acceptance prerequisites are independently admitted.

## Validation status

The focused workflow and exact-head repository CI are authoritative for the code commit, once completed. Synthetic known-answer passage demonstrates arithmetic and declared invariants only.
