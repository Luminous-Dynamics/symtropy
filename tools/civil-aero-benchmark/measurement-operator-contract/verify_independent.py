#!/usr/bin/env python3
"""Independent standard-library oracle for AER-01 measurement operators.

This implementation validates synthetic observation-operator contracts and
recomputes pair compatibility. It does not use the Rust implementation, read
or transform real measurements, calibrate sensors, or perform spectral work.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "measurement-operator-fixture-v0.1.json"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PRESENT = {"PRESENT"}
MISSING = {"MASKED", "UNAVAILABLE", "NOT_COLLECTED", "UNKNOWN"}


class AuditError(ValueError):
    """Raised when a measurement contract violates its frozen rules."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def nonempty(value: Any, label: str) -> None:
    require(isinstance(value, str) and value.strip(), f"{label}: must be non-empty")


def digest(value: Any, label: str) -> None:
    require(
        isinstance(value, str) and SHA256_RE.fullmatch(value) is not None,
        f"{label}: expected lowercase SHA-256",
    )


def positive(value: Any, label: str) -> None:
    require(
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
        and float(value) > 0.0,
        f"{label}: must be finite and positive",
    )


def validate_identity(identity: dict[str, Any], label: str) -> None:
    for key in ("configuration_sha256", "geometry_sha256", "loading_state_sha256", "environment_sha256"):
        digest(identity.get(key), f"{label}.{key}")
    nonempty(identity.get("mission_phase_id"), f"{label}.mission_phase_id")
    nonempty(identity.get("test_article_id"), f"{label}.test_article_id")


def validate_channel(channel: dict[str, Any]) -> None:
    cid = channel.get("id")
    for key in (
        "id",
        "instrument_family",
        "instrument_id",
        "quantity",
        "units",
        "sign_convention",
        "spatial_support",
        "spatial_location_id",
        "coordinate_frame",
        "measurement_run_id",
    ):
        nonempty(channel.get(key), f"{cid}.{key}")

    for key in ("source_artifact_sha256", "raw_data_sha256", "derived_processing_sha256"):
        digest(channel.get(key), f"{cid}.{key}")
    validate_identity(channel.get("identity", {}), cid)

    registration = channel.get("registration_sha256")
    if registration is not None:
        digest(registration, f"{cid}.registration_sha256")

    calibration = channel.get("calibration", {})
    status = calibration.get("status")
    require(status in {"VALID", "EXPIRED", "UNKNOWN"}, f"{cid}: unsupported calibration status {status!r}")
    if status == "VALID":
        nonempty(calibration.get("calibration_id"), f"{cid}.calibration_id")
        digest(calibration.get("calibration_artifact_sha256"), f"{cid}.calibration_artifact_sha256")
        digest(calibration.get("validity_envelope_sha256"), f"{cid}.validity_envelope_sha256")
        require(calibration.get("valid_at_test_state") is True, f"{cid}: calibration outside test-state envelope")
    else:
        require(calibration.get("valid_at_test_state") is False,
                f"{cid}: expired/unknown calibration cannot be valid at test state")

    sampling = channel.get("sampling", {})
    nonempty(sampling.get("statistic"), f"{cid}.statistic")
    nonempty(sampling.get("time_window_id"), f"{cid}.time_window_id")
    sync = sampling.get("synchronization_status")
    require(sync in {"SYNCHRONIZED", "UNSYNCHRONIZED", "UNKNOWN"},
            f"{cid}: unsupported synchronization status {sync!r}")
    if sync == "SYNCHRONIZED":
        nonempty(sampling.get("time_base_id"), f"{cid}.time_base_id")
        digest(sampling.get("synchronization_receipt_sha256"), f"{cid}.synchronization_receipt_sha256")
    else:
        require(sampling.get("synchronization_receipt_sha256") is None,
                f"{cid}: non-synchronized channel cannot carry a synchronization receipt")

    rate = sampling.get("sample_rate_hz")
    if rate is not None:
        positive(rate, f"{cid}.sample_rate_hz")
    for key in ("frequency_grid_sha256", "spectral_estimator_sha256"):
        if sampling.get(key) is not None:
            digest(sampling[key], f"{cid}.{key}")
    if sampling.get("statistic") in {"PSD", "CROSS_SPECTRUM", "COHERENCE"}:
        positive(rate, f"{cid}.spectral sample rate")
        digest(sampling.get("frequency_grid_sha256"), f"{cid}.frequency_grid_sha256")
        digest(sampling.get("spectral_estimator_sha256"), f"{cid}.spectral_estimator_sha256")

    operator = channel.get("filter", {})
    if operator.get("transfer_function_sha256") is not None:
        digest(operator["transfer_function_sha256"], f"{cid}.transfer_function_sha256")
    if operator.get("bandwidth_hz") is not None:
        positive(operator["bandwidth_hz"], f"{cid}.bandwidth_hz")
    if operator.get("integration_time_s") is not None:
        positive(operator["integration_time_s"], f"{cid}.integration_time_s")
    if operator.get("exposure_time_s") is not None:
        positive(operator["exposure_time_s"], f"{cid}.exposure_time_s")
    if operator.get("mask_sha256") is not None:
        digest(operator["mask_sha256"], f"{cid}.mask_sha256")
    if operator.get("low_pass") is True:
        require(operator.get("bandwidth_hz") is not None or operator.get("exposure_time_s") is not None,
                f"{cid}: low-pass operator requires bandwidth or exposure metadata")

    observation = channel.get("observation", {})
    obs_status = observation.get("status")
    if obs_status in PRESENT:
        value = observation.get("value")
        require(
            isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value)),
            f"{cid}: PRESENT observation requires finite numeric value",
        )
        require(observation.get("missing_reason") is None,
                f"{cid}: PRESENT observation cannot carry missingness reason")
    elif obs_status in MISSING:
        require(observation.get("value") is None,
                f"{cid}: {obs_status} observation must remain non-numeric (no zero-fill)")
        nonempty(observation.get("missing_reason"), f"{cid}.missing_reason")
    else:
        raise AuditError(f"{cid}: unsupported observation status {obs_status!r}")

    limitations = channel.get("interpretation_limitations")
    require(isinstance(limitations, list), f"{cid}: interpretation_limitations must be a list")
    for item in limitations:
        nonempty(item, f"{cid}.interpretation_limitation")


def validate_dataset(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-measurement-operator/0.1.0", "unsupported schema version")
    nonempty(data.get("dataset_id"), "dataset_id")
    require(data.get("evidence_class") == "synthetic_contract_fixture",
            "v0.1 only admits the synthetic contract fixture")
    require(data.get("claim_ceiling") == "measurement_operator_compatibility_only",
            "claim ceiling missing or weakened")
    channels = data.get("channels")
    comparisons = data.get("comparisons")
    require(isinstance(channels, list) and channels, "channels must be non-empty")
    require(isinstance(comparisons, list), "comparisons must be a list")
    ids = [item.get("id") for item in channels]
    require(all(isinstance(x, str) and x for x in ids), "every channel requires an ID")
    require(len(ids) == len(set(ids)), "duplicate channel ID")
    for channel in channels:
        validate_channel(channel)
    channel_ids = set(ids)
    comp_ids = [item.get("id") for item in comparisons]
    require(all(isinstance(x, str) and x for x in comp_ids), "every comparison requires an ID")
    require(len(comp_ids) == len(set(comp_ids)), "duplicate comparison ID")
    for item in comparisons:
        require(item.get("left_channel_id") in channel_ids, f"{item.get('id')}: unknown left channel")
        require(item.get("right_channel_id") in channel_ids, f"{item.get('id')}: unknown right channel")
        require(item["left_channel_id"] != item["right_channel_id"], f"{item.get('id')}: self-comparison")
        require(item.get("mode") in {"DIRECT_SCALAR", "COHERENCE_OR_PHASE"},
                f"{item.get('id')}: unsupported comparison mode")


def make_result(data: dict[str, Any], source_bytes: bytes) -> dict[str, Any]:
    validate_dataset(data)
    channels = {item["id"]: item for item in data["channels"]}
    channel_results = []
    for channel in data["channels"]:
        present = channel["observation"]["status"] == "PRESENT"
        channel_results.append({
            "id": channel["id"],
            "observation_status": channel["observation"]["status"],
            "calibration_status": channel["calibration"]["status"],
            "claim_status": "OBSERVATION_PRESENT" if present else "OBSERVATION_WITHHELD",
            "reason_codes": ["RAW_OR_DECLARED_DERIVED_VALUE_RETAINED"] if present
                else [f"OBSERVATION_{channel['observation']['status']}"],
        })

    results = []
    for request in data["comparisons"]:
        left = channels[request["left_channel_id"]]
        right = channels[request["right_channel_id"]]
        status = None
        reasons: list[str] = []

        left_obs = left["observation"]["status"]
        right_obs = right["observation"]["status"]
        if left_obs != "PRESENT" or right_obs != "PRESENT":
            if left_obs != "PRESENT":
                reasons.append(f"LEFT_{left_obs}")
            if right_obs != "PRESENT":
                reasons.append(f"RIGHT_{right_obs}")
            status = "ABSTAIN"
        elif left["calibration"]["status"] != "VALID" or right["calibration"]["status"] != "VALID":
            if left["calibration"]["status"] != "VALID":
                reasons.append(f"LEFT_CALIBRATION_{left['calibration']['status']}")
            if right["calibration"]["status"] != "VALID":
                reasons.append(f"RIGHT_CALIBRATION_{right['calibration']['status']}")
            status = "ABSTAIN"
        else:
            if left["identity"] != right["identity"]:
                reasons.append("STATE_IDENTITY_MISMATCH")
            if left["quantity"] != right["quantity"]:
                reasons.append("QUANTITY_MISMATCH")
            if left["units"] != right["units"]:
                reasons.append("UNIT_MISMATCH")
            if left["sign_convention"] != right["sign_convention"]:
                reasons.append("SIGN_CONVENTION_MISMATCH")
            if left["spatial_support"] != right["spatial_support"]:
                reasons.append("SPATIAL_SUPPORT_MISMATCH")
            if left["spatial_location_id"] != right["spatial_location_id"]:
                reasons.append("SPATIAL_LOCATION_MISMATCH")
            if left["coordinate_frame"] != right["coordinate_frame"]:
                reasons.append("COORDINATE_FRAME_MISMATCH")
            if left["registration_sha256"] != right["registration_sha256"]:
                reasons.append("REGISTRATION_MISMATCH")
            if left["sampling"]["statistic"] != right["sampling"]["statistic"]:
                reasons.append("STATISTIC_MISMATCH")
            if left["sampling"]["time_window_id"] != right["sampling"]["time_window_id"]:
                reasons.append("TIME_WINDOW_MISMATCH")
            if left["measurement_run_id"] != right["measurement_run_id"]:
                reasons.append("MEASUREMENT_RUN_MISMATCH")
            filt_left, filt_right = left["filter"], right["filter"]
            for key in ("transfer_function_sha256", "bandwidth_hz", "low_pass", "integration_time_s", "exposure_time_s"):
                if filt_left[key] != filt_right[key]:
                    reasons.append("MEASUREMENT_FILTER_MISMATCH")
                    break
            if filt_left["mask_sha256"] != filt_right["mask_sha256"]:
                reasons.append("SPATIAL_MASK_MISMATCH")
            if left["sampling"]["statistic"] in {"PSD", "CROSS_SPECTRUM", "COHERENCE"}:
                for key in ("sample_rate_hz", "frequency_grid_sha256", "spectral_estimator_sha256"):
                    if left["sampling"][key] != right["sampling"][key]:
                        reasons.append("SPECTRAL_PROCESSING_MISMATCH")
                        break

            if reasons:
                status = "INCOMPATIBLE"
            elif request["mode"] == "COHERENCE_OR_PHASE":
                l_s, r_s = left["sampling"], right["sampling"]
                if (
                    l_s["synchronization_status"] != "SYNCHRONIZED"
                    or r_s["synchronization_status"] != "SYNCHRONIZED"
                    or l_s["time_base_id"] != r_s["time_base_id"]
                    or l_s["synchronization_receipt_sha256"] != r_s["synchronization_receipt_sha256"]
                    or l_s["sample_rate_hz"] != r_s["sample_rate_hz"]
                ):
                    status = "UNSYNCHRONIZED"
                    reasons = ["COMMON_TIMEBASE_OR_SYNC_RECEIPT_NOT_ESTABLISHED"]
                else:
                    status = "DIRECT_COMPARISON_ADMITTED"
                    reasons = ["OPERATOR_IDENTITIES_MATCH_DECLARED_V0_1_RULES"]
            else:
                status = "DIRECT_COMPARISON_ADMITTED"
                reasons = ["OPERATOR_IDENTITIES_MATCH_DECLARED_V0_1_RULES"]

        results.append({
            "id": request["id"],
            "left_channel_id": left["id"],
            "right_channel_id": right["id"],
            "status": status,
            "reason_codes": reasons,
        })

    return {
        "schema_version": "aero-measurement-operator-result/0.1.0",
        "dataset_id": data["dataset_id"],
        "source_fixture_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "channel_results": channel_results,
        "comparisons": results,
        "admitted_comparisons": sum(x["status"] == "DIRECT_COMPARISON_ADMITTED" for x in results),
        "abstained_comparisons": sum(x["status"] == "ABSTAIN" for x in results),
        "incompatible_comparisons": sum(x["status"] == "INCOMPATIBLE" for x in results),
        "unsynchronized_comparisons": sum(x["status"] == "UNSYNCHRONIZED" for x in results),
        "claim_ceiling": "measurement_operator_compatibility_only",
    }


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_controls(fixture: dict[str, Any]) -> None:
    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "balance_psp_simultaneous_cd")
    channel["observation"]["value"] = 0.0
    expect_rejected("unavailable_balance_drag_not_zero", lambda: validate_dataset(bad))

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "kulite_point_a")
    channel["observation"]["value"] = None
    expect_rejected("present_channel_requires_numeric_value", lambda: validate_dataset(bad))

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "upsp_surface_rms")
    channel["filter"]["bandwidth_hz"] = None
    channel["filter"]["exposure_time_s"] = None
    expect_rejected("low_pass_requires_operator_metadata", lambda: validate_dataset(bad))

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "kulite_point_a")
    channel["sampling"]["sample_rate_hz"] = 0
    expect_rejected("zero_sample_rate_rejected", lambda: validate_dataset(bad))

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "healthy_point_mean_b")
    channel["registration_sha256"] = None
    result = make_result(bad, json.dumps(bad, sort_keys=True).encode("utf-8"))
    comp = next(x for x in result["comparisons"] if x["id"] == "healthy_same_point_mean")
    require(comp["status"] == "INCOMPATIBLE", "stale/missing registration must not be admitted")

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "healthy_point_mean_b")
    channel["calibration"]["status"] = "UNKNOWN"
    channel["calibration"]["valid_at_test_state"] = False
    result = make_result(bad, json.dumps(bad, sort_keys=True).encode("utf-8"))
    comp = next(x for x in result["comparisons"] if x["id"] == "healthy_same_point_mean")
    require(comp["status"] == "ABSTAIN", "unknown calibration must abstain")

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "healthy_point_mean_b")
    channel["sampling"]["time_base_id"] = "unrelated-clock"
    result = make_result(bad, json.dumps(bad, sort_keys=True).encode("utf-8"))
    comp = next(x for x in result["comparisons"] if x["id"] == "phase_pair_unsynchronized")
    require(comp["status"] == "UNSYNCHRONIZED", "coherence without common clock must abstain")

    bad = copy.deepcopy(fixture)
    channel = next(x for x in bad["channels"] if x["id"] == "strain_derived_cb")
    channel["quantity"] = "wing_tip_vertical_deflection"
    result = make_result(bad, json.dumps(bad, sort_keys=True).encode("utf-8"))
    comp = next(x for x in result["comparisons"] if x["id"] == "derived_buffet_index_vs_deflection")
    require(comp["status"] == "INCOMPATIBLE", "derived C_B quantity cannot be relabelled")

    bad = copy.deepcopy(fixture)
    bad["claim_ceiling"] = "physical_validation"
    expect_rejected("claim_ceiling_cannot_be_promoted", lambda: validate_dataset(bad))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        source = FIXTURE.read_bytes()
        fixture = json.loads(source)
        expected = make_result(fixture, source)
        require(len(fixture["channels"]) == 12, "frozen fixture channel count changed")
        require(len(fixture["comparisons"]) == 9, "frozen fixture comparison count changed")
        require(expected["admitted_comparisons"] == 1, "frozen direct-comparison count changed")
        require(expected["abstained_comparisons"] == 4, "frozen abstention count changed")
        require(expected["incompatible_comparisons"] == 3, "frozen incompatibility count changed")
        require(expected["unsynchronized_comparisons"] == 1, "frozen sync count changed")

        if args.rust_output is not None:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            require(actual == expected, "Rust/Python measurement-operator compatibility mismatch")

        hostile_controls(fixture)
        mode = "rust_python_boundary=pass" if args.rust_output is not None else "rust_python_boundary=not_run"
        print(
            "measurement_operator_oracle=pass "
            f"channels={len(fixture['channels'])} comparisons={len(fixture['comparisons'])} "
            f"admitted={expected['admitted_comparisons']} abstain={expected['abstained_comparisons']} "
            f"incompatible={expected['incompatible_comparisons']} "
            f"unsynchronized={expected['unsynchronized_comparisons']} {mode}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"measurement_operator_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
