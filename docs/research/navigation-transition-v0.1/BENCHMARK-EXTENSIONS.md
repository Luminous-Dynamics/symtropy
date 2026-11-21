# Navigation benchmark extensions v0.1

**Status:** draft research references; not reproduced or qualified.  
**Parent benchmark:** NAV-TRANSITION-001 #6434 / draft PR #6435.  
**Machine-readable companion:** `benchmark-extensions.v0.1.json`.

## Multi-session mapping and relocalization: CROSS

The [CROSS benchmark](https://jiaming.im/CROSS/benchmark/) is an adjacent industry comparator for map lifecycle and recovery, beyond single-sequence ATE. Its fixed protocol separates:

- **T1 — Mapping:** ATE RMSE and completeness.
- **T2 — Multi-session localization:** localization recall and map-frame ATE in a stored map from an earlier session.
- **T3 — Relocalization:** success over 10-second trials from no initial pose, with indoor/outdoor distance thresholds.

The published protocol covers KITTI, OpenLORIS-Scene, ROVER and SimChange v3. Its public 25 October 2026 results snapshot reports 12,491 runs. The protocol's important caveat is sensor and odometry privilege: some configurations use dataset/external odometry, and some use ground-truth-derived noisy odometry. That makes it essential to match exact input privileges, map/query sessions, failure denominators and tuning rules. CROSS is not a protection-level or PNT-integrity benchmark.

Our proposed extension is to evaluate **relocalization under qualification constraints**: a pose should not return to NORMAL simply because the system matches a map. The map's version, datum, frame, epoch, freshness, route/site lineage and observation dependencies must still be qualified.

Sources: [CROSS protocol](https://github.com/jiaming-ai/CROSS/blob/master/benchmark/PROTOCOL.md), [current results](https://jiaming.im/CROSS/benchmark/).

## Maritime sector conformance: IMO MSC.401(95)

The [IMO MSC.401(95) standard](https://wwwcdn.imo.org/localresources/en/KnowledgeCentre/IndexofIMOResolutions/MSCResolutions/MSC.401%2895%29.pdf) supplies a sector-specific receiver profile reference. It requires means for integrity monitoring of each PVT source and multi-source autonomous integrity monitoring; the receiver should assess whether achieved accuracy/integrity meets the requirements for each navigation phase and alert when the assessment cannot be determined. It also specifies update cadence expectations for high-speed craft and conventional vessels.

This translates into profile-specific test dimensions:
- integrity evidence for each PVT input;
- cross-source consistency with genuine independence assessment;
- per-phase limits and explicit unknown/alert behavior;
- update freshness, stale-last-fix indication and requalification after interruptions;
- no inference that two GNSS constellations are independent when shared antenna, receiver, clock, power, correction or processing dependencies remain.

This is a receiver performance standard, not a universal algorithm prescription. A synthetic test can align to selected requirements; it cannot establish type approval or vessel safety.

Sources: [official IMO MSC.401(95) resolution](https://wwwcdn.imo.org/localresources/en/KnowledgeCentre/IndexofIMOResolutions/MSCResolutions/MSC.401%2895%29.pdf), [IMO current navigation-equipment document index](https://www.imo.org/en/ourwork/safety/pages/docs-shipbornerc-navigationequipment.aspx).

## Statistical confidence: reuse SIM-REL-054

Reuse [SIM-REL-054 #3385](https://github.com/Luminous-Dynamics/symtropy/issues/3385) for exact Clopper–Pearson interval machinery. Do not implement another exact-binomial engine inside navigation.

The navigation-specific sampling hierarchy must be independently defined. Candidate grouping dimensions include route, site, recording session, receiver/device, campaign, scenario generator and oracle provenance. A stream of 100,000 frames from one run is not 100,000 independent trials. Define an episode-level integrity failure outcome, count only defensibly independent episode clusters for the interval, and publish time-integrated false-safe exposure separately. Any confidence claim must explicitly state the cluster mapping, independence assumptions, alpha allocation and underpowered strata.

## Claim ceiling

These references improve the benchmark design; they are not reproduced system results, certification evidence or Luminous superiority claims. Dataset access and rights, exact versions, sensor privileges, ground-truth quality and complete protocols must be reviewed before a matched bake-off.
