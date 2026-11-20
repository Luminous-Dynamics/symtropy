#!/usr/bin/env python3
"""Validate the FMPy 0.3.32 dispatch transcript without third-party dependencies."""
from __future__ import annotations

import json
import hashlib
import math
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TRANSCRIPT = ROOT / "build" / "stage" / "fmi3-api-transcript.json"
SCHEMA = ROOT / "tests" / "fmi3-dispatch-transcript.schema.json"
HEX64 = re.compile(r"^[0-9a-f]{16}$")
HEX256 = re.compile(r"^[0-9a-f]{64}$")

def fail(message: str) -> None:
    raise AssertionError(message)

def load_json_no_duplicate_keys(path: Path):
    def no_duplicates(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                fail(f"{path}: duplicate JSON object key {key!r}")
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=no_duplicates)

def decode_f64(bits: str) -> float:
    if not isinstance(bits, str) or not HEX64.fullmatch(bits):
        fail(f"invalid binary64 hex encoding: {bits!r}")
    return struct.unpack(">d", bytes.fromhex(bits))[0]

def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)

def validate_schema_shape(transcript, schema):
    require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema",
            "transcript schema is not JSON Schema Draft 2020-12")
    require(schema.get("title") == "FMPy FMI 3.0.2 importer dispatch transcript v0.1",
            "unexpected transcript schema identity")
    require(transcript.get("schema_version") == "fmi3-dispatch-transcript-0.1.0",
            "unsupported transcript schema version")
    require(transcript.get("observation_boundary") ==
            "fmpy-0.3.32 _call wrapper immediately around ctypes function-pointer invocation",
            "observation boundary does not match the pinned adapter")
    require(transcript.get("importer") == {"name": "FMPy", "version": "0.3.32"},
            "transcript importer/version mismatch")
    for key in ("claim_ceiling", "runtime", "fmus", "summary", "events"):
        require(key in transcript, f"missing transcript root key: {key}")
    runtime = transcript["runtime"]
    require(set(runtime) == {"python", "platform", "dependencies"},
            "runtime field set mismatch")
    required_dependencies = {
        "FMPy", "attrs", "cmake", "jinja2", "lark",
        "lxml", "msgpack", "nbformat", "numpy"
    }
    require(set(runtime["dependencies"]) == required_dependencies,
            "runtime dependency version set mismatch")
    require(all(isinstance(value, str) and value
                for value in runtime["dependencies"].values()),
            "runtime dependency versions must be non-empty strings")

    require(set(transcript) == {
        "schema_version", "observation_boundary", "claim_ceiling", "importer",
        "runtime", "fmus", "summary", "events"
    }, "unexpected root-level transcript fields")

    require(set(transcript["fmus"]) == {"geothermal-source", "heat-exchanger-buffer"},
            "FMU identity set mismatch")
    for federate, artifact in transcript["fmus"].items():
        require(set(artifact) == {
            "package_sha256", "model_description_sha256", "binary_sha256",
            "model_name", "model_identifier"
        }, f"{federate}: FMU digest fields mismatch")
        for field in ("package_sha256", "model_description_sha256", "binary_sha256"):
            require(bool(HEX256.fullmatch(artifact[field])),
                    f"{federate}: invalid {field}")

    events = transcript["events"]
    require(isinstance(events, list) and len(events) > 0, "transcript events must be non-empty")
    allowed_functions = {
        "fmi3InstantiateCoSimulation", "fmi3EnterInitializationMode",
        "fmi3ExitInitializationMode", "fmi3GetFloat64", "fmi3SetFloat64",
        "fmi3DoStep", "fmi3Terminate", "fmi3FreeInstance"
    }
    required_event_keys = {
        "sequence", "federate_id", "function", "argument_count", "argument_types",
        "status_code", "status", "exception_type", "exception_message", "payload"
    }
    for index, event in enumerate(events):
        require(set(required_event_keys).issubset(event),
                f"event {index}: missing required fields")
        require(set(event).issubset(required_event_keys | {"result_type"}),
                f"event {index}: unexpected fields")
        require(event["sequence"] == index, f"event sequence discontinuity at {index}")
        require(event["federate_id"] in ("geothermal-source", "heat-exchanger-buffer"),
                f"event {index}: unknown federate")
        require(event["function"] in allowed_functions, f"event {index}: unrecognized FMI function")
        require(event["argument_count"] == len(event["argument_types"]),
                f"event {index}: argument type/count mismatch")
        require(event["status"] in {
            "ok", "warning", "discard", "error", "fatal", "pending",
            "returned", "void_return", "exception", "unknown"
        }, f"event {index}: unknown status label")
        require(isinstance(event["payload"], dict), f"event {index}: payload must be an object")

def validate_semantics(transcript):
    events = transcript["events"]
    by_function = {}
    for event in events:
        by_function.setdefault(event["function"], []).append(event)

    require(len(by_function.get("fmi3InstantiateCoSimulation", [])) == 2,
            "expected two Co-Simulation instantiation calls")
    for event in by_function["fmi3InstantiateCoSimulation"]:
        require(event["status"] == "returned" and event.get("result_type") == "instance_pointer",
                "FMU instance pointer was not distinguished from FMI status")
        require(event["payload"].get("instance_created") is True,
                "FMU instantiation did not produce a non-null instance pointer")

    for symbol, expected in (
        ("fmi3EnterInitializationMode", 2),
        ("fmi3ExitInitializationMode", 2),
        ("fmi3DoStep", 4),
        ("fmi3SetFloat64", 2),
        ("fmi3GetFloat64", 5),
        ("fmi3Terminate", 2),
        ("fmi3FreeInstance", 2),
    ):
        require(len(by_function.get(symbol, [])) == expected,
                f"{symbol}: expected {expected} calls, got {len(by_function.get(symbol, []))}")

    for symbol in ("fmi3EnterInitializationMode", "fmi3ExitInitializationMode",
                   "fmi3DoStep", "fmi3SetFloat64", "fmi3Terminate"):
        require(all(event["status"] == "ok" and event["status_code"] == 0
                    for event in by_function[symbol]),
                f"{symbol}: non-OK FMI status recorded in the positive path")
    require(all(event["status"] == "void_return" for event in by_function["fmi3FreeInstance"]),
            "fmi3FreeInstance must be recorded as a void return")

    steps = by_function["fmi3DoStep"]
    # Calls are interleaved per communication interval: source, then exchanger.
    expected_starts = [0.0, 0.0, 1.0, 1.0]
    expected_ends = [1.0, 1.0, 2.0, 2.0]
    for index, event in enumerate(steps):
        payload = event["payload"]
        require(payload.get("no_set_fmu_state_prior_to_current_point") is True,
                f"doStep {index}: unexpected state-retention flag")
        require(decode_f64(payload.get("current_communication_point_bits")) == expected_starts[index],
                f"doStep {index}: current communication point mismatch")
        require(decode_f64(payload.get("communication_step_size_bits")) == 1.0,
                f"doStep {index}: step size mismatch")
        require(decode_f64(payload.get("last_successful_time_bits")) == expected_ends[index],
                f"doStep {index}: returned successful time mismatch")
        require(payload.get("event_handling_needed") is False
                and payload.get("terminate_simulation") is False
                and payload.get("early_return") is False,
                f"doStep {index}: unexpected step outcome flags")

    sets = by_function["fmi3SetFloat64"]
    for event in sets:
        payload = event["payload"]
        require(payload.get("value_references") == [21], "setFloat64 wrote an unexpected value reference")
        require(payload.get("n_value_references") == 1 and payload.get("n_values") == 1,
                "setFloat64 array lengths mismatch")
        values = payload.get("values", [])
        require(len(values) == 1 and decode_f64(values[0].get("raw_f64_bits")) == 500000.0,
                "setFloat64 did not exchange the observed 500 kW source value")

    gets = by_function["fmi3GetFloat64"]
    successful = [event for event in gets if event["status"] == "ok"]
    failed = [event for event in gets if event["status"] in ("error", "exception")]
    require(len(successful) == 4 and len(failed) == 1,
            "getFloat64 success/failure disposition mismatch")
    geo_outputs = [event for event in successful if event["federate_id"] == "geothermal-source"]
    hx_outputs = [event for event in successful if event["federate_id"] == "heat-exchanger-buffer"]
    require(len(geo_outputs) == 2 and len(hx_outputs) == 2,
            "getFloat64 calls are not split two source / two exchanger observations")
    for event in geo_outputs:
        payload = event["payload"]
        require(payload.get("value_references") == [11], "source getFloat64 VR mismatch")
        values = payload.get("values", [])
        require(len(values) == 1 and decode_f64(values[0].get("raw_f64_bits")) == 500000.0,
                "source getFloat64 did not retain 500 kW output bits")
    expected_hx = ([480000.0, 10000.0, 10000.0, 10000.0],
                   [480000.0, 10000.0, 10000.0, 20000.0])
    for event, expected_values in zip(hx_outputs, expected_hx):
        payload = event["payload"]
        require(payload.get("value_references") == [31, 32, 33, 34],
                "heat-exchanger getFloat64 VR map mismatch")
        values = payload.get("values", [])
        observed = [decode_f64(value.get("raw_f64_bits")) for value in values]
        require(observed == list(expected_values),
                f"heat-exchanger values mismatch: {observed!r}")

    rejected = failed[0]
    require(rejected["federate_id"] == "geothermal-source"
            and rejected["status"] == "error" and rejected["status_code"] == 3,
            "unknown-value-reference negative control did not retain FMI Error status")
    require(rejected["payload"].get("value_references") == [999],
            "unknown-value-reference negative control identity missing")
    require("values" not in rejected["payload"],
            "undefined output values from a failed GetFloat64 call must not be recorded")

    summary = transcript["summary"]
    require(summary["api_events"] == len(events), "summary API event count mismatch")
    require(summary["do_step_calls"] == 4 and summary["set_float64_calls"] == 2
            and summary["get_float64_calls"] == 5,
            "summary call totals mismatch")
    require(summary["successful_get_float64_calls"] == 4
            and summary["failed_get_float64_calls"] == 1,
            "summary get success/failure counters mismatch")
    require(summary["input_j"] == 1000000.0 and summary["service_j"] == 960000.0
            and summary["loss_j"] == 20000.0 and summary["storage_delta_j"] == 20000.0
            and summary["final_stored_energy_j"] == 20000.0,
            "summary energy values mismatch")
    require(math.isclose(summary["residual_j"], 0.0, rel_tol=0.0, abs_tol=1.0e-9),
            "summary conservation residual is nonzero")

def main() -> int:
    if not TRANSCRIPT.is_file():
        raise FileNotFoundError(TRANSCRIPT)
    schema = load_json_no_duplicate_keys(SCHEMA)
    transcript = load_json_no_duplicate_keys(TRANSCRIPT)
    validate_schema_shape(transcript, schema)
    validate_semantics(transcript)
    digest = hashlib.sha256(TRANSCRIPT.read_bytes()).hexdigest()
    digest_path = TRANSCRIPT.with_name("fmi3-api-transcript.sha256")
    digest_path.write_text(f"{digest}  {TRANSCRIPT.name}\n", encoding="utf-8")
    print("FMPy FMI dispatch transcript contract: PASS")
    print(f"api_events={len(transcript['events'])} doStep=4 setFloat64=2 getFloat64=5")
    print("unknown_value_reference=ERROR retained; Float64 raw bits and artifact digests checked")
    print(f"transcript_sha256={digest}")
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"FMPy FMI dispatch transcript contract: FAIL: {error}", file=sys.stderr)
        raise
