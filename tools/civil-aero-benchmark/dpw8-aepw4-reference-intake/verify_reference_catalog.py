#!/usr/bin/env python3
"""Fail-closed validator for the DPW-8/AePW-4 source-discovery catalog.

Only the Python standard library is required. This validates the catalog's
internal consistency and explicit unknown states; it does not authenticate
NASA/AIAA, download artifacts, validate licenses, or qualify any physics.
"""

from __future__ import annotations

import copy
import json
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
CATALOG = HERE / "dpw8-reference-catalog-v0.2.json"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_ACQUISITION = {
    "page_inspected",
    "listing_only",
    "text_preview_inspected_exact_bytes_not_acquired",
    "linked_file_not_acquired",
    "linked_files_not_acquired",
}
ADMITTED_TERMS = {"reviewed_permitted", "public_domain_or_open_license_verified"}
ADMITTED_ACQUISITION = {"bytes_acquired_and_verified"}


class CatalogError(ValueError):
    """Raised when a catalog invariant is violated."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CatalogError(message)


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def validate_source(source: dict[str, Any]) -> None:
    source_id = source.get("source_id")
    require(isinstance(source_id, str) and source_id.strip(), "source_id must be non-empty")
    for key in ("canonical_url", "evidence_url"):
        value = source.get(key)
        require(
            isinstance(value, str) and value.startswith("https://"),
            f"{source_id}.{key} must be an HTTPS URL",
        )

    exact = source.get("exact_bytes_acquired")
    require(isinstance(exact, bool), f"{source_id}.exact_bytes_acquired must be boolean")
    digest = source.get("sha256")
    acquisition = source.get("acquisition_state")
    terms = source.get("terms_status")

    if exact:
        require(
            isinstance(digest, str) and SHA256_RE.fullmatch(digest) is not None,
            f"{source_id}: exact acquired bytes require a lowercase SHA-256",
        )
        require(
            acquisition in ADMITTED_ACQUISITION,
            f"{source_id}: exact bytes cannot be marked acquired without the acquired state",
        )
    else:
        require(
            digest is None,
            f"{source_id}: do not publish a byte digest when exact source bytes were not acquired",
        )
        require(
            acquisition in ALLOWED_ACQUISITION,
            f"{source_id}: unknown or falsely promoted acquisition state",
        )

    # v0.1 is deliberately discovery-only. No source is admitted until bytes,
    # terms, content classification, units and applicability are reviewed.
    require(
        terms not in ADMITTED_TERMS,
        f"{source_id}: v0.1 discovery catalog cannot assert terms clearance",
    )
    require(
        acquisition not in ADMITTED_ACQUISITION,
        f"{source_id}: v0.1 discovery catalog cannot assert artifact admission",
    )

    attempts = source.get("retrieval_attempts", [])
    require(isinstance(attempts, list), f"{source_id}.retrieval_attempts must be a list")
    for index, attempt in enumerate(attempts):
        require(isinstance(attempt, dict), f"{source_id}.retrieval_attempts[{index}] must be an object")
        require(attempt.get("requested_url") == source["canonical_url"],
                f"{source_id}: retrieval attempt URL differs from canonical source")
        require(isinstance(attempt.get("attempted_on"), str) and attempt["attempted_on"],
                f"{source_id}: retrieval attempt date missing")
        require(isinstance(attempt.get("method"), str) and attempt["method"],
                f"{source_id}: retrieval method missing")
        received = attempt.get("exact_bytes_received")
        require(isinstance(received, bool), f"{source_id}: exact_bytes_received must be boolean")
        attempt_sha = attempt.get("sha256")
        if received:
            require(
                isinstance(attempt_sha, str) and SHA256_RE.fullmatch(attempt_sha) is not None,
                f"{source_id}: received bytes require an exact SHA-256",
            )
            require(exact and digest == attempt_sha,
                    f"{source_id}: received-byte attempt must match the admitted source digest")
        else:
            require(attempt_sha is None,
                    f"{source_id}: no digest may be recorded when a retrieval attempt returned no bytes")
            require(attempt.get("outcome") == "failed_no_bytes_returned",
                    f"{source_id}: failed retrieval must remain explicitly not acquired")

    listed_size = source.get("listed_size")
    if listed_size is not None:
        require(
            isinstance(listed_size, str) and listed_size.strip(),
            f"{source_id}.listed_size must be null or a non-empty publisher label",
        )

    for fact in ("observed_facts", "unknowns"):
        values = source.get(fact)
        require(
            isinstance(values, list) and all(isinstance(item, str) and item.strip() for item in values),
            f"{source_id}.{fact} must be a list of non-empty strings",
        )

    if source_id == "crm-halfspan-equivalent-beam-bdf":
        require(
            acquisition == "text_preview_inspected_exact_bytes_not_acquired",
            "equivalent beam source must retain its text-preview-only acquisition state",
        )
        observed = " ".join(source["observed_facts"])
        for card in ("SOL 101", "CBEAM", "PBEAM", "CONM2", "RBE2", "SPC", "FORCE"):
            require(card in observed, f"equivalent beam preview inventory missing {card}")
        require(
            any("unit interpretation is unknown" in item for item in source["observed_facts"]),
            "equivalent beam material unit ambiguity must remain explicit",
        )
        require(
            any("unit system" in item for item in source["known_unknowns"]),
            "equivalent beam property unit uncertainty must remain explicit",
        )


def validate_case(case: dict[str, Any], source_ids: set[str]) -> None:
    case_id = case.get("case_id")
    require(isinstance(case_id, str) and case_id.strip(), "case_id must be non-empty")
    for key in ("authority_url", "working_group_url"):
        url = case.get(key)
        require(isinstance(url, str) and url.startswith("https://"), f"{case_id}.{key} must be HTTPS")

    conditions = case.get("conditions")
    require(isinstance(conditions, dict), f"{case_id}.conditions must be an object")
    for key in ("mach", "reynolds_chord"):
        if key in conditions:
            require(
                is_number(conditions[key]) and math_is_positive(conditions[key]),
                f"{case_id}.{key} must be a positive number",
            )

    grid = case.get("grid_family")
    if grid is not None:
        require(isinstance(grid, dict), f"{case_id}.grid_family must be an object")
        levels = grid.get("levels")
        if levels is not None:
            require(
                isinstance(levels, list)
                and levels
                and all(isinstance(level, int) and not isinstance(level, bool) and level > 0 for level in levels),
                f"{case_id}.grid_family.levels must be positive integer levels",
            )
            require(levels == sorted(set(levels)), f"{case_id}.grid_family.levels must be unique and ordered")
        if "minimum_required_members" in grid and "desirable_members" in grid:
            minimum = grid["minimum_required_members"]
            desirable = grid["desirable_members"]
            require(
                isinstance(minimum, int)
                and isinstance(desirable, int)
                and 1 <= minimum <= desirable,
                f"{case_id}: required grid member count exceeds desirable member count",
            )

    stations = case.get("measured_span_stations_eta")
    if stations is not None:
        require(
            isinstance(stations, list)
            and len(stations) >= 2
            and all(is_number(x) and 0.0 <= float(x) <= 1.0 for x in stations),
            f"{case_id}: normalized span stations must lie in [0, 1]",
        )
        require(
            all(float(stations[i + 1]) > float(stations[i]) for i in range(len(stations) - 1)),
            f"{case_id}: normalized span stations must be strictly increasing",
        )

    refs = case.get("structural_reference_source_ids", [])
    require(isinstance(refs, list), f"{case_id}.structural_reference_source_ids must be a list")
    for source_id in refs:
        require(source_id in source_ids, f"{case_id}: dangling structural source reference {source_id}")

    case_sheet_refs = case.get("case_sheet_source_ids", [])
    require(isinstance(case_sheet_refs, list), f"{case_id}.case_sheet_source_ids must be a list")
    for source_id in case_sheet_refs:
        require(source_id in source_ids, f"{case_id}: dangling case-sheet source reference {source_id}")

    data_refs = case.get("data_source_ids", [])
    require(isinstance(data_refs, list), f"{case_id}.data_source_ids must be a list")
    for source_id in data_refs:
        require(source_id in source_ids, f"{case_id}: dangling data source reference {source_id}")

    for channel in case.get("measurement_channels", []):
        require(isinstance(channel, dict), f"{case_id}: measurement channel must be an object")
        source_id = channel.get("source_id")
        require(source_id in source_ids, f"{case_id}: dangling measurement-channel source {source_id}")

    ceiling = case.get("claim_ceiling")
    require(isinstance(ceiling, str) and ceiling.strip(), f"{case_id}: explicit claim ceiling required")


def math_is_positive(value: int | float) -> bool:
    return value > 0


def validate_catalog(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-reference-catalog/0.2.0", "unsupported schema_version")
    require(data.get("claim_ceiling") == "public_source_discovery_and_partial_content_inspection_only",
            "catalog claim ceiling was weakened or changed")
    policy = data.get("acquisition_policy")
    require(isinstance(policy, dict), "acquisition_policy missing")
    for key in (
        "exact_bytes_required_before_admission",
        "no_inferred_units_or_terms",
        "unknown_is_not_zero_or_pass",
        "archives_are_not_extracted_until_safety_limits_are_set",
    ):
        require(policy.get(key) is True, f"acquisition policy invariant missing: {key}")

    sources = data.get("sources")
    cases = data.get("benchmark_cases")
    require(isinstance(sources, list) and sources, "sources must be a non-empty list")
    require(isinstance(cases, list) and cases, "benchmark_cases must be a non-empty list")

    source_ids = [source.get("source_id") for source in sources]
    require(
        all(isinstance(value, str) and value for value in source_ids),
        "every source needs a non-empty source_id",
    )
    require(len(source_ids) == len(set(source_ids)), "duplicate source_id")
    source_id_set = set(source_ids)

    for source in sources:
        require(isinstance(source, dict), "each source must be an object")
        validate_source(source)

    case_ids = [case.get("case_id") for case in cases]
    require(
        all(isinstance(value, str) and value for value in case_ids),
        "every benchmark case needs a non-empty case_id",
    )
    require(len(case_ids) == len(set(case_ids)), "duplicate benchmark case_id")
    for case in cases:
        require(isinstance(case, dict), "each benchmark case must be an object")
        validate_case(case, source_id_set)

    # Cross-check the important known-answer reference relationships.
    oat = next((case for case in cases if case["case_id"] == "DPW8-OAT15A-V1A"), None)
    require(oat is not None, "required OAT15A Verification 1a case missing")
    require(oat["conditions"].get("mach") == 0.73, "OAT15A Mach reference mismatch")
    require(oat["conditions"].get("reynolds_chord") == 3_000_000, "OAT15A Reynolds reference mismatch")
    require(oat["conditions"].get("angles_of_attack_deg") == [1.36, 1.5, 2.5, 3.0, 3.1],
            "OAT15A angle schedule changed")

    static_fsi = next((case for case in cases if case["case_id"] == "DPW8-CRM-STATIC-FSI-2A-E"), None)
    require(static_fsi is not None, "CRM static FSI 2a(E) case missing")
    require(static_fsi["conditions"].get("mach") == 0.85, "CRM FSI Mach reference mismatch")
    require(static_fsi["conditions"].get("dynamic_pressure_psf") == 1384.0,
            "CRM FSI dynamic-pressure reference mismatch")
    require(static_fsi["measured_span_stations_eta"] == [0.0, 0.4286, 0.5546, 0.6773, 0.7954, 0.9150],
            "CRM measured span station schedule changed")

    require(static_fsi["conditions"].get("lift_coefficient_target") == 0.5,
            "CRM FSI 2a(E) lift coefficient target mismatch")
    require(static_fsi["conditions"].get("lift_coefficient_tolerance") == 0.0001,
            "CRM FSI 2a(E) lift coefficient tolerance mismatch")
    require(static_fsi["conditions"].get("angle_of_attack_approx_deg") == 2.75,
            "CRM FSI 2a(E) approximate trim angle mismatch")
    require("angle_of_attack_deg" not in static_fsi["conditions"],
            "CRM FSI 2a(E) must not substitute rigid 2a's exact 2.50 degree angle")
    require(static_fsi["conditions"].get("static_temperature_f") == 120.0,
            "CRM FSI 2a(E) temperature in degrees F mismatch")
    require(static_fsi["conditions"].get("static_temperature_k") == 322.04,
            "CRM FSI 2a(E) temperature in kelvin mismatch")
    require(static_fsi["conditions"].get("initial_geometry") == "jig_unloaded_wind_off",
            "CRM FSI 2a(E) initial geometry mismatch")

    static_polar = next((case for case in cases if case["case_id"] == "DPW8-CRM-STATIC-FSI-2B-E"), None)
    require(static_polar is not None, "CRM static FSI 2b(E) case missing")
    require(static_polar["conditions"].get("angles_of_attack_deg") ==
            [-1.5, 0.0, 1.5, 2.5, 3.0, 3.5, 4.0, 4.5],
            "CRM static FSI 2b(E) angle schedule mismatch")
    require(static_polar["conditions"].get("static_temperature_f") == 120.0,
            "CRM static FSI 2b(E) temperature mismatch")
    require(static_polar["conditions"].get("static_temperature_k") == 322.04,
            "CRM static FSI 2b(E) kelvin mismatch")
    require(static_polar["conditions"].get("initial_geometry") == "jig_unloaded_wind_off",
            "CRM static FSI 2b(E) initial geometry mismatch")
    require(static_polar["grid_family"].get("levels") == [3],
            "CRM static FSI 2b(E) grid level mismatch")

    rigid_grid = next((case for case in cases if case["case_id"] == "DPW8-CRM-RIGID-2A"), None)
    rigid_polar = next((case for case in cases if case["case_id"] == "DPW8-CRM-RIGID-2B"), None)
    require(rigid_grid is not None and rigid_polar is not None,
            "rigid CRM Configuration B cases must remain separate from FSI")
    require(rigid_grid["conditions"].get("angle_of_attack_deg") == 2.5,
            "Rigid Configuration B 2a must remain alpha=2.50 deg")
    require(static_fsi["case_sheet_source_ids"] == ["aiaa-static-fsi-2ae-2be-case-sheet"],
            "Elastic 2a(E) must bind its dedicated AIAA case sheet")
    require(static_polar["case_sheet_source_ids"] == ["aiaa-static-fsi-2ae-2be-case-sheet"],
            "Elastic 2b(E) must bind its dedicated AIAA case sheet")
    for case in (rigid_grid, rigid_polar):
        require(case["conditions"].get("static_temperature_f") == 100.0,
                f"{case['case_id']}: rigid Configuration B temperature mismatch")
        require(case["conditions"].get("static_temperature_k") == 310.928,
                f"{case['case_id']}: rigid Configuration B kelvin mismatch")
    require(rigid_polar["conditions"].get("angles_of_attack_deg") ==
            [-1.5, 2.5, 2.75, 3.0, 3.25, 3.5, 3.75, 4.0, 4.25],
            "rigid CRM Configuration B polar schedule mismatch")

    # DPW-8 Buffet Working Group Test Case 3 is optional and dynamic.
    # Its measurement streams have different operators, bandwidths and frames;
    # do not promote them into one interchangeable response vector.
    dynamic = next((case for case in cases if case["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3"), None)
    require(dynamic is not None, "CRM dynamic FSI Test Case 3 missing")
    require(dynamic.get("required") is False, "CRM Test Case 3 must remain optional")
    require(dynamic.get("workshop_status") == "optional_buffet_working_group_case",
            "CRM Test Case 3 optional status changed")
    conditions = dynamic.get("conditions", {})
    require(conditions.get("mach") == 0.85, "CRM Test Case 3 Mach mismatch")
    require(conditions.get("reynolds_chord") == 2_270_000, "CRM Test Case 3 Reynolds mismatch")
    require(conditions.get("static_temperature_k") == 326.15, "CRM Test Case 3 static temperature mismatch")
    require(conditions.get("total_pressure_kpa") == 12.0, "CRM Test Case 3 total pressure mismatch")
    require(conditions.get("angles_of_attack_deg") == [3.05, 3.61, 4.70],
            "CRM Test Case 3 angle schedule mismatch")
    require(conditions.get("initial_geometry") == "unloaded_wind_off",
            "CRM Test Case 3 must start from unloaded wind-off geometry")
    require(dynamic.get("grid_family", {}).get("levels") == [1, 2, 3, 4, 5, 6],
            "CRM Test Case 3 grid levels mismatch")

    measurement_channels = dynamic.get("measurement_channels", [])
    by_family = {item.get("channel_family"): item for item in measurement_channels}
    expected_families = {
        "integrated_balance",
        "buffet_intensity",
        "steady_pressure",
        "unsteady_point_pressure",
        "unsteady_surface_pressure",
        "structural_deformation",
    }
    require(set(by_family) == expected_families, "CRM Test Case 3 measurement families changed")
    require(
        any("CD is N/A" in item for item in by_family["integrated_balance"].get("limitations", [])),
        "PSP-simultaneous drag missingness must remain explicit",
    )
    require(
        "Kulite" in by_family["unsteady_point_pressure"].get("instrument_or_method", ""),
        "unsteady point pressure must preserve Kulite provenance",
    )
    require(
        "uPSP" in by_family["unsteady_surface_pressure"].get("instrument_or_method", ""),
        "surface pressure must preserve uPSP provenance",
    )
    upsp_limits = " ".join(by_family["unsteady_surface_pressure"].get("limitations", []))
    require("low-pass" in upsp_limits, "uPSP camera low-pass effect must remain explicit")
    require("masks" in upsp_limits, "uPSP reliability masks must remain explicit")
    kulite_limits = " ".join(by_family["unsteady_point_pressure"].get("limitations", []))
    require("9 kHz" in kulite_limits and "wind-tunnel noise" in kulite_limits,
            "9 kHz tunnel-noise PSD feature must not be promoted to a structural mode")

    structural_channel = by_family["structural_deformation"]
    require(structural_channel.get("evidence_status") == "PENDING_NOT_PUBLISHED",
            "official dynamic structural-deformation evidence must remain pending")
    deformation = dynamic.get("deformation_evidence", {})
    require(deformation.get("official_measurements_status") == "pending",
            "official dynamic deformation measurements must remain pending")
    require(deformation.get("preliminary_tip_deflection_mm") == [10, 12],
            "preliminary JAXA tip-deflection note changed")
    require(deformation.get("preliminary_angle_of_attack_deg") == 4.70,
            "preliminary tip-deflection point must remain at alpha=4.70 deg")
    require(deformation.get("preliminary_evidence_class") == "qualitative_only_not_validation",
            "preliminary tip-deflection point cannot be promoted to validation data")
    require(deformation.get("allow_as_quantitative_acceptance_threshold") is False,
            "preliminary tip-deflection must not become a quantitative acceptance threshold")

    spectral = dynamic.get("spectral_processing", {})
    require(spectral.get("status") == "must_be_admitted_from_case3_artifact_headers_or_controlling_procedure",
            "Test Case 3 spectral processing must be source-admitted")
    require(spectral.get("time_step_s") is None and spectral.get("sampling_rate_hz") is None,
            "Test Case 3 sampling values must not be invented")
    require(spectral.get("window") is None and spectral.get("segment_length") is None
            and spectral.get("overlap") is None and spectral.get("psd_normalization") is None,
            "Test Case 3 PSD settings must remain unknown until source admission")
    require("Do not copy Test Case 2" in spectral.get("explicit_non_inheritance", ""),
            "Test Case 2 PSD settings must not be inherited into Test Case 3")

    jaxa_index = next((source for source in sources if source["source_id"] == "jaxa-dpw8-test3-experimental-data-index"), None)
    require(jaxa_index is not None, "JAXA Test Case 3 data index source missing")
    require(jaxa_index.get("terms_status") == "all_rights_reserved_review_required",
            "JAXA copyright/terms review state was changed")
    require(jaxa_index.get("exact_bytes_acquired") is False and jaxa_index.get("sha256") is None,
            "JAXA page inspection must not be promoted to acquired data bytes")
    deformation_source = next((source for source in sources if source["source_id"] == "jaxa-dpw8-test3-preliminary-deformation-note"), None)
    require(deformation_source is not None, "JAXA preliminary deformation note source missing")
    require(any("official deformation measurements" in fact and "still pending" in fact
                for fact in deformation_source.get("observed_facts", [])),
            "JAXA official-deformation-pending statement must remain visible")

    format_observations = data.get("data_format_observations")
    require(isinstance(format_observations, dict), "data_format_observations missing")
    require(
        format_observations.get("optional_missing_value_sentinel") == -999,
        "AIAA optional-data sentinel must remain -999",
    )
    require(
        "normalized by reference semi-span" in format_observations.get("normalization_note", ""),
        "bending/twist normalization semantics missing",
    )


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (CatalogError, KeyError, TypeError):
        print(f"negative_control=pass case={label}")
        return
    raise CatalogError(f"negative control unexpectedly passed: {label}")


def run_negative_controls(data: dict[str, Any]) -> None:
    bad = copy.deepcopy(data)
    bdf = next(x for x in bad["sources"] if x["source_id"] == "crm-halfspan-equivalent-beam-bdf")
    bdf["exact_bytes_acquired"] = True
    expect_rejected("admit_source_without_exact_digest", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    bdf = next(x for x in bad["sources"] if x["source_id"] == "crm-halfspan-equivalent-beam-bdf")
    bdf["sha256"] = "0" * 64
    expect_rejected("invent_digest_for_unacquired_preview", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    bdf = next(x for x in bad["sources"] if x["source_id"] == "crm-halfspan-equivalent-beam-bdf")
    bdf["retrieval_attempts"][0]["exact_bytes_received"] = True
    expect_rejected("failed_retrieval_cannot_be_promoted_to_acquired_bytes", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-STATIC-FSI-2A-E")
    case["measured_span_stations_eta"][2] = 1.2
    expect_rejected("span_station_outside_normalized_domain", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-STATIC-FSI-2A-E")
    case["structural_reference_source_ids"].append("not-a-real-source")
    expect_rejected("dangling_source_reference", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-OAT15A-V1A")
    case["grid_family"]["minimum_required_members"] = 7
    expect_rejected("required_grid_count_exceeds_desirable", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    bad["data_format_observations"]["optional_missing_value_sentinel"] = 0
    expect_rejected("missing_value_sentinel_mutation", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    case["required"] = True
    expect_rejected("optional_dynamic_case_not_silently_promoted_to_required", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    case["deformation_evidence"]["preliminary_evidence_class"] = "quantitative_validation"
    expect_rejected("preliminary_tip_deflection_not_validation", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    case["spectral_processing"]["sampling_rate_hz"] = 50000.0
    expect_rejected("unadmitted_sampling_rate_not_invented", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    case["measurement_channels"] = [
        item for item in case["measurement_channels"] if item["channel_family"] != "unsteady_surface_pressure"
    ]
    expect_rejected("uPSP_measurement_operator_cannot_be_omitted", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    by_family = {item["channel_family"]: item for item in case["measurement_channels"]}
    by_family["unsteady_surface_pressure"]["limitations"] = [
        "uPSP data provide a direct unfiltered surface pressure signal"
    ]
    expect_rejected("uPSP_low_pass_and_mask_limitations_must_remain", lambda: validate_catalog(bad))

    bad = copy.deepcopy(data)
    case = next(x for x in bad["benchmark_cases"] if x["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
    by_family = {item["channel_family"]: item for item in case["measurement_channels"]}
    by_family["unsteady_point_pressure"]["limitations"] = [
        "9 kHz is the main structural resonance"
    ]
    expect_rejected("wind_tunnel_9khz_peak_not_structural_mode", lambda: validate_catalog(bad))


def main() -> int:
    try:
        data = json.loads(CATALOG.read_text(encoding="utf-8"))
        validate_catalog(data)
        run_negative_controls(data)
        print(
            "dpw8_reference_catalog=pass "
            f"sources={len(data['sources'])} "
            f"cases={len(data['benchmark_cases'])} "
            "admitted_artifacts=0 "
            "claim_ceiling=discovery_and_partial_preview_only"
        )
    except (OSError, json.JSONDecodeError, CatalogError, KeyError, TypeError) as error:
        print(f"dpw8_reference_catalog_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
