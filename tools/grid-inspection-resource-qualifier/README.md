# Grid inspection resource certificate qualifier v0.1

**Owner:** Symtropy #6295  
**Parent:** #6273 — Grid Restoration Inspection  
**Upstream diagnosis/policy tree:** Symthaea #12403 / #12436, PRs #12412 / #12441  
**Snapshot:** 2026-11-19

## Purpose

This Rust package consumes a finite inspection policy certificate and independently qualifies whether its inspection and candidate recovery activities fit a pinned resource calendar.

It does **not** compute diagnoses, posteriors, likelihoods, probabilities, information gain, value of information or policy-tree choices. Those belong to Symthaea. The branch weights in the input are consumed as integer basis points and must sum to 10,000.

## What is verified

- Canonical SHA-256 of a separately referenced resource manifest.
- Resource identity, freshness and status.
- Capacity windows, confirmed reservations and unknown reservations.
- Common simultaneous availability for every resource required by one activity.
- Inspection occupancy versus result latency: resources are released when inspection work ends; the next decision waits until the result is available.
- Second-stage measurements cannot reuse the first measurement ID or evidence origin.
- First and second outcome weights conserve 10,000 basis points; nested leaf products must be exactly representable.
- Mutually exclusive branches are scheduled independently and resource-hours are probability-weighted, not summed as concurrent work.
- Service trajectories remain references to #6235. Candidate action completion is never labelled service restored.

Only resources explicitly in `available` state can be allocated. A resource marked degraded, unknown, unavailable or proposed-only fails closed. Unknown reservations block overlapping intervals until their end; confirmed reservations consume capacity.

## Synthetic fixture

The example's independent first inspection occupies one specialist crew and one test set from hour 8 to 10; its result is available at 10.5. On the negative first branch, a second bushing test uses the same crew/test set from 10.5 to 12; its result is available at 12.25. Repair cannot start before 12.25. Replacement needs the same crew plus crane and heavy-haul, whose windows begin at hour 12.

The upstream policy branch weights are a synthetic adapter example:
- first positive: 3,000 bp -> replacement;
- first negative: 7,000 bp, then second positive 2,000 bp -> replacement;
- first negative then second negative 8,000 bp -> repair.

The leaf weights are 3,000, 1,400 and 5,600 bp. The package does not claim these are empirically calibrated probabilities.

## Output semantics

`expectedCandidateActionCompletionHour` is a probability-weighted candidate repair/replacement completion time under the supplied branch weights and resource calendars. It is **not** time to first service, energy restored or independently accepted service. The report emits the #6235 service trajectory reference for separate evaluation.

`expectedResourceHours` are probability-weighted unit-hours for resource planning. They are not currency costs and are not another VoI penalty; do not subtract them from Symthaea's diagnostic loss unless a separately declared, dimensionally consistent conversion model is supplied.

## External comparison

- Symthaea's #12436 depth-two exact policy tree already enumerates adaptive diagnostic choices under a finite model. This package deliberately does not duplicate that search.
- Kumari & De (June 2026), DOI 10.1016/j.segan.2026.102294, jointly models repair-crew depot placement, uncertain travel/repair duration and mobile generators.
- Han et al. (2026), DOI 10.1109/ICPSAsia70813.2026.11692267, combine UAV visual assessment, crew routing, reinspection and restoration timing.
- Boateng et al. (24 June 2026), DOI 10.1186/s43065-026-00194-y, show how incomplete data and socio-economic impacts alter restoration priorities.
- IEEE PES TR143 (June 2026), DOI 10.17023/7e16-km93, addresses dynamic resilience metrics; TR148 (October 2026), DOI 10.17023/srdf-0n62, covers planning, operations, investment and supply-chain resilience.

The intended differentiator is the auditable connection between adaptive inspection branches and the shared operational resource calendar—not a claim that repair scheduling or multi-resource restoration is new.

## Claim boundary

Synthetic resource/calendar policy-certificate qualification only. No diagnosis, physical measurement, authorization, crew dispatch, actual work, switching, energization or restored service is produced.
