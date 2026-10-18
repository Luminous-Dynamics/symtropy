# MARITIME-EVIDENCE-ACTION-001: decision regret under uncertain port-call evidence

**Status:** executable synthetic reference; exact-head Rust qualification pending.  
**Implementation:** crates/domains/symtropy-maritime-evidence/src/evidence_action.rs  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/1949  
**Evidence source boundary:** https://github.com/Luminous-Dynamics/symtropy/pull/1940

## Paradigm shift

A scheduler can be mathematically optimal for the wrong input. This benchmark asks whether source freshness, conflict detection, abstention and bounded contingency improve actual decisions, while charging abstention a declared service cost.

The synthetic actual-arrival value belongs to a separate EvaluationTruth type. Policy functions accept only EvidenceDecisionInput; they cannot inspect hidden truth. The evaluator receives that truth only after an action is produced.

## Compared policies

- LatestIngestedPointEstimate: naive baseline that selects the latest-ingested planned-arrival observation without validating source age or conflicting claims.
- FreshestValidatedSourcePointEstimate: selects the valid observation with the newest source-event timestamp, but does not resolve disagreement. This is a comparison baseline, not a safe authority.
- FailClosedUncontestedOnly: abstains if any relevant observation is rejected, the target arrival conflicts, or the target value is not uniquely resolved.
- BoundedWorstCaseContingency: treats valid planned-arrival values for the same port-call and same Pilot Boarding Place as a finite uncertainty set. It chooses the candidate window that minimizes worst predicted service cost across that set. It abstains if no admissible candidate remains or the location-context set itself conflicts.

The score is intentionally small and explicit: waiting costs round up to minutes; lateness costs minutes; lateness beyond the window's grace adds a declared missed-window penalty. Abstention has an explicit declared cost and is not assumed free.

## Frozen synthetic controls

Seven Rust unit tests currently cover:
1. Conflicting 12:00 and 13:00 planned arrivals with hidden actual arrival at 12:50.
2. A clean two-source agreement where fail-closed must still choose a window.
3. Stale-only data where freshness-based and robust policies abstain but the naive baseline still acts.
4. Vessel identity collision for a port-scoped call.
5. Different Pilot Boarding Place contexts where timing robustness alone is insufficient.
6. Input-row permutation invariance across all four policies.
7. Separate oracle/regret evaluation, no-window abstention, and a nonzero abstention cost.

The main illustrative scenario uses 12:00 and 13:00 candidate windows, 15-minute grace, a 100-minute missed-window penalty and a 130-minute abstention cost. In its synthetic conflict case, the latest-ingested point estimate picks the 12:00 window and incurs 150 cost-minutes against hidden truth at 12:50, while the robust envelope policy chooses 13:00 and incurs 10. This is an authored scenario calculation, not a measured port outcome or a Rust test result until CI confirms it.

All values and actual-arrival labels are synthetic. No policy receives the hidden actual-arrival label. No live port API or historical case is included.

## Metrics and proposed gates (not measured)

Report per policy and per case: chosen window or abstention, basis observation IDs, rejected count, conflict count, whether a point estimate acted despite evidence quality issues, service cost, oracle cost, regret, and served/unserved disposition.

Qualification targets:
1. 100% of frozen evidence-admissibility cases match the declared fail-closed policy.
2. Zero fail-closed assignments on stale, conflicted or identity-ambiguous cases.
3. Zero unnecessary abstentions on the clean, current, agreeing-source negative control.
4. 100% identical decisions under permutations of the same observation set.
5. 100% worst-case window-choice agreement with a separately written exhaustive tiny-instance oracle before increasing model complexity.
6. Report regret and abstention cost separately; do not treat abstention as costless or a composite score as evidence of safety.

This first slice does not introduce a route optimizer, assign real berths, activate flexibility offers, or mutate schedules. It demonstrates finite synthetic decision trade-offs only. Historical replay and real-port claims remain separate qualification gates under #1671 and #1949.