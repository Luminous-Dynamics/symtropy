# Verified Water Loss Recovery T0 protocol v0.1

**Tracking issue:** #5942 WATER-VERIFIED-LOSS-RECOVERY-001  
**Evidence class:** synthetic known-answer event ledger only.

## 1. Core change: detection is not recovery

The benchmark follows an event from signal to independently observed outcome: diagnosis, inspection, field confirmation, repair, post-repair measurement and follow-up. A model alert, crew dispatch or closed work order cannot self-certify success.

## 2. Attribution calculation

Use a matched treated DMA and control DMA with **separate explicit zone IDs and separate physical meter boundaries**. Within each zone, pre- and post-repair readings must retain the same zone, meter boundary and topology binding. The treated and control zones are not required to share one physical boundary.

`point_recovered_l_per_day = max(0, (treated_pre_loss - treated_post_loss) - (control_pre_loss - control_post_loss))`

Then calculate a conservative lower bound:

`lower_bound = max(0, point_recovered - k * (u_treated_pre + u_treated_post + u_control_pre + u_control_post))`

v0.1 sums standard uncertainties and does not assume independent meter/calibration errors. This is an expanded-uncertainty screen, not a confidence interval or population inference.

Pressure and demand must be comparable within each zone's pre/post pair. Pre windows and post windows must be aligned between zones; pre and post windows must not overlap. Each measurement must bind to the event's exact treated or control zone ID, the appropriate zone-specific boundary and the same topology revision/digest. If they are not, the output is `attribution_indeterminate`; no water recovery is credited.

## 3. Independent outcome requirements

- Inspection and field confirmation are recorded separately from the diagnostic model.
- A confirmed component must bind to the exact network topology revision.
- A completed/closed repair work order is not proof of a successful repair.
- Post-repair measurements must use current calibration/time evidence and the same boundary/topology. Method references may be shared across instruments where the same validated method is properly applicable; instrument IDs and raw-record digests must remain distinct.
- Partial repair may earn only a measured partial lower-bound reduction; unresolved leaks remain visible.
- Recurrence or a reopened repair defeats a sustained/permanent-repair claim.
- Duplicate recovery-claim IDs are rejected as a group.

## 4. Outputs

Preserve inspection cost, crew hours and repair cost even when no recovery is credited. Report localization correctness independently. Recovery outputs include point and lower-bound L/day, and a sustained-recovery eligibility flag. This evaluator does not infer cumulative litres over an unobserved time period.

## 5. Synthetic challenge pack

Thirteen cases include successful repair, no post-test, partial recovery, pressure and demand confounding, stale calibration, incorrect pipe localization, false alarm, concurrent leaks with one repair, failed repair, recurrence, duplicated recovery claims and topology/boundary mismatch.

## 6. Claim ceiling

A synthetic PASS proves only event-ledger and arithmetic behavior. It does not establish field water savings, leak-detector performance, live utility response, crew productivity, actual repair permanence, lifecycle superiority or authorization to dispatch, isolate, or actuate infrastructure.