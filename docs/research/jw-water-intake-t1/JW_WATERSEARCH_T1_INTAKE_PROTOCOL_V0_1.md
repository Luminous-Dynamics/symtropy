# Johannesburg Watersearch T1 intake — protocol v0.1

**Status:** Offline capture-verifier implementation; synthetic conformance only until a separate rights-reviewed publisher capture is supplied.  
**Date:** 17 November 2026  
**Parent:** Symtropy #5850, #5792 and #5799.

## 1. Why T1 exists

T0 proves that the evaluator preserves evidence boundaries on frozen synthetic inputs. T1 adds an offline, reproducible admission boundary for a future bounded Watersearch suburb-history response. The first implementation does not fetch the network. It consumes the exact body bytes, raw response-header bytes and capture manifest produced by an operator.

A tool-generated checksum is not an acquisition. A checksum over a synthetic fixture is not a publisher-data hash. These remain distinct evidence classes.

## 2. Source contract checked 17 November 2026

- API docs: https://watersearch.co.za/api
- OpenAPI 3.1: https://watersearch.co.za/api/v1/openapi.json
- Example route: GET /api/v1/suburbs/{slug}/history
- Release index: https://watersearch.co.za/api/v1/releases
- Bulk status: https://watersearch.co.za/data

The documented body includes licence, release, source_view, caveat, suburb, suburb_name and days. Each daily row contains day, asset_id, asset_name, level, bad, on_bypass, boards and wording. The level vocabulary is adequate, fair, pressure, low, no_water, critical and no_report. The API says each row is one suburb × linked supply × SAST day; no_report is a gap and never normal service.

The API says its compilation/readings are CC BY 4.0, asks for credit to Johannesburg Water for underlying facts, and places a licence statement in the body and X-Licence header. The license statement is a source assertion, not a legal opinion by this verifier. The source states it is not run by Johannesburg Water or the City.

As checked, API release 2026-11-17-ee99662f was advertised. The bulk download page listed current releases as "Not built here yet", so the first practical source path is one bounded suburb-history response, not a presumed bulk archive.

## 3. Capture procedure

For one suburb and an explicitly bounded period where practical:

1. Confirm current API docs, source page, release index, response licence field and X-Licence header.
2. Capture the exact HTTP response body bytes and the raw response header block. Do not pretty-print, normalize or reserialize the body before hashing.
3. Record GET URL, final URL, HTTP status, capture timestamp in UTC, content type, response headers, suburb slug, requested from/to dates and advertised release ID.
4. Hash the raw body and raw header bytes independently; record byte counts and SHA-256 values in the capture manifest.
5. Run the offline verifier over those exact files.
6. Keep rights review separate. A pending rights review yields an integrity-checked but not admitted receipt. A rights-approved receipt must cite a human review reference and still only admits publisher-reported asset status.
7. Preserve original JW report/notice links where available in a later source-lineage admission step; do not turn this API summary into a primary JW record.

The CI fixture uses fixture.invalid and SYNTHETIC-ONLY values. It is not a live source capture and is not a license assertion.

## 4. Fail-closed checks

- Raw body/header byte counts and SHA-256 values exactly match the manifest.
- The raw header block parses to HTTP 200 and agrees with the normalized content-type and X-Licence manifest fields.
- Request and final URL are identical; publisher mode is HTTPS and confined to watersearch.co.za's suburb-history route.
- Body release, suburb and source_view match the manifest request; publisher release IDs use YYYY-MM-DD-8hex format.
- Body licence and X-Licence must match. Publisher mode requires the stated CC BY 4.0 assertion, but rights remain pending until explicitly reviewed.
- Every row has valid SAST date, unique (asset_id, day), known level, consistent bad flag, boolean bypass, board count and non-empty wording.
- no_report is preserved explicitly and must have zero boards; it is never filled or converted to fair/adequate.
- Requested from/to bounds are inclusive and every returned day must fall inside them. The verifier does not fill absent rows or infer service outside the publisher response.
- Unknown fields and unknown status levels fail closed under the versioned response schema.

## 5. Receipt semantics

The receipt binds body bytes, raw header bytes, capture manifest, response schema, package manifest, Cargo.lock, implementation and exact checkout revision.

For synthetic fixture mode:
- evidence_class = synthetic_only;
- qualification_status = NotRun;
- admission_status = not_admitted_synthetic_conformance.

For publisher capture mode:
- evidence_class = publisher_reported_asset_status;
- integrity_status = pass only if all byte, header and semantic checks pass;
- rights_review pending => admission_status = pending_rights_review;
- rights_review approved with a non-empty reference => admission_status = rights_review_marked_approved_source_integrity_checked.

Even the last state does not establish household tap status, pressure/flow, water quality, physically verified restoration, pipe-break cause, causal hydrology, forecasts, water savings or regulatory compliance.

## 6. Reproducibility and nonclaims

Do not commit real source response bytes to a synthetic fixture or use live personal/household data in the conformance suite. Future captured data may be kept outside the repository while rights review is pending. Any later admitted data release must have its own source-bound manifest and immutable digest.

A passing CI job proves only that the offline verifier behaves on synthetic fixtures. It does not mean a Watersearch capture has been acquired or admitted.
