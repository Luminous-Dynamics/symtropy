#!/usr/bin/env python3
"""Coupled FMI 3 Co-Simulation run through the independent FMPy importer."""
from pathlib import Path
from tempfile import TemporaryDirectory
import math

from fmpy import extract, read_model_description
from fmpy.fmi3 import FMU3Slave
from fmpy.validation import validate_fmu

ROOT = Path(__file__).resolve().parents[1]
STAGE = ROOT / "build" / "stage"
GEO_FMU = STAGE / "GeoFusion-GeothermalSource.fmu"
HX_FMU = STAGE / "GeoFusion-HeatExchangerBuffer.fmu"

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
    slave.instantiate(
        visible=False,
        loggingOn=False,
        eventModeUsed=False,
        earlyReturnAllowed=False,
    )
    slave.enterInitializationMode(startTime=0.0, stopTime=2.0)
    slave.exitInitializationMode()
    return slave

def main() -> int:
    for path in (GEO_FMU, HX_FMU):
        if not path.is_file():
            raise FileNotFoundError(path)

    with TemporaryDirectory(prefix="geofusion-fmpy-") as temporary:
        workdir = Path(temporary)
        geothermal = create_slave(GEO_FMU, "geothermal-source", workdir)
        exchanger = create_slave(HX_FMU, "heat-exchanger-buffer", workdir)
        total_input = 0.0
        total_service = 0.0
        total_loss = 0.0
        total_storage = 0.0
        final_stored_energy = 0.0

        try:
            current_time = 0.0
            for _ in range(2):
                event, terminate, early_return, last_successful_time = geothermal.doStep(
                    currentCommunicationPoint=current_time,
                    communicationStepSize=1.0,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if event or terminate or early_return or last_successful_time != current_time + 1.0:
                    raise AssertionError("unexpected geothermal doStep result")

                thermal_power = geothermal.getFloat64([11])[0]
                exchanger.setFloat64([21], [thermal_power])

                event, terminate, early_return, last_successful_time = exchanger.doStep(
                    currentCommunicationPoint=current_time,
                    communicationStepSize=1.0,
                    noSetFMUStatePriorToCurrentPoint=True,
                )
                if event or terminate or early_return or last_successful_time != current_time + 1.0:
                    raise AssertionError("unexpected heat-exchanger doStep result")

                service, loss, storage, stored_energy = exchanger.getFloat64([31, 32, 33, 34])
                if (service, loss, storage) != (480000.0, 10000.0, 10000.0):
                    raise AssertionError("FMPy thermal allocation output mismatch")

                total_input += thermal_power
                total_service += service
                total_loss += loss
                total_storage += storage
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

            print("FMPy FMI 3.0.2 coupled importer smoke: PASS")
            print(
                f"input_J={total_input:.0f} service_J={total_service:.0f} "
                f"loss_J={total_loss:.0f} storage_J={total_storage:.0f} "
                f"residual_J={residual:.0f} final_stored_J={final_stored_energy:.0f}"
            )
            print("FMPy modelDescription validation: PASS")
            print("negative_control=unknown_value_reference: REJECTED")
        finally:
            try:
                geothermal.terminate()
            finally:
                geothermal.freeInstance()
            try:
                exchanger.terminate()
            finally:
                exchanger.freeInstance()
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
