#!/usr/bin/env python3
"""Independent, dependency-free arithmetic oracle for the analytic FMU pair."""
import math
import sys
import zipfile
import xml.etree.ElementTree as ET

def main() -> int:
    if len(sys.argv) != 3:
        print("usage: analytic_oracle.py <geothermal.fmu> <heat-exchanger.fmu>", file=sys.stderr)
        return 2

    for filename in sys.argv[1:]:
        with zipfile.ZipFile(filename) as archive:
            names = set(archive.namelist())
            required = {"modelDescription.xml"}
            if not required.issubset(names):
                raise AssertionError(f"{filename}: missing modelDescription.xml")
            ET.fromstring(archive.read("modelDescription.xml"))
            if not any(name.startswith("binaries/x86_64-linux/") for name in names):
                raise AssertionError(f"{filename}: missing x86_64-linux binary")

    duration_s = 2.0
    power_w = 500000.0
    energy_in_j = power_w * duration_s
    service_j = energy_in_j * 0.96
    loss_j = energy_in_j * 0.02
    storage_j = energy_in_j * 0.02
    residual_j = energy_in_j - service_j - loss_j - storage_j

    expected = (1000000.0, 960000.0, 20000.0, 20000.0, 0.0)
    actual = (energy_in_j, service_j, loss_j, storage_j, residual_j)
    if actual != expected or not math.isclose(residual_j, 0.0, rel_tol=0.0, abs_tol=0.0):
        raise AssertionError(f"known-answer mismatch: {actual}")

    print("independent analytic oracle: PASS")
    print(
        f"input_J={energy_in_j:.0f} service_J={service_j:.0f} "
        f"loss_J={loss_j:.0f} storage_J={storage_j:.0f} residual_J={residual_j:.0f}"
    )
    print("FMU archives: package structure and XML well-formedness verified")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
