#!/usr/bin/env python3
"""Execute the integrated-flux FMI pair through pinned FMPy.

The source reports cumulative and per-interval energy in J. The receiver
consumes exactly one interval-energy value per successful communication step.
These are synthetic analytic FMUs, not physical geothermal measurements.
"""
from __future__ import annotations

from math import isclose
from pathlib import Path
from tempfile import TemporaryDirectory
import math

from fmpy import extract, read_model_description
from fmpy.fmi1 import FMICallException
from fmpy.fmi3 import FMU3Slave
from fmpy.validation import validate_fmu

ROOT = Path(__file__).resolve().parents[1]
STAGE = ROOT / "build" / "stage"
SOURCE_FMU = STAGE / "GeoFusion-IntegratedFluxSource.fmu"
RECEIVER_FMU = STAGE / "GeoFusion-IntegratedFluxReceiver.fmu"

MEAN_POWER_W = 500_000.0
AMPLITUDE_W = 100_000.0
OMEGA_RAD_S = 2.0 * math.pi / 400.0
ENDPOINT_RESOLUTION_BUDGET_J = 1.0e-6


def sinc(value: float) -> float:
    if abs(value) < 1.0e-4:
        value2 = value * value
        return 1.0 - value2 / 6.0 + value2 * value2 / 120.0
    return math.sin(value) / value


def cumulative_energy_j(time_s: float) -> float:
    half_phase = OMEGA_RAD_S * time_s * 0.5
    return (
        MEAN_POWER_W * time_s
        + (2.0 * AMPLITUDE_W / OMEGA_RAD_S) * math.sin(half_phase) ** 2
    )


def interval_energy_j(start_s: float, end_s: float) -> float:
    duration = end_s - start_s
    midpoint = start_s + duration * 0.5
    return (
        MEAN_POWER_W * duration
        + AMPLITUDE_W
        * duration
        * math.sin(OMEGA_RAD_S * midpoint)
        * sinc(OMEGA_RAD_S * duration * 0.5)
    )


def float64_resolution_bound(previous: float, current: float) -> float:
    """Conservative sum of one-ULP endpoint spacings for non-negative counters."""
    def ulp(value: float) -> float:
        return math.ulp(value) if value != 0.0 else math.ulp(0.0)
    return ulp(previous) + ulp(current)


def make_slave(path: Path, name: str, workdir: Path, model_identifier: str):
    problems = validate_fmu(str(path))
    if problems:
        raise AssertionError(f"{path.name} failed FMPy validation: {problems}")
    description = read_model_description(str(path), validate=True)
    if description.coSimulation.modelIdentifier != model_identifier:
        raise AssertionError(f"{path.name} modelIdentifier mismatch")
    unzip_dir = workdir / name
    unzip_dir.mkdir(parents=True, exist_ok=True)
    extract(str(path), unzipdir=str(unzip_dir))
    slave = FMU3Slave(
        guid=description.guid,
        unzipDirectory=str(unzip_dir),
        modelIdentifier=description.coSimulation.modelIdentifier,
        instanceName=name,
    )
    return slave


def expect_fmi_error(operation, label: str) -> None:
    try:
        operation()
    except FMICallException as error:
        if error.status != 3:
            raise AssertionError(
                f"negative control returned FMI status {error.status}, expected fmi3Error: {label}"
            ) from error
        return
    except Exception as error:
        raise AssertionError(
            f"negative control raised a non-FMI exception: {label}: {type(error).__name__}: {error}"
        ) from error
    raise AssertionError(f"negative control was accepted: {label}")


def main() -> int:
    for path in (SOURCE_FMU, RECEIVER_FMU):
        if not path.is_file():
            raise FileNotFoundError(path)

    with TemporaryDirectory(prefix="geofusion-integrated-flux-") as temporary:
        workdir = Path(temporary)
        source = make_slave(SOURCE_FMU, "integrated-flux-source", workdir, "IntegratedFluxSource")
        receiver = make_slave(RECEIVER_FMU, "integrated-flux-receiver", workdir, "IntegratedFluxReceiver")
        slaves = (source, receiver)
        try:
            for slave in slaves:
                slave.instantiate(
                    visible=False,
                    loggingOn=False,
                    eventModeUsed=False,
                    earlyReturnAllowed=False,
                )
                slave.enterInitializationMode(startTime=0.0, stopTime=120.0)
                slave.exitInitializationMode()

            expect_fmi_error(
                lambda: source.enterInitializationMode(startTime=0.0, stopTime=120.0),
                "source re-enter initialization mode",
            )
            expect_fmi_error(
                lambda: receiver.enterInitializationMode(startTime=0.0, stopTime=120.0),
                "receiver re-enter initialization mode",
            )

            current_time = 0.0
            previous_cumulative = 0.0
            previous_stored_energy = 0.0
            totals = {"input": 0.0, "service": 0.0, "loss": 0.0, "storage": 0.0}
            step_bounds = [(0.0, 30.0), (30.0, 120.0)]

            for start_s, end_s in step_bounds:
                duration_s = end_s - start_s
                if start_s != current_time:
                    raise AssertionError("test master lost the committed time chain")

                # Producer advances first and exposes the completed interval J value.
                event, terminate, early_return, source_last = source.doStep(
                    currentCommunicationPoint=start_s,
                    communicationStepSize=duration_s,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if event or terminate or early_return or source_last != end_s:
                    raise AssertionError("unexpected producer FMI step result")

                cumulative_energy, direct_interval_energy = source.getFloat64([11, 12])
                if cumulative_energy < previous_cumulative:
                    raise AssertionError("producer cumulative energy regressed")
                delta_energy = cumulative_energy - previous_cumulative
                resolution_bound = float64_resolution_bound(previous_cumulative, cumulative_energy)
                if resolution_bound > ENDPOINT_RESOLUTION_BUDGET_J:
                    raise AssertionError("producer cumulative counter exceeded its Float64 resolution budget")

                expected_interval = interval_energy_j(start_s, end_s)
                if not isclose(direct_interval_energy, expected_interval, rel_tol=0.0, abs_tol=2e-6):
                    raise AssertionError("producer per-interval energy output differs from analytic integral")
                if not isclose(delta_energy, direct_interval_energy, rel_tol=0.0, abs_tol=2e-6):
                    raise AssertionError("cumulative-energy delta disagrees with direct interval-energy output")
                if not isclose(cumulative_energy, cumulative_energy_j(end_s), rel_tol=0.0, abs_tol=2e-6):
                    raise AssertionError("producer cumulative-energy output differs from analytic known answer")

                # Consumer accepts one non-negative interval-energy input and commits it once.
                receiver.setFloat64([21], [direct_interval_energy])
                event, terminate, early_return, receiver_last = receiver.doStep(
                    currentCommunicationPoint=start_s,
                    communicationStepSize=duration_s,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if event or terminate or early_return or receiver_last != source_last:
                    raise AssertionError("producer/receiver lastSuccessfulTime mismatch")

                service, loss, storage_delta, cumulative_stored = receiver.getFloat64([31, 32, 33, 34])
                residual = direct_interval_energy - service - loss - storage_delta
                if abs(residual) > 16.0 * math.ulp(max(direct_interval_energy, 1.0)):
                    raise AssertionError(f"receiver energy allocation residual too large: {residual}")
                if not isclose(service, direct_interval_energy * 0.96, rel_tol=0.0, abs_tol=1e-7):
                    raise AssertionError("service energy fraction mismatch")
                if not isclose(loss, direct_interval_energy * 0.02, rel_tol=0.0, abs_tol=1e-7):
                    raise AssertionError("loss energy fraction mismatch")
                if not isclose(storage_delta, direct_interval_energy * 0.02, rel_tol=0.0, abs_tol=1e-7):
                    raise AssertionError("storage energy fraction mismatch")
                if not isclose(cumulative_stored - previous_stored_energy, storage_delta,
                               rel_tol=0.0, abs_tol=1e-7):
                    raise AssertionError("stored-energy state increment does not match interval storage J")

                totals["input"] += direct_interval_energy
                totals["service"] += service
                totals["loss"] += loss
                totals["storage"] += storage_delta

                # A duplicate/stale commit must fail; it must not consume energy twice.
                expect_fmi_error(
                    lambda: receiver.doStep(
                        currentCommunicationPoint=start_s,
                        communicationStepSize=duration_s,
                        noSetFMUStatePriorToCurrentPoint=True,
                    ),
                    "receiver duplicate interval consumption",
                )
                if start_s == 0.0:
                    expect_fmi_error(
                        lambda: receiver.setFloat64([21], [-1.0]),
                        "negative interval energy",
                    )
                    expect_fmi_error(
                        lambda: source.doStep(
                            currentCommunicationPoint=0.0,
                            communicationStepSize=30.0,
                            noSetFMUStatePriorToCurrentPoint=True,
                        ),
                        "producer stale currentCommunicationPoint",
                    )

                previous_cumulative = cumulative_energy
                previous_stored_energy = cumulative_stored
                current_time = receiver_last

            if current_time != 120.0:
                raise AssertionError("campaign did not reach its declared final time")
            expected_total = cumulative_energy_j(120.0)
            if not isclose(totals["input"], expected_total, rel_tol=0.0, abs_tol=2e-6):
                raise AssertionError("total integrated source energy differs from the analytic reference")
            total_residual = totals["input"] - totals["service"] - totals["loss"] - totals["storage"]
            if abs(total_residual) > 16.0 * math.ulp(max(totals["input"], 1.0)):
                raise AssertionError(f"federation energy residual too large: {total_residual}")

            expect_fmi_error(
                lambda: source.doStep(
                    currentCommunicationPoint=120.0,
                    communicationStepSize=1.0,
                    noSetFMUStatePriorToCurrentPoint=True,
                ),
                "producer step beyond declared stop time",
            )

            print("FMPy integrated-flux FMI 3.0.2 pair: PASS")
            print(f"intervals={len(step_bounds)}")
            print(f"input_J={totals['input']:.9f} service_J={totals['service']:.9f} "
                  f"loss_J={totals['loss']:.9f} storage_J={totals['storage']:.9f}")
            print(f"total_residual_J={total_residual:.12g}")
            print("negative_controls=duplicate-consume,negative-J,stale-step,reenter-init,stop-time-overrun: FMI_ERROR")
            print("claim_ceiling=synthetic FMI interface and energy accounting only")
        finally:
            for slave in slaves:
                try:
                    slave.terminate()
                except Exception:
                    pass
                try:
                    slave.freeInstance()
                except Exception:
                    pass

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
