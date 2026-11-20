#!/usr/bin/env python3
"""Coupled FMI 3 Co-Simulation run plus structured FMPy dispatch transcript.

Instrumentation wraps the pinned FMPy _call method immediately around its
ctypes function-pointer invocation. It is an importer-dispatch observation,
not a direct C-level interposer.
"""
from pathlib import Path
from tempfile import TemporaryDirectory
import ctypes
import hashlib
from importlib.metadata import version as package_version
import json
import math
import platform
import struct
import sys

from fmpy import __version__ as FMPY_VERSION
from fmpy import extract, read_model_description
from fmpy.fmi3 import FMU3Slave
from fmpy.validation import validate_fmu

ROOT = Path(__file__).resolve().parents[1]
STAGE = ROOT / "build" / "stage"
GEO_FMU = STAGE / "GeoFusion-GeothermalSource.fmu"
HX_FMU = STAGE / "GeoFusion-HeatExchangerBuffer.fmu"
STATUS_LABELS = {0: "ok", 1: "warning", 2: "discard", 3: "error", 4: "fatal", 5: "pending"}

def scalar(value):
    return value.value if hasattr(value, "value") else value

def f64_bits(value):
    return struct.pack(">d", float(scalar(value))).hex()

def text_value(value):
    value = scalar(value)
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    if value is None:
        return None
    return str(value)

def array_values(value, count):
    return [scalar(value[index]) for index in range(count)]

def pointer_output(value):
    obj = getattr(value, "_obj", None)
    if obj is None:
        return None
    return scalar(getattr(obj, "value", obj))

def file_sha256(path: Path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def instrument_dispatch(slave: FMU3Slave, federate_id: str, events: list):
    """Capture structured arguments and results around a real ctypes call."""
    original_call = slave._call

    def observed_call(function_name, *args):
        event = {
            "sequence": len(events),
            "federate_id": federate_id,
            "function": function_name,
            "argument_count": len(args),
            "argument_types": [type(arg).__name__ for arg in args],
            "status_code": None,
            "status": "returned",
            "exception_type": None,
            "exception_message": None,
            "payload": {},
        }

        if function_name == "fmi3InstantiateCoSimulation":
            event["payload"] = {
                "instance_name": text_value(args[0]),
                "instantiation_token": text_value(args[1]),
                "resource_path": text_value(args[2]),
                "visible": bool(scalar(args[3])),
                "logging_on": bool(scalar(args[4])),
                "event_mode_used": bool(scalar(args[5])),
                "early_return_allowed": bool(scalar(args[6])),
                "required_intermediate_variables": array_values(args[7], int(scalar(args[8]))),
            }
        elif function_name == "fmi3EnterInitializationMode":
            event["payload"] = {
                "tolerance_defined": bool(scalar(args[1])),
                "tolerance_bits": f64_bits(args[2]),
                "start_time_bits": f64_bits(args[3]),
                "stop_time_defined": bool(scalar(args[4])),
                "stop_time_bits": f64_bits(args[5]),
            }
        elif function_name == "fmi3GetFloat64":
            n_refs = int(scalar(args[2]))
            n_values = int(scalar(args[4]))
            event["payload"] = {
                "value_references": array_values(args[1], n_refs),
                "n_value_references": n_refs,
                "n_values": n_values,
            }
        elif function_name == "fmi3SetFloat64":
            n_refs = int(scalar(args[2]))
            n_values = int(scalar(args[4]))
            values = array_values(args[3], n_values)
            event["payload"] = {
                "value_references": array_values(args[1], n_refs),
                "n_value_references": n_refs,
                "n_values": n_values,
                "values": [
                    {"value": float(value), "raw_f64_bits": f64_bits(value)}
                    for value in values
                ],
            }
        elif function_name == "fmi3DoStep":
            event["payload"] = {
                "current_communication_point_bits": f64_bits(args[1]),
                "communication_step_size_bits": f64_bits(args[2]),
                "no_set_fmu_state_prior_to_current_point": bool(scalar(args[3])),
            }

        try:
            result = original_call(function_name, *args)
            if function_name in ("fmi3GetFloat64", "fmi3SetFloat64"):
                if function_name == "fmi3GetFloat64":
                    n_values = int(scalar(args[4]))
                    values = array_values(args[3], n_values)
                    event["payload"]["values"] = [
                        {"value": float(value), "raw_f64_bits": f64_bits(value)}
                        for value in values
                    ]
            elif function_name == "fmi3DoStep":
                event["payload"].update({
                    "event_handling_needed": bool(pointer_output(args[4])),
                    "terminate_simulation": bool(pointer_output(args[5])),
                    "early_return": bool(pointer_output(args[6])),
                    "last_successful_time_bits": f64_bits(pointer_output(args[7])),
                })
            if function_name == "fmi3InstantiateCoSimulation":
                # The C API returns an opaque instance pointer, not fmi3Status.
                # Do not misclassify an integer-address c_void_p result as a status.
                event["status"] = "returned"
                event["result_type"] = "instance_pointer"
                event["payload"]["instance_created"] = bool(result)
            elif isinstance(result, int):
                event["status_code"] = int(result)
                event["status"] = STATUS_LABELS.get(int(result), "unknown")
            elif result is None:
                event["status"] = "void_return"
            else:
                event["status"] = "returned"
                event["result_type"] = type(result).__name__
            return result
        except Exception as error:
            event["exception_type"] = type(error).__name__
            event["exception_message"] = str(error)
            status = getattr(error, "status", None)
            if status is not None:
                event["status_code"] = int(status)
                event["status"] = STATUS_LABELS.get(int(status), "unknown")
            else:
                event["status"] = "exception"
            raise
        finally:
            events.append(event)

    # _call is looked up dynamically by the pinned FMPy wrappers. Assigning a
    # closure to the instance intercepts those calls without changing the FMU.
    slave._call = observed_call

def create_slave(fmu_path: Path, instance_name: str, workdir: Path):
    problems = validate_fmu(str(fmu_path))
    if problems:
        raise AssertionError(f"{fmu_path.name} failed FMPy validation: {problems}")

    model_description = read_model_description(str(fmu_path), validate=True)
    unzip_dir = workdir / instance_name
    unzip_dir.mkdir(parents=True, exist_ok=True)
    extract(str(fmu_path), unzipdir=str(unzip_dir))

    model_identifier = model_description.coSimulation.modelIdentifier
    slave = FMU3Slave(
        guid=model_description.guid,
        unzipDirectory=str(unzip_dir),
        modelIdentifier=model_identifier,
        instanceName=instance_name,
    )
    return slave, model_description, unzip_dir

def main() -> int:
    for path in (GEO_FMU, HX_FMU):
        if not path.is_file():
            raise FileNotFoundError(path)

    events = []
    with TemporaryDirectory(prefix="geofusion-fmpy-") as temporary:
        workdir = Path(temporary)
        geothermal, geo_md, geo_dir = create_slave(GEO_FMU, "geothermal-source", workdir)
        exchanger, hx_md, hx_dir = create_slave(HX_FMU, "heat-exchanger-buffer", workdir)

        instrument_dispatch(geothermal, "geothermal-source", events)
        instrument_dispatch(exchanger, "heat-exchanger-buffer", events)

        total_input = 0.0
        total_service = 0.0
        total_loss = 0.0
        total_storage = 0.0
        final_stored_energy = 0.0

        try:
            for slave in (geothermal, exchanger):
                slave.instantiate(
                    visible=False,
                    loggingOn=False,
                    eventModeUsed=False,
                    earlyReturnAllowed=False,
                )
                slave.enterInitializationMode(startTime=0.0, stopTime=2.0)
                slave.exitInitializationMode()

            current_time = 0.0
            for _ in range(2):
                event, terminate, early_return, last_successful_time = geothermal.doStep(
                    currentCommunicationPoint=current_time,
                    communicationStepSize=1.0,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if event or terminate or early_return or last_successful_time != current_time + 1.0:
                    raise AssertionError("unexpected geothermal doStep result")
                geothermal_last_successful_time = last_successful_time

                thermal_power = geothermal.getFloat64([11])[0]
                exchanger.setFloat64([21], [thermal_power])

                event, terminate, early_return, last_successful_time = exchanger.doStep(
                    currentCommunicationPoint=current_time,
                    communicationStepSize=1.0,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if (event or terminate or early_return
                        or last_successful_time != geothermal_last_successful_time):
                    raise AssertionError("geothermal / heat-exchanger successful times do not match")

                service, loss, storage, stored_energy = exchanger.getFloat64([31, 32, 33, 34])
                if (service, loss, storage) != (480000.0, 10000.0, 10000.0):
                    raise AssertionError("FMPy thermal allocation output mismatch")

                interval_duration = last_successful_time - current_time
                if interval_duration <= 0.0:
                    raise AssertionError("federation produced a non-positive committed interval")
                storage_delta = stored_energy - final_stored_energy
                if not math.isclose(storage_delta, storage * interval_duration,
                                    rel_tol=0.0, abs_tol=1e-9):
                    raise AssertionError("buffer energy increment does not match storage power over interval")

                # Integrate powers over the exact interval; do not label sums of
                # power samples as energies merely because this smoke uses 1 s.
                total_input += thermal_power * interval_duration
                total_service += service * interval_duration
                total_loss += loss * interval_duration
                total_storage += storage_delta
                final_stored_energy = stored_energy
                current_time = last_successful_time

            if total_input != 1000000.0 or final_stored_energy != 20000.0:
                raise AssertionError("FMPy known-answer result mismatch")
            residual = total_input - total_service - total_loss - total_storage
            if not math.isclose(residual, 0.0, rel_tol=0.0, abs_tol=1e-9):
                raise AssertionError(f"FMPy thermal conservation residual: {residual}")

            try:
                geothermal.getFloat64([999])
            except Exception:
                pass
            else:
                raise AssertionError("FMPy importer did not surface an unknown value-reference error")

        finally:
            for slave in (geothermal, exchanger):
                try:
                    slave.terminate()
                finally:
                    slave.freeInstance()

    # The rejected unknown-value-reference call is retained as a negative
    # observation; do not strip or relabel it as a successful get.
    steps = [e for e in events if e["function"] == "fmi3DoStep"]
    sets = [e for e in events if e["function"] == "fmi3SetFloat64"]
    gets = [e for e in events if e["function"] == "fmi3GetFloat64"]
    if len(steps) != 4 or len(sets) != 2:
        raise AssertionError(f"unexpected API transcript cardinality: steps={len(steps)} sets={len(sets)}")
    if len([e for e in gets if e["status"] == "ok"]) != 4:
        raise AssertionError("expected four successful Float64 get calls (two source, two exchanger)")
    if not any(e["function"] == "fmi3GetFloat64" and e["status"] in ("error", "exception")
               for e in events):
        raise AssertionError("the invalid value-reference call was not preserved as a failed API event")
    for event in events:
        if event["function"] == "fmi3DoStep" and event["status"] != "ok":
            raise AssertionError(f"successful smoke step has non-OK transcript status: {event}")

    transcript = {
        "schema_version": "fmi3-dispatch-transcript-0.1.0",
        "observation_boundary": "fmpy-0.3.32 _call wrapper immediately around ctypes function-pointer invocation",
        "claim_ceiling": "Observed FMPy dispatches for this run only; not a direct C-level interposer or FMI-wide conformance result.",
        "importer": {"name": "FMPy", "version": FMPY_VERSION},
        "runtime": {
            "python": sys.version,
            "platform": platform.platform(),
            "dependencies": {
                name: package_version(name)
                for name in (
                    "FMPy", "attrs", "cmake", "jinja2", "lark",
                    "lxml", "msgpack", "nbformat", "numpy"
                )
            },
        },
        "fmus": {
            "geothermal-source": {
                "package_sha256": file_sha256(GEO_FMU),
                "model_description_sha256": file_sha256(geo_dir / "modelDescription.xml"),
                "binary_sha256": file_sha256(geo_dir / "binaries" / "x86_64-linux" / "GeothermalSource.so"),
                "model_name": geo_md.modelName,
                "model_identifier": geo_md.coSimulation.modelIdentifier,
            },
            "heat-exchanger-buffer": {
                "package_sha256": file_sha256(HX_FMU),
                "model_description_sha256": file_sha256(hx_dir / "modelDescription.xml"),
                "binary_sha256": file_sha256(hx_dir / "binaries" / "x86_64-linux" / "HeatExchangerBuffer.so"),
                "model_name": hx_md.modelName,
                "model_identifier": hx_md.coSimulation.modelIdentifier,
            },
        },
        "summary": {
            "api_events": len(events),
            "do_step_calls": len(steps),
            "set_float64_calls": len(sets),
            "get_float64_calls": len(gets),
            "successful_get_float64_calls": sum(e["status"] == "ok" for e in gets),
            "failed_get_float64_calls": sum(e["status"] in ("error", "exception") for e in gets),
            "input_j": total_input,
            "service_j": total_service,
            "loss_j": total_loss,
            "storage_delta_j": total_storage,
            "residual_j": residual,
            "final_stored_energy_j": final_stored_energy,
        },
        "events": events,
    }
    transcript_path = STAGE / "fmi3-api-transcript.json"
    transcript_path.write_text(json.dumps(transcript, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print("FMPy FMI 3.0.2 coupled importer smoke: PASS")
    print(
        f"input_J={total_input:.0f} service_J={total_service:.0f} "
        f"loss_J={total_loss:.0f} storage_J={total_storage:.0f} "
        f"residual_J={residual:.0f} final_stored_J={final_stored_energy:.0f}"
    )
    print("FMPy modelDescription validation: PASS")
    print("negative_control=unknown_value_reference: REJECTED and retained in transcript")
    print(f"structured_api_events={len(events)} transcript={transcript_path}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
