#!/usr/bin/env python3
"""Fail-closed validator for the AER-01 low-speed propeller/wing source catalog.

The catalog is discovery / partial paper inspection only. This verifier checks
internal provenance, separation of component vs installed-interaction evidence,
channel-specific metrology states and immutable claim ceilings. It does not
download/parse experimental data or qualify any aerodynamic model.
"""

from __future__ import annotations

import copy
import json
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
CATALOG = HERE / "reference-catalog-v0.1.json"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
ADMITTED_TERMS = {"reviewed_permitted", "public_domain_or_open_license_verified"}
ADMITTED_STATE = "bytes_acquired_and_verified"
DISCOVERY_STATES = {
    "page_inspected",
    "page_and_tables_index_inspected",
    "abstract_and_metadata_inspected",
    "pdf_text_and_selected_page_images_inspected_exact_bytes_not_acquired",
    "index_abstract_inspected",
}


class CatalogError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CatalogError(message)


def validate_https(url: Any, label: str) -> None:
    require(isinstance(url, str) and url.startswith("https://"), f"{label} must be an HTTPS URL")


def validate_source(source: dict[str, Any]) -> None:
    source_id = source.get("source_id")
    require(isinstance(source_id, str) and source_id.strip(), "source_id must be non-empty")
    validate_https(source.get("canonical_url"), f"{source_id}.canonical_url")
    if source.get("full_text_url") is not None:
        validate_https(source["full_text_url"], f"{source_id}.full_text_url")

    acquired = source.get("exact_bytes_acquired")
    require(isinstance(acquired, bool), f"{source_id}.exact_bytes_acquired must be boolean")
    sha = source.get("sha256")
    state = source.get("acquisition_state")
    if acquired:
        require(isinstance(sha, str) and SHA256.fullmatch(sha) is not None,
                f"{source_id}: exact bytes require a lowercase SHA-256")
        require(state == ADMITTED_STATE, f"{source_id}: exact bytes need admitted acquisition state")
        require(source.get("terms_status") in ADMITTED_TERMS,
                f"{source_id}: exact bytes cannot be admitted before terms review")
    else:
        require(sha is None, f"{source_id}: no digest may be claimed without exact bytes")
        require(state in DISCOVERY_STATES, f"{source_id}: unsupported discovery/acquisition state")
        require(source.get("terms_status") not in ADMITTED_TERMS,
                f"{source_id}: discovery-only v0.1 cannot claim license/terms clearance")
        require(source.get("redistribution_status") == "not_decided",
                f"{source_id}: discovery-only source cannot claim redistribution permission")

    facts = source.get("observed_facts")
    unknowns = source.get("unknowns")
    require(isinstance(facts, list) and facts and all(isinstance(x, str) and x.strip() for x in facts),
            f"{source_id}: observed_facts must be non-empty strings")
    require(isinstance(unknowns, list) and all(isinstance(x, str) and x.strip() for x in unknowns),
            f"{source_id}: unknowns must be strings")

    if source_id.startswith("uiuc-propeller-volume"):
        require(source.get("artifact_role") == "isolated_propeller_thrust_torque_power_and_geometry",
                f"{source_id}: UIUC volume role changed")
        require(source.get("claim_ceiling") == "isolated_propeller_component_map_only",
                f"{source_id}: UIUC component map cannot claim installed interaction validation")

    if source_id == "nasa-x57-high-lift-propeller-test-2021":
        quality = source.get("measurement_quality", {})
        require("cross-loading" in quality.get("thrust_channel", ""),
                "NASA X-57 thrust cross-loading warning was removed")
        require("thermal drift" in quality.get("thrust_channel", ""),
                "NASA X-57 thrust thermal-drift warning was removed")
        require("reliable" in quality.get("torque_channel", ""),
                "NASA X-57 torque source-reported reliability distinction was removed")
        require(source.get("claim_ceiling") == "full_scale_propeller_component_stand_test_only",
                "NASA component stand cannot be promoted to integrated aircraft validation")


def validate_catalog(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-low-speed-propwing-reference-catalog/0.1.0",
            "unsupported catalog schema")
    require(data.get("claim_ceiling") == "public_source_discovery_and_partial_paper_inspection_only",
            "catalog claim ceiling changed")
    policy = data.get("source_admission_policy", {})
    for key in (
        "exact_bytes_and_sha256_required_before_admission",
        "terms_and_redistribution_review_required",
        "component_data_not_installed_interaction_validation",
        "measurement_uncertainty_is_channel_specific",
        "unknown_is_not_zero_or_pass",
        "no_extrapolation_outside_source_domain",
    ):
        require(policy.get(key) is True, f"source policy invariant missing: {key}")

    sources = data.get("sources")
    tracks = data.get("benchmark_tracks")
    require(isinstance(sources, list) and sources, "sources must be non-empty")
    require(isinstance(tracks, list) and tracks, "benchmark_tracks must be non-empty")

    ids = [source.get("source_id") for source in sources]
    require(all(isinstance(x, str) and x for x in ids), "source_id missing")
    require(len(ids) == len(set(ids)), "duplicate source_id")
    source_ids = set(ids)
    for source in sources:
        require(isinstance(source, dict), "each source must be an object")
        validate_source(source)

    track_ids = [track.get("track_id") for track in tracks]
    require(len(track_ids) == len(set(track_ids)), "duplicate benchmark track")
    for track in tracks:
        require(isinstance(track.get("claim_ceiling"), str) and track["claim_ceiling"],
                f"{track.get('track_id')}: claim ceiling required")
        refs = track.get("source_ids")
        require(isinstance(refs, list) and refs, f"{track.get('track_id')}: source IDs required")
        for source_id in refs:
            require(source_id in source_ids, f"{track.get('track_id')}: dangling source {source_id}")

    component = next(x for x in tracks if x["track_id"] == "UIUC-COMPONENT-MAPS")
    require(component["role"] == "isolated_propeller_component_reference",
            "UIUC component track role must remain isolated")
    require("installed_wing_lift_augmentation" in component.get("must_not_claim", []),
            "UIUC component track must explicitly disallow installed wing claim")

    mav = next(x for x in tracks if x["track_id"] == "LOW-RE-MAV-INSTALLED-FLOW")
    require(mav["conditions"]["freestream_velocity_m_per_s"] == 9.0, "MAV reference speed mismatch")
    require(mav["conditions"]["reynolds_root_chord"] == 135000, "MAV Reynolds reference mismatch")
    require(mav["conditions"]["propeller_rpm"] == [8000, 10000], "MAV RPM interval mismatch")

    x57 = next(x for x in tracks if x["track_id"] == "NASA-X57-HLP-COMPONENT-STAND")
    require(x57["conditions"]["freestream_KEAS"] == [0, 35, 58, 75, 90],
            "X-57 test speed schedule mismatch")
    x57_source = next(x for x in sources if x["source_id"] == "nasa-x57-high-lift-propeller-test-2021")
    require("cross-loading" in x57_source["measurement_quality"]["thrust_channel"],
            "X-57 per-channel thrust quality not preserved")

    constraints = data.get("global_claim_constraints")
    require(isinstance(constraints, list) and constraints, "global claim constraints missing")
    joined = " ".join(constraints).lower()
    for phrase in (
        "isolated component evidence",
        "not direct validation",
        "channel-specific",
    ):
        require(phrase in joined, f"global claim constraint missing: {phrase}")


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (CatalogError, KeyError, StopIteration, TypeError):
        print(f"negative_control=pass case={label}")
        return
    raise CatalogError(f"negative control unexpectedly passed: {label}")


def hostile_controls(data: dict[str, Any]) -> None:
    bad = copy.deepcopy(data)
    source = next(x for x in bad["sources"] if x["source_id"] == "uiuc-propeller-volume-1")
    source["claim_ceiling"] = "installed_propeller_wing_validation"
    expect_rejected("component_data_promoted_to_installed_validation", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    source = next(x for x in bad["sources"] if x["source_id"] == "nasa-x57-high-lift-propeller-test-2021")
    source["measurement_quality"]["thrust_channel"] = "precise_and_reliable"
    expect_rejected("thrust_cross_loading_warning_removed", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    source = next(x for x in bad["sources"] if x["source_id"] == "uiuc-propeller-volume-2")
    source["exact_bytes_acquired"] = True
    expect_rejected("source_admitted_without_digest", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    source = next(x for x in bad["sources"] if x["source_id"] == "mav-propeller-induced-wing-flow-2014")
    source["sha256"] = "0" * 64
    expect_rejected("invented_digest_for_abstract_only_source", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    track = next(x for x in bad["benchmark_tracks"] if x["track_id"] == "LOW-RE-MAV-INSTALLED-FLOW")
    track["conditions"]["reynolds_root_chord"] = 0
    expect_rejected("invalid_low_re_reference_condition", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    track = next(x for x in bad["benchmark_tracks"] if x["track_id"] == "NASA-X57-HLP-COMPONENT-STAND")
    track["source_ids"].append("not-a-real-source")
    expect_rejected("dangling_benchmark_source_reference", lambda: validate_catalog(bad))


def main() -> int:
    try:
        data = json.loads(CATALOG.read_text(encoding="utf-8"))
        validate_catalog(data)
        hostile_controls(data)
        admitted = sum(1 for source in data["sources"] if source["exact_bytes_acquired"])
        print(
            "low_speed_propwing_catalog=pass "
            f"sources={len(data['sources'])} tracks={len(data['benchmark_tracks'])} "
            f"admitted_artifacts={admitted} "
            "claim_ceiling=discovery_and_partial_paper_inspection_only"
        )
    except (OSError, json.JSONDecodeError, CatalogError, KeyError, TypeError) as error:
        print(f"low_speed_propwing_catalog_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
