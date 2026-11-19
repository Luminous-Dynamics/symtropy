# South African grid restoration source profile v0.1

**Parent:** [SOUTH-AFRICA-GRID-PROFILE-001 #6286](https://github.com/Luminous-Dynamics/symtropy/issues/6286)  
**Programme:** #5241 Power Fabric benchmark  
**Snapshot:** 2026-11-19

## Status

This is a **source-register and evidence-scope profile**, not a dataset benchmark and not a utility partnership. Every source is currently `reference_only_unfrozen`: URLs, dates, reporting periods and claim ceilings are recorded, but the exact source bytes have not yet been downloaded, hashed, license-reviewed and frozen. The CI contract deliberately keeps that status explicit.

## Most important recent source: NTCSA MTSAO 2027–2036

The 30 October 2026 Refined Draft is the current system-adequacy reference in this profile. It uses an hourly chronological network-constrained production-cost model with 200 Monte Carlo samples per study year, regional nodes and transmission corridors. It is an adequacy assessment under declared assumptions, not an expansion plan, operating plan, investment recommendation or firm forecast.

The model reports that the Reference Case exceeds the 20 GWh annual UE criterion in 2027 and 2028 mainly because of one local transmission bottleneck; a fourth 315 MVA transformer is assumed to relieve it in 2029. A second substation load risk of about 102 MW appears from 2033. The Perfect Storm conditional scenario exceeds 900 GWh annual UE and 2,000 MW CVaR by 2036. These are conditional model results, not probabilities or predictions.

The report's structured Reference, Perfect Storm and Accelerated Build scenarios and Climate Stress / Reduced Storage sensitivities are not assigned scenario probabilities. The 200 Monte Carlo samples are internal simulation runs per study year; they do not turn the three named scenarios into a probability distribution.

## Layer boundaries

- **Generation:** EAF and other fleet availability indicators are national generation metrics.
- **Transmission:** adequacy model nodes, corridors, transfer constraints, TDP projects and ITP programme scope are separate from distribution reliability.
- **Distribution:** municipal and Eskom feeder/substation interruption histories require their own source data, coverage and denominator.
- **Household/service:** household, organization/community, town/municipality and city/metropolitan impacts remain separate beneficiary scopes under #6214.

The NTCSA MTSAO explicitly does not replace the TDP, GCCA, detailed network design studies or project-specific connection studies. It is not a municipal feeder reliability dataset.

## Claim controls

No local SAIDI/SAIFI, customer minutes lost, feeder fragility, municipality outage rates or customer ENS may be inferred from national EAF, transmission planned-kilometres or MTSAO system-level UE. A planned or announced transformer/line is not commissioned capacity. A modelled project inclusion is not a delivery commitment. The source records remain unpinned until exact source bytes and reuse permissions are frozen.

## Next data increment

1. Download the original TDP and MTSAO PDFs through a rights-cleared workflow; capture exact version, bytes, SHA-256 and extraction tools.
2. Create a page/table/row extraction register with original units and transformation digests.
3. Find non-sensitive municipal/feeder reliability data with explicit coverage; if unavailable, maintain the distribution layer as UNKNOWN.
4. Create an independently reviewed synthetic generation-to-transmission-to-distribution-to-service dependency case, separate from the operator's adequacy scenarios.
