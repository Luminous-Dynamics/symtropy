# Johannesburg Water Reported-Service Data Admission Note v0.1

**Parent:** HIGHVELD-HYDRO-DATA-ADMISSION-001 #5350  
**Source candidate ID:** `JOHANNESBURG-WATERSEARCH-DAILY-STATUS-API`  
**Evidence class:** third-party publisher compilation of utility-reported asset status  
**Acquisition status:** API documentation and live sample metadata observed; bulk data, release manifest and bytes not acquired  
**As-of evidence:** 17 November 2026  
**Claim ceiling:** reported reservoir/tower/pump-station/direct-feed status; not tap-level service or hydrometric truth

## 1. Discovery

Watersearch exposes a free, read-only, no-key API for Johannesburg Water (JW) status boards and notices. Its API documentation states that it reads the boards/notices published on JW's website since March 2025 and records what JW reported for named reservoirs/towers/pump stations/direct feeds and which suburbs JW links to them.

Useful source entry points:
- API documentation: https://watersearch.co.za/api
- Supply register: https://watersearch.co.za/api/v1/supplies
- API OpenAPI document: https://watersearch.co.za/api/v1/openapi.json
- Suburb history example: https://watersearch.co.za/api/v1/suburbs/melville/history
- JW source website: https://www.johannesburgwater.co.za/

The API documentation explicitly states that this is **not** a reading of anyone's tap. It also states a CC BY 4.0 licence for the compilation/readings, but the licence is marked provisional until the site is public. The original facts are attributed to Johannesburg Water and the source requires credit to JW. The publisher states that it is not run by Johannesburg Water or the City.

The register now includes this candidate source, but does not admit its raw data bytes or treat the provisional licence as finally cleared.

## 2. Live observations captured in this research pass

The supply register endpoint returned:
- publisher release ID: `2026-11-16-892a6573`;
- records for reservoirs, towers, pump stations and direct feeds;
- JW system relationships where available;
- asset IDs and asset kinds.

The suburb history endpoint for Melville returned:
- publisher release ID: `2026-11-17-ee99662f`;
- one row per linked supply per SAST day;
- daily worst reported status, textual wording, board count and bypass flag;
- an explicit `no_report` status where JW had not published a board that day.

The example showed a Hursthill 2 Reservoir row marked `no_report` on 8 November 2026 and separate `warned` / on-bypass rows on 14–16 November 2026. This demonstrates why missing, warned, on-bypass and no-water must remain separate states. These sample values were read from live API output; the full history and bulk release have not been acquired or hashed.

## 3. Data products and semantics

The documentation describes the following endpoints:
- supply register and individual supply metadata;
- daily history per supply and per linked suburb;
- individual status-board reports, including board time, original wording and links to JW posts/images;
- outage episodes, including censored/minimum durations where JW has not reported an end;
- notices with a stated completion date and first later board on which JW reports recovery;
- systems and regions;
- city rankings;
- dated releases, including bulk CSV/JSONL links and manifests with row counts and SHA-256 checksums.

Important interpretation rules declared by the publisher:
- A day with no JW board is a gap (`no_report`), not a normal day.
- A daily row is the worst status JW reported for that supply that SAST day; it is not a continuous sensor time series.
- A suburb may be linked to multiple supplies; the suburb must be interpreted across all linked supplies.
- Scheduled overnight closures are counted separately, not as unplanned outages.
- Censored outage durations are lower bounds when no end was reported.
- The derived outage logic may bridge up to four no-board days between two outage reports; retain that rule version and do not treat it as proof of physical continuity or restoration.

## 4. What this can and cannot support

**Potentially useful for:**
- historical reported-supply service scenarios for WaterLoop continuity;
- event grouping and notice-to-reported-recovery comparisons;
- testing how well hydrology/utility event models anticipate supply-level stress periods;
- service-cascade studies joining DWS catchment conditions, dam/rainfall context, JW operational reports, power interruption reports and independently measured household observations;
- selecting candidate periods and assets for later household-level instrumented studies.

**Cannot support on its own:**
- a household tap-level outage label;
- actual fixture pressure, flow, volume or water availability at a specific dwelling;
- pipe-break location, failure cause or measured repair effectiveness;
- an independent hydrometric streamflow observation;
- drinking-water quality or health claims;
- a claim that a notice completion time equals actual restoration at all customer taps.

The data are therefore a **reported utility asset-state layer**, separate from DWS gauge observations and separate from household point-of-use measurements.

## 5. Suggested benchmark task

Build three linked but non-interchangeable populations:

1. **Hydrometric physical observations:** DWS station flow/level and rainfall/meteorology, with exact rating tables, quality flags and timestamp semantics.
2. **Utility-reported service state:** Watersearch/JW supply status, notice and outage reports, preserving publisher release, JW source URL, SAST, `no_report`, bypass, scheduled closures and censored episodes.
3. **Point-of-use observations:** later test-cell/household pressure, flow, fixture availability and resident reports, independently admitted through Mycelix/Sol-Atlas.

Benchmark the model separately on each layer. Only link them when the asset topology, supply mapping, temporal semantics and spatial coverage support the association. Do not train on the supply status and claim it predicts household tap service unless point-of-use outcomes are independently measured.

Recommended metrics for the utility-reported layer:
- outage onset probability / Brier score and Brier skill against seasonal climatology;
- lead time and false-warning burden;
- duration error with right-censoring handled explicitly;
- return-to-reported-service delay relative to published notice end;
- calibration by asset type, JW system and season;
- missing-board coverage / gap rate;
- transfer to held-out assets and years.

Any metric must keep the label class explicit: reported state, not observed tap state.

## 6. Data acquisition and admission checklist

Before benchmark use:
1. Confirm the current licence and attribution statement is still valid; record the licence field and response `X-Licence` header for each acquired release.
2. Acquire one immutable bulk release and the associated CSV/JSONL manifest; store release ID, retrieval timestamp, row counts and source SHA-256 checksums.
3. Preserve the JW original notice/post/image URL references and, where available, the exact board text/timestamp behind each compiled row.
4. Verify SAST day boundaries and multiple-board-to-daily-worst aggregation rules.
5. Preserve `no_report`, `on_bypass`, scheduled closure, `warned`, `low`, `no_water`, `critical` and unknown statuses as separate typed values.
6. Preserve the publisher's derived outage algorithm/version, censoring and no-board-day bridging policy.
7. Compare a stratified sample against the original JW source boards, including normal, low, no-water, critical, bypass, planned closure, missing-board and censored-outage cases.
8. Only after the bulk release and cross-checks are admitted, create frozen asset/year/system holdouts and independent event oracle rows.

## 7. Recommended first use

Use a small, stratified historical sample (one central/commando system plus one Soweto or Midrand system) only after the release bytes and licence are admitted. Select:
- a long outage episode;
- a shorter unplanned outage;
- a scheduled overnight closure;
- a bypass/warned period;
- a censored/no-reported-end event;
- a no-report gap;
- a notice with an announced end date and later board recovery.

Do not use only the most severe outages; include ordinary and missing-report days to avoid prevalence and selection bias.

## 8. Nonclaims

No full data acquisition, source-rights clearance, row-level quality review, raw board archival, household service validation, utility endorsement, or current water-quality/hydrology claim is complete in this note. The source remains a candidate until immutable release bytes, licence state, manifest and source-board cross-checks are admitted.