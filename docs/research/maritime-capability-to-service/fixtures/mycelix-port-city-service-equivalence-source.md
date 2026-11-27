# Port-city service-equivalence owner fixture provenance

- Owner: Mycelix PORT-CITY-SERVICE-GOLDEN-TRACE-001 (#6343), child of cross-stack golden trace #5974.
- Owner PR: https://github.com/Luminous-Dynamics/mycelix/pull/6344
- Source branch: `research/port-city-service-golden-trace-v0-1`
- Source commit: `654d37bc1aac06ece189c8fb0c0a0202c69bf6eb`
- Source file: `docs/research/port-city/PORT_CITY_SERVICE_EQUIVALENCE_GOLDEN_TRACE_V0_1.json`
- GitHub blob SHA: `ee9ecb7fc42009c6658e644b57e8f459aa68b979`
- Snapshot date: 19 November 2026
- Source status: draft integration benchmark; no physical/site claim and no owner-native CI qualification established on this source head.
- The exact JSON is frozen unchanged in this MCSC fixture. The companion TSV is a deterministic projection of its 48 case dispositions, three policy traces and three heat partitions.

The source trace is synthetic and contains:
- 48 owner/cross-owner dispositions;
- four 15-minute intervals and three matched policies (baseline, naive objective-only shift, no-new-peak admitted);
- identical 6.125 MWh PCC import energy across the policies;
- naive 9.2 MW peak and 1.0 MW violation of an 8.2 MW synthetic PCC cap;
- guarded 7.2 MW peak and unchanged 100 compute work units, vessel service, terminal service, cooling and RO electrical load;
- three per-policy heat partitions whose residuals close arithmetically.

The water product volume/quality remain null. Heat closure alone cannot establish qualified water output or environmental compliance. The fixture makes no physical power, potable-water, marine, commercial or site-performance claim.

MCSC consumes this owner trace as a conformance fixture. It does not implement the PCC verifier, compute-flex scheduler, thermal store, RO process, marine forecast, spatial rights layer or common-cause dependency graph.
