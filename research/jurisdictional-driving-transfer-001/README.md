# JURISDICTIONAL-DRIVING-TRANSFER-001

**Status:** source-qualified profile skeleton and geometry-only LHT/RHT metamorphic auditor  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4774  
**First profile:** ZA (South Africa), explicitly `draft` and not policy eligible

## Purpose

Make jurisdiction assumptions explicit, versioned and auditable rather than silently embedding a universal/US-centric traffic convention in the driver.

Three independent concerns remain separate:
1. traffic geometry: left/right traffic, lane topology and maneuver handedness;
2. legal/rule semantics: signs, markings, signals, right of way, passing and temporary traffic controls;
3. interaction ecology: vehicles, pedestrians, cyclists, public transport and local traffic participants.

Traffic side does not imply steering-wheel position. Vehicle imports mean the latter must remain a separately evidenced field. Neither implies roundabout circulation without an independently sourced rule.

## First South Africa source skeleton

`fixtures/za-profile-draft-v0-1.json` records:
- jurisdiction code `ZA`;
- `traffic_side = left`, supported by the South African Department of Tourism's road-travel guidance;
- `steering_wheel_side = unknown`;
- `roundabout_circulation = unknown`;
- references to the Department of Transport's SADC Road Traffic Signs Manual and the National Road Traffic Act 93 of 1996 source page;
- no claimed current legal revision/effective date/content digest for those legal sources;
- all sources `not_reviewed`, profile status `draft`.

This is intentionally a source inventory, not a machine-readable South African driving rule pack. The cited tourism guidance is suitable as corroboration that South Africa drives on the left; it is not the authority for maneuver details. The Department of Transport manual and gazetted legislation require cross-checking against current editions/amendments, commencement and applicability. A URL that exists is not proof that the linked text is current law.

Before any policy use, a competent local legal/transport reviewer must confirm:
- exact manual edition/volume/chapter and amendments currently applicable;
- exact Act and Regulations consolidated revisions, amendment history, commencement and effective dates;
- rule-by-rule source mapping and unresolved/conflicting interpretations;
- source content digest and a versioned rule-bundle digest;
- data/label provenance and the jurisdictional scope of each scenario.

The validator will not permit `policy_eligible` until a profile has an effective date, a rule-bundle digest, a legal-reviewed statute source and a legal-reviewed traffic-sign-manual source, each with revision, effective date and content SHA-256. This is a structural gate, not a legal correctness oracle.

## Convention metamorphic fixture

`fixtures/za-us-metamorphic-lht-rht-v0-1.json` is a synthetic geometry-only pair:
- same topological task-intent identifier;
- left-side traffic vs right-side traffic;
- lateral coordinates mirrored about the road centreline;
- left/right maneuver handedness mirrored;
- distinct (synthetic) jurisdiction rule-bundle digests.

The transform is useful for checking coordinate/intent plumbing. It **does not** transform traffic law, sign semantics, roundabout rules, actor behaviour or right-of-way. Those are separate jurisdiction-specific labels and tests. A geometry-mirror pass is not evidence that the legal tasks are equivalent.

## Rust crate and CLI

Pinned Rust 1.96.0, Rust 2024:

```sh
cargo fmt --manifest-path research/jurisdictional-driving-transfer-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/jurisdictional-driving-transfer-001/Cargo.toml
cargo clippy --locked --manifest-path research/jurisdictional-driving-transfer-001/Cargo.toml --all-targets --all-features -- -D warnings
```

Audit the South Africa source skeleton:

```sh
cargo run --locked \
  --manifest-path research/jurisdictional-driving-transfer-001/Cargo.toml -- \
  audit-profile \
  research/jurisdictional-driving-transfer-001/fixtures/za-profile-draft-v0-1.json \
  /tmp/za-profile-audit.json
```

It should exit 0 with `disposition = valid_draft` and `policy_eligible = false`.

Audit the left/right metamorphic fixture:

```sh
cargo run --locked \
  --manifest-path research/jurisdictional-driving-transfer-001/Cargo.toml -- \
  audit-metamorphic-pair \
  research/jurisdictional-driving-transfer-001/fixtures/za-us-metamorphic-lht-rht-v0-1.json \
  /tmp/lht-rht-audit.json
```

Exit codes: 0 structurally valid, 1 rejected record (report is written), 2 invocation/input error. Input is bounded to 16 MiB and unknown fields are rejected by the Rust types.

## Benchmark integration

The jurisdiction track should evaluate zero-shot and fixed-budget adaptation across:
- LHT -> RHT and RHT -> LHT;
- profile present, missing, stale and contradicted by visual sign/marking evidence;
- steering-wheel side known, unknown and inconsistent with traffic side;
- signs, markings, signals, lane association, passing/turning and roundabout tasks;
- new region held-out from the training data.

Report route competence separately from legal compliance, sign-compliant destination, jurisdiction recognition/calibration, profile uncertainty, adaptation examples/interventions, worst rule-family performance, false-rule activation, fallback and latency/energy.

Use the native TrafficSignBench Sign-Compliant Destination metric and GeoDrive-Bench region reasoning accuracy as separate diagnostic/evaluation tracks. The former is not a generic driving score; the latter is open-loop reasoning, not closed-loop competence. South Africa-specific results must be identified as a new locally sourced profile/benchmark split and not represented as upstream GeoDrive-Bench coverage.

## Sources

- South African Department of Tourism, [Safety and Travel Tips — Exploring Mzansi by Road](https://www.tourism.gov.za/Pages/Safety--Travel-Tips-Info.aspx) (left-hand traffic guidance; corroboration only).
- South African Department of Transport, [SADC Road Traffic Signs Manual, Volume 4](https://www.transport.gov.za/wp-content/uploads/2023/02/V4C1.pdf) (edition/adoption/effectiveness review pending).
- South African Government, [National Road Traffic Act 93 of 1996](https://www.gov.za/documents/national-road-traffic-act) (amendments, commencement, regulations and current applicability review pending).
- [TrafficSignBench](https://github.com/emb-ai/traffic-sign-bench) (native Sign-Compliant Destination protocol; don't pool with route scores).
- [GeoDrive-Bench](https://arxiv.org/abs/2606.02774) (region-specific reasoning benchmark; published six-country coverage does not include South Africa).

## Authored tests

15 library tests and 6 CLI tests cover source URL/digest/revision requirements, profile eligibility gates, draft/reviewed distinction, Gregorian date validity, steering-side separation, duplicate source IDs, LHT/RHT geometry transforms, task identity preservation, maneuver mirror, invalid rule digests, i64 mirror overflow, checked-in fixtures and schema parsing.

These are authored tests. The exact-head CI workflow is the first compilation authority; no local Rust execution is claimed from the authoring environment.

## Claim ceiling

This validates finite profile/fixture structure and declared source provenance only. It does not fetch/verify source contents, decide legal interpretation, establish a complete South African rule set, prove regional behavior, measure driving performance, transfer to roads or provide a vehicle safety case.