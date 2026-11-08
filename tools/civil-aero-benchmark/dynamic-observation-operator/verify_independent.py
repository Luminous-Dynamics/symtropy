#!/usr/bin/env python3
"""Independent verifier for the AER-01 dynamic observation-operator contract.

This parser does not import or execute the Rust implementation. It independently
checks the DPW8 source catalog, validates the dynamic measurement restrictions,
and compares the Rust audit output to the independently derived expected report.

All positive numeric examples live in synthetic in-memory fixtures. The JAXA
Test Case 3 files remain unacquired and are never treated as measured data here.
"""

from __future__ import annotations

import copy
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
CATALOG = (
    HERE.parent
    / "dpw8-aepw4-reference-intake"
    / "dpw8-reference-catalog-v0.2.json"
)


class AuditError(ValueError):
    """Raised when a reference or comparison violates the frozen contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def find_case(catalog: dict[str, Any]) -> dict[str, Any]:
    cases = catalog.get("benchmark_cases")
    require(isinstance(cases, list), "benchmark_cases missing")
    matches = [item for item in cases if item.get("case_id") == "DPW8-CRM-DYNAMIC-FSI-3"]
    require(len(matches) == 1, "exactly one DPW8-CRM-DYNAMIC-FSI-3 case is required")
    return matches[0]


def find_sources(catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    sources = catalog.get("sources")
    require(isinstance(sources, list), "sources missing")
    result: dict[str, dict[str, Any]] = {}
    for source in sources:
        source_id = source.get("source_id")
        require(isinstance(source_id, str) and source_id, "source_id missing")
        require(source_id not in result, f"duplicate source_id {source_id}")
        result[source_id] = source
    return result


def is_source_admitted(source: dict[str, Any]) -> bool:
    digest = source.get("sha256")
    return (
        source.get("exact_bytes_acquired") is True
        and source.get("acquisition_state") == "bytes_acquired_and_verified"
        and source.get("terms_status")
        in {"reviewed_permitted", "public_domain_or_open_license_verified"}
        and isinstance(digest, str)
        and len(digest) == 64
        and all(character in "0123456789abcdef" for character in digest)
    )


def validate_case3(catalog: dict[str, Any]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    require(
        catalog.get("schema_version") == "aero-reference-catalog/0.2.0",
        "unsupported DPW8 catalog schema",
    )
    case = find_case(catalog)
    sources = find_sources(catalog)
    require(case.get("required") is False, "Test Case 3 must remain optional")
    require(
        case.get("physics_track") == "unsteady_dynamic_aeroelastic_fsi",
        "Test Case 3 physics track changed",
    )
    conditions = case.get("conditions", {})
    require(conditions.get("mach") == 0.85, "Test Case 3 Mach changed")
    require(conditions.get("reynolds_chord") == 2_270_000, "Test Case 3 Reynolds number changed")
    require(conditions.get("total_pressure_kpa") == 12.0, "Test Case 3 total pressure changed")
    require(
        conditions.get("angles_of_attack_deg") == [3.05, 3.61, 4.7],
        "Test Case 3 alpha schedule changed",
    )

    channels = case.get("measurement_channels")
    require(isinstance(channels, list) and len(channels) == 6, "six measurement channel families required")
    by_family = {item.get("channel_family"): item for item in channels}
    expected_families = {
        "integrated_balance",
        "buffet_intensity",
        "steady_pressure",
        "unsteady_point_pressure",
        "unsteady_surface_pressure",
        "structural_deformation",
    }
    require(set(by_family) == expected_families, "measurement channel families changed")

    for channel in channels:
        source_id = channel.get("source_id")
        require(source_id in sources, f"dangling source reference {source_id}")
        source = sources[source_id]
        require(
            source.get("exact_bytes_acquired") is False and source.get("sha256") is None,
            f"{source_id}: exact source bytes were not admitted; do not invent a digest",
        )
        require(
            source.get("acquisition_state")
            in {
                "page_inspected",
                "listing_only",
                "linked_file_not_acquired",
                "linked_files_not_acquired",
                "text_preview_inspected_exact_bytes_not_acquired",
            },
            f"{source_id}: source admission state was unexpectedly promoted",
        )
        require(
            not is_source_admitted(source),
            f"{source_id}: unacquired source unexpectedly counted as admitted",
        )

    kulite_limits = " ".join(by_family["unsteady_point_pressure"].get("limitations", []))
    require("9 kHz" in kulite_limits, "Kulite 9 kHz wind-tunnel-noise warning missing")
    upsp_limits = " ".join(by_family["unsteady_surface_pressure"].get("limitations", []))
    require("low-pass" in upsp_limits.lower(), "uPSP low-pass limitation missing")
    require("masks" in upsp_limits.lower(), "uPSP reliability-mask limitation missing")

    balance_limits = " ".join(by_family["integrated_balance"].get("limitations", []))
    require("N/A" in balance_limits and "null" in balance_limits, "PSP drag null semantics missing")

    cb = by_family["buffet_intensity"]
    require(cb.get("quantity") == "C_B", "buffet channel must remain C_B")
    require(
        "not a full structural mode" in " ".join(cb.get("limitations", [])),
        "C_B indicator-only limitation missing",
    )

    structural = by_family["structural_deformation"]
    require(
        structural.get("evidence_status") == "PENDING_NOT_PUBLISHED",
        "official structural deformation must remain pending",
    )
    deformation = case.get("deformation_evidence", {})
    require(deformation.get("official_measurements_status") == "pending", "deformation pending status changed")
    require(deformation.get("preliminary_tip_deflection_mm") == [10, 12], "preliminary tip estimate changed")
    require(deformation.get("preliminary_evidence_class") == "qualitative_only_not_validation",
            "preliminary deflection cannot become validation data")
    require(deformation.get("allow_as_quantitative_acceptance_threshold") is False,
            "preliminary deflection cannot be a quantitative acceptance threshold")

    spectral = case.get("spectral_processing", {})
    require(
        spectral.get("status") == "must_be_admitted_from_case3_artifact_headers_or_controlling_procedure",
        "spectral processing must remain source-admitted",
    )
    for field in (
        "time_step_s",
        "sampling_rate_hz",
        "window",
        "segment_length",
        "overlap",
        "psd_normalization",
    ):
        require(spectral.get(field) is None, f"{field} cannot be invented for Test Case 3")

    measurement_requirements = " ".join(case.get("measurement_operator_requirements", []))
    for required_phrase in (
        "calibration",
        "sampling",
        "time synchronization",
        "9 kHz",
        "raw measurement values and masks",
        "Test Case 2 Welch",
    ):
        require(required_phrase in measurement_requirements, f"measurement operator policy missing {required_phrase}")

    return case, sources


def source_state_report(case: dict[str, Any], sources: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    reports = []
    for channel in case["measurement_channels"]:
        source = sources[channel["source_id"]]
        admitted = is_source_admitted(source)
        reports.append(
            {
                "channel_family": channel["channel_family"],
                "instrument_or_method": channel.get("instrument_or_method"),
                "source_id": channel["source_id"],
                "acquisition_state": source.get("acquisition_state", "UNKNOWN"),
                "exact_bytes_acquired": source.get("exact_bytes_acquired") is True,
                "sha256_present": isinstance(source.get("sha256"), str),
                "source_admitted": admitted,
                "quantitative_comparison": (
                    "REQUIRES_OPERATOR_VALIDATION"
                    if admitted
                    else "ABSTAIN_SOURCE_NOT_ADMITTED"
                ),
                "source_limitations": channel.get("limitations", []),
            }
        )
    return reports


def expected_report(case: dict[str, Any], sources: dict[str, dict[str, Any]]) -> dict[str, Any]:
    channels = source_state_report(case, sources)
    spectral = case["spectral_processing"]
    missing_spectral = [
        field
        for field in (
            "time_step_s",
            "sampling_rate_hz",
            "window",
            "segment_length",
            "overlap",
            "psd_normalization",
        )
        if spectral.get(field) is None
    ]
    deformation = case["deformation_evidence"]
    return {
        "schema_version": "aero-dynamic-observation-audit/0.1.0",
        "case_id": "DPW8-CRM-DYNAMIC-FSI-3",
        "case_status": "OPTIONAL",
        "source_admission": "NOT_ADMITTED",
        "channel_count": len(channels),
        "channel_reports": channels,
        "spectral_processing": {
            "status": "ABSTAIN_MISSING_SPECTRAL_METADATA"
            if missing_spectral
            else "METADATA_PRESENT_REQUIRES_REVIEW",
            "missing_fields": missing_spectral,
        },
        "comparison_decisions": {
            "psd_comparison": "ABSTAIN_MISSING_SAMPLING_AND_ESTIMATOR_METADATA",
            "cross_channel_coherence": "ABSTAIN_NO_ADMITTED_COMMON_TIMEBASE_OR_SYNCHRONIZATION",
            "kulite_vs_upsp_raw_rms": "REJECT_DIRECT_EQUALITY_DIFFERENT_OBSERVATION_OPERATORS",
            "buffet_coefficient_c_b": "INDICATOR_ONLY_NOT_MODAL_OR_DURABILITY_EVIDENCE",
            "psp_simultaneous_drag": "NULL_NOT_IMPUTED",
            "quantitative_deformation": "NOT_ASSESSED_OFFICIAL_DEFORMATION_PENDING"
            if deformation["official_measurements_status"] == "pending"
            else "REQUIRES_SOURCE_REVIEW",
            "structural_modal_validation": "NOT_ASSESSED_NO_ADMITTED_STRUCTURAL_MODE_TIME_HISTORY",
        },
        "known_confounders": [
            {
                "instrument": "Kulite",
                "frequency_hz": 9000.0,
                "classification": "WIND_TUNNEL_NOISE",
                "promotion_to_structural_mode": False,
            }
        ],
        "preliminary_tip_deflection": {
            "value_mm": deformation.get("preliminary_tip_deflection_mm"),
            "angle_of_attack_deg": deformation.get("preliminary_angle_of_attack_deg"),
            "evidence_class": deformation.get("preliminary_evidence_class"),
            "quantitative_acceptance_threshold": deformation.get("allow_as_quantitative_acceptance_threshold"),
        },
        "claim_ceiling": "measurement_operator_contract_and_source_admission_only",
    }


def assess_pair(
    left: dict[str, Any],
    right: dict[str, Any],
    kind: str,
    maximum_sync_uncertainty_s: float,
) -> tuple[str, str]:
    if not left.get("source_admitted") or not right.get("source_admitted"):
        return "ABSTAIN", "source bytes, terms and applicability are not admitted"
    if not left.get("calibration_admitted") or not right.get("calibration_admitted"):
        return "ABSTAIN", "instrument calibration/uncertainty is not admitted"
    for key, description in (
        ("quantity", "physical quantities differ"),
        ("unit", "units differ and no explicit canonical conversion was supplied"),
        ("coordinate_frame", "coordinate/reference frames differ"),
        ("spatial_operator_id", "spatial sampling/mapping operators differ"),
        ("temporal_operator_id", "temporal instrument responses differ"),
        ("bandwidth_hz", "declared comparison bandwidths differ"),
    ):
        if left.get(key) != right.get(key):
            return "REJECT", description
    if (left.get("mask_required") and not left.get("mask_applied")) or (
        right.get("mask_required") and not right.get("mask_applied")
    ):
        return "REJECT", "a required reliability mask has not been applied"
    band = left.get("bandwidth_hz")
    require(
        isinstance(band, list)
        and len(band) == 2
        and all(isinstance(x, (int, float)) and math.isfinite(x) for x in band)
        and band[0] >= 0
        and band[1] > band[0],
        "synthetic fixture bandwidth is invalid",
    )

    if kind in {"pointwise", "rms"}:
        return "ALLOW", "identical admitted observation operators"
    if kind in {"psd", "coherence"}:
        for channel in (left, right):
            rate = channel.get("sample_rate_hz")
            if not isinstance(rate, (int, float)) or not math.isfinite(rate) or rate <= 0:
                return "ABSTAIN", "sample rate is missing or invalid"
            psd = channel.get("psd", {})
            if not all(
                psd.get(key) is not None
                for key in (
                    "window",
                    "segment_length_samples",
                    "overlap_samples",
                    "detrend",
                    "estimator",
                    "normalization",
                    "output_unit",
                )
            ):
                return "ABSTAIN", "PSD estimator metadata are incomplete"
        if left["sample_rate_hz"] != right["sample_rate_hz"]:
            return "REJECT", "sample rates differ"
        if left["psd"] != right["psd"]:
            return "REJECT", "PSD estimator configurations differ"
        if kind == "psd":
            return "ALLOW", "PSD processing metadata match"
    if kind == "coherence":
        if not left.get("timebase_id") or not right.get("timebase_id"):
            return "ABSTAIN", "a common acquisition timebase has not been identified"
        if left["timebase_id"] != right["timebase_id"]:
            return "REJECT", "channels do not share the same admitted timebase"
        for channel in (left, right):
            sync = channel.get("synchronization_uncertainty_s")
            if not isinstance(sync, (int, float)) or not math.isfinite(sync) or sync < 0:
                return "ABSTAIN", "synchronization uncertainty is unknown or invalid"
            if sync > maximum_sync_uncertainty_s:
                return "ABSTAIN", "synchronization uncertainty exceeds the declared budget"
        return "ALLOW", "shared clock and bounded synchronization uncertainty"
    if kind == "modal_validation":
        if not left.get("structural_mode_evidence_admitted") or not right.get("structural_mode_evidence_admitted"):
            return "NOT_ASSESSED", "admitted structural mode/deformation evidence is absent"
        if not left.get("timebase_id") or left.get("timebase_id") != right.get("timebase_id"):
            return "ABSTAIN", "modal comparison requires a shared timebase"
        return "ALLOW", "admitted structural mode evidence and common timebase"
    raise AuditError(f"unsupported comparison kind {kind}")


def synthetic_channel(
    channel_id: str,
    *,
    quantity: str = "pressure",
    unit: str = "Pa",
    spatial_operator_id: str = "sensor-grid-A",
    temporal_operator_id: str = "bandpass-A",
) -> dict[str, Any]:
    return {
        "channel_id": channel_id,
        "quantity": quantity,
        "unit": unit,
        "coordinate_frame": "CRM-jig-metric-v1",
        "spatial_operator_id": spatial_operator_id,
        "temporal_operator_id": temporal_operator_id,
        "source_admitted": True,
        "calibration_admitted": True,
        "mask_required": False,
        "mask_applied": True,
        "timebase_id": "synthetic-clock-A",
        "sample_rate_hz": 20_000.0,
        "synchronization_uncertainty_s": 1.0e-6,
        "bandwidth_hz": [0.0, 8_000.0],
        "psd": {
            "window": "hann",
            "segment_length_samples": 2048,
            "overlap_samples": 1024,
            "detrend": "constant",
            "estimator": "welch-mean",
            "normalization": "density",
            "output_unit": "Pa^2/Hz",
        },
        "structural_mode_evidence_admitted": False,
    }


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError, ValueError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_catalog_controls(catalog: dict[str, Any]) -> None:
    bad = copy.deepcopy(catalog)
    find_case(bad)["required"] = True
    expect_rejected("optional_case_promoted_to_required", lambda: validate_case3(bad))

    bad = copy.deepcopy(catalog)
    case = find_case(bad)
    upsp = next(item for item in case["measurement_channels"] if item["channel_family"] == "unsteady_surface_pressure")
    upsp["limitations"] = ["direct unfiltered pressure field"]
    expect_rejected("uPSP_filter_or_masks_removed", lambda: validate_case3(bad))

    bad = copy.deepcopy(catalog)
    case = find_case(bad)
    kulite = next(item for item in case["measurement_channels"] if item["channel_family"] == "unsteady_point_pressure")
    kulite["limitations"] = ["9 kHz is the primary structural mode"]
    expect_rejected("wind_tunnel_noise_promoted_to_mode", lambda: validate_case3(bad))

    bad = copy.deepcopy(catalog)
    case = find_case(bad)
    case["spectral_processing"]["sampling_rate_hz"] = 50_000.0
    expect_rejected("unadmitted_sampling_rate_invented", lambda: validate_case3(bad))

    bad = copy.deepcopy(catalog)
    case = find_case(bad)
    case["deformation_evidence"]["preliminary_evidence_class"] = "quantitative_validation"
    expect_rejected("preliminary_tip_deflection_promoted", lambda: validate_case3(bad))

    bad = copy.deepcopy(catalog)
    case = find_case(bad)
    case["measurement_channels"] = [
        item for item in case["measurement_channels"] if item["channel_family"] != "unsteady_surface_pressure"
    ]
    expect_rejected("uPSP_observation_family_removed", lambda: validate_case3(bad))


def synthetic_comparison_controls() -> None:
    left = synthetic_channel("synthetic-measured")
    right = synthetic_channel("synthetic-solver-through-same-operator")
    require(assess_pair(left, right, "rms", 1.0e-5)[0] == "ALLOW", "matched synthetic RMS must pass")

    lowpass = synthetic_channel("synthetic-upsp", temporal_operator_id="camera-lowpass")
    require(assess_pair(left, lowpass, "rms", 1.0e-5)[0] == "REJECT", "different temporal responses must reject")

    unsynced = copy.deepcopy(right)
    unsynced["timebase_id"] = None
    require(assess_pair(left, unsynced, "coherence", 1.0e-5)[0] == "ABSTAIN", "unknown timebase must abstain")

    no_rate = copy.deepcopy(right)
    no_rate["sample_rate_hz"] = None
    require(assess_pair(left, no_rate, "psd", 1.0e-5)[0] == "ABSTAIN", "unknown sample rate must abstain")

    masked = copy.deepcopy(right)
    masked["mask_required"] = True
    masked["mask_applied"] = False
    require(assess_pair(left, masked, "pointwise", 1.0e-5)[0] == "REJECT", "unapplied mask must reject")

    no_source = copy.deepcopy(right)
    no_source["source_admitted"] = False
    require(assess_pair(left, no_source, "rms", 1.0e-5)[0] == "ABSTAIN", "unadmitted source must abstain")

    no_modes = copy.deepcopy(right)
    require(assess_pair(left, no_modes, "modal_validation", 1.0e-5)[0] == "NOT_ASSESSED",
            "missing structural mode evidence must not validate modal response")


def verify_rust_report(actual: dict[str, Any], expected: dict[str, Any]) -> None:
    require(actual == expected, "Rust audit output differs from independent Python reconstruction")
    require(actual["source_admission"] == "NOT_ADMITTED", "unacquired JAXA sources were promoted")
    require(
        actual["comparison_decisions"]["structural_modal_validation"]
        == "NOT_ASSESSED_NO_ADMITTED_STRUCTURAL_MODE_TIME_HISTORY",
        "structural modal claim was promoted",
    )


def main() -> int:
    try:
        catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
        case, sources = validate_case3(catalog)
        expected = expected_report(case, sources)
        hostile_catalog_controls(catalog)
        synthetic_comparison_controls()

        import argparse

        parser = argparse.ArgumentParser()
        parser.add_argument("--rust-output", type=Path)
        args = parser.parse_args()
        if args.rust_output is not None:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_rust_report(actual, expected)
            boundary = "rust_python_boundary=pass"
        else:
            boundary = "rust_python_boundary=not_run"

        print(
            "dynamic_observation_operator=pass "
            f"case_channels={len(case['measurement_channels'])} "
            f"source_admitted=0 "
            f"psd_metadata_missing=6 "
            f"official_deformation=pending "
            f"{boundary}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"dynamic_observation_operator_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
