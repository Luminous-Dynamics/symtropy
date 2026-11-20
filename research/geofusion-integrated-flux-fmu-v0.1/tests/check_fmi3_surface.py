#!/usr/bin/env python3
"""Audit each integrated-flux FMU as an independent FMI 3 Co-Simulation link unit."""
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT.parent / "geofusion-analytic-fmu-pair-v0.1"
MANIFEST = json.loads(
    (BASE / "tests" / "fmi3-symbol-manifest.json").read_text(encoding="utf-8")
)
SOURCE_DIR = ROOT / "src"
MODEL_DIR = ROOT / "model-description"

def function_blocks(source: str):
    pattern = re.compile(
        r"^\s*(?:FMI3_Export\s+)?(?:const\s+char\s*\*|fmi3Status|fmi3Instance|void)\s+(fmi3[A-Za-z0-9_]+)\s*\(",
        re.MULTILINE,
    )
    matches = list(pattern.finditer(source))
    blocks = {}
    for match in matches:
        open_brace = source.find("{", match.end())
        if open_brace < 0:
            raise AssertionError(f"missing function body for {match.group(1)}")
        depth = 0
        end = None
        for index in range(open_brace, len(source)):
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
                if depth == 0:
                    end = index + 1
                    break
        if end is None:
            raise AssertionError(f"unterminated function body for {match.group(1)}")
        name = match.group(1)
        if name in blocks:
            raise AssertionError(f"duplicate function definition: {name}")
        blocks[name] = source[match.start():end]
    return blocks


def main() -> int:
    source_units = {
        "IntegratedFluxSource": ("integrated_flux_source.c", "fmi3_unsupported.c"),
        "IntegratedFluxReceiver": ("integrated_flux_receiver.c", "fmi3_unsupported.c"),
    }
    expected = set(MANIFEST["importer_resolved_symbols"])
    implemented = set(MANIFEST["implemented_symbols"])
    unsupported = set(MANIFEST["unsupported_fail_closed_symbols"])
    if expected != implemented | unsupported:
        raise AssertionError("manifest implementation/stub sets do not partition importer symbol surface")
    if implemented & unsupported:
        raise AssertionError("implemented and unsupported symbol sets overlap")

    common_stub = (BASE / "src" / "fmi3_unsupported.c").read_text(encoding="utf-8")
    for component_name, source_names in source_units.items():
        blocks = {}
        for source_name in source_names:
            source = common_stub if source_name == "fmi3_unsupported.c" else (
                SOURCE_DIR / source_name
            ).read_text(encoding="utf-8")
            for symbol, body in function_blocks(source).items():
                if symbol in blocks:
                    raise AssertionError(
                        f"{component_name}: duplicate FMI symbol {symbol} within one shared library"
                    )
                blocks[symbol] = body
        if set(blocks) != expected:
            missing = sorted(expected - set(blocks))
            extra = sorted(set(blocks) - expected)
            raise AssertionError(
                f"{component_name}: FMI symbol surface mismatch; missing={missing}, extra={extra}"
            )
        for symbol in unsupported:
            body = blocks[symbol]
            if "return fmi3Error;" not in body and "return NULL;" not in body:
                raise AssertionError(
                    f"{component_name}: unsupported symbol does not fail closed: {symbol}"
                )
        if '"3.0"' not in blocks["fmi3GetVersion"]:
            raise AssertionError(f"{component_name}: FMI version function must return 3.0")

    expected_variables = {
        "integrated-flux-source.xml": {
            "cumulativeThermalEnergy": (11, "output", "J"),
            "intervalThermalEnergy": (12, "output", "J"),
        },
        "integrated-flux-receiver.xml": {
            "intervalThermalEnergy": (21, "input", "J"),
            "serviceEnergy": (31, "output", "J"),
            "lossEnergy": (32, "output", "J"),
            "storageDeltaEnergy": (33, "output", "J"),
            "storedEnergy": (34, "output", "J"),
        },
    }
    for filename, expected_map in expected_variables.items():
        root = ET.parse(MODEL_DIR / filename).getroot()
        if root.get("fmiVersion") != "3.0":
            raise AssertionError(f"{filename}: expected FMI 3.0")
        cosim = root.find("CoSimulation")
        if cosim is None or cosim.get("canHandleVariableCommunicationStepSize", "false").lower() != "true":
            raise AssertionError(f"{filename}: CoSimulation/variable-step capability missing")
        if cosim.get("canGetAndSetFMUState", "false").lower() != "false":
            raise AssertionError(f"{filename}: unqualified state capability advertised")
        if cosim.get("canSerializeFMUState", "false").lower() != "false":
            raise AssertionError(f"{filename}: unqualified serialized state capability advertised")

        units = root.findall("./UnitDefinitions/Unit")
        unit_map = {unit.get("name"): unit.find("BaseUnit") for unit in units}
        joule = unit_map.get("J")
        if joule is None or {
            "kg": joule.get("kg"), "m": joule.get("m"), "s": joule.get("s")
        } != {"kg": "1", "m": "2", "s": "-2"}:
            raise AssertionError(f"{filename}: joule unit must be kg*m^2*s^-2")

        actual = {}
        for variable in root.findall("./ModelVariables/Float64"):
            name = variable.get("name")
            actual[name] = (
                int(variable.get("valueReference")),
                variable.get("causality"),
                variable.get("unit"),
            )
        if actual != expected_map:
            raise AssertionError(f"{filename}: variable map mismatch: {actual!r}")
        for output in root.findall("./ModelVariables/Float64[@causality='output']"):
            if "start" in output.attrib:
                raise AssertionError(f"{filename}: calculated output {output.get('name')} must not declare start")

    print("Integrated-flux FMI implementation/stub closure: PASS")
    print(
        f"shared_libraries={len(source_units)} pinned_symbols_per_library={len(expected)} "
        f"implemented={len(implemented)} fail_closed_unsupported={len(unsupported)}"
    )
    print("FMI J-port maps and state-capability declarations: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"Integrated-flux FMI surface audit: FAIL: {error}", file=sys.stderr)
        raise
