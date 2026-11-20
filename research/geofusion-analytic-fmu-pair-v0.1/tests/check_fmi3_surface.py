#!/usr/bin/env python3
"""Verify FMI implementation/stub closure against the pinned FMPy symbol surface."""
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
MANIFEST=json.loads((ROOT/"tests/fmi3-symbol-manifest.json").read_text(encoding="utf-8"))
SOURCE_DIR=ROOT/"src"
MODEL_DIR=ROOT/"model-description"

def function_blocks(source: str):
    pattern=re.compile(
        r"^\s*(?:FMI3_Export\s+)?(?:const\s+char\s*\*|fmi3Status|fmi3Instance|void)\s+(fmi3[A-Za-z0-9_]+)\s*\(",
        re.MULTILINE,
    )
    matches=list(pattern.finditer(source))
    blocks={}
    for match in matches:
        open_brace=source.find("{",match.end())
        if open_brace<0:
            raise AssertionError(f"missing function body for {match.group(1)}")
        depth=0
        end=None
        for index in range(open_brace,len(source)):
            if source[index]=="{":
                depth+=1
            elif source[index]=="}":
                depth-=1
                if depth==0:
                    end=index+1
                    break
        if end is None:
            raise AssertionError(f"unterminated function body for {match.group(1)}")
        name=match.group(1)
        if name in blocks:
            raise AssertionError(f"duplicate function definition: {name}")
        blocks[name]=source[match.start():end]
    return blocks

def main() -> int:
    # These are separate shared libraries. They intentionally export the
    # same FMI C-API names; duplicate detection is scoped to each link unit.
    source_units = {
        "GeothermalSource": ("geothermal_source.c", "fmi3_unsupported.c"),
        "HeatExchangerBuffer": ("heat_exchanger_buffer.c", "fmi3_unsupported.c"),
    }
    expected=set(MANIFEST["importer_resolved_symbols"])
    implemented=set(MANIFEST["implemented_symbols"])
    unsupported=set(MANIFEST["unsupported_fail_closed_symbols"])
    if expected != implemented | unsupported:
        raise AssertionError("manifest implementation/stub sets do not partition importer symbol surface")
    if implemented & unsupported:
        raise AssertionError("implemented and unsupported symbol sets overlap")

    common_source = (SOURCE_DIR / "fmi3_unsupported.c").read_text(encoding="utf-8")
    for component_name, source_names in source_units.items():
        blocks={}
        for source_name in source_names:
            source = common_source if source_name == "fmi3_unsupported.c" else (
                SOURCE_DIR / source_name
            ).read_text(encoding="utf-8")
            for symbol,body in function_blocks(source).items():
                if symbol in blocks:
                    raise AssertionError(
                        f"{component_name}: duplicate FMI symbol {symbol} within one shared library"
                    )
                blocks[symbol]=body

        if set(blocks) != expected:
            missing=sorted(expected-set(blocks))
            extra=sorted(set(blocks)-expected)
            raise AssertionError(
                f"{component_name}: FMI symbol surface mismatch; missing={missing}, extra={extra}"
            )

        for symbol in unsupported:
            body=blocks[symbol]
            if "return fmi3Error;" not in body and "return NULL;" not in body:
                raise AssertionError(
                    f"{component_name}: unsupported symbol does not fail closed: {symbol}"
                )

    if '"3.0"' not in blocks["fmi3GetVersion"] or '"3.0.2"' in blocks["fmi3GetVersion"]:
        raise AssertionError("FMI version function does not return the pinned interface version")

    for xml_path in sorted(MODEL_DIR.glob("*.xml")):
        root=ET.parse(xml_path).getroot()
        cosim=root.find("CoSimulation")
        if cosim is None:
            raise AssertionError(f"{xml_path.name}: CoSimulation interface missing")
        for output in root.findall("./ModelVariables/Float64[@causality='output']"):
            if "start" in output.attrib:
                raise AssertionError(f"{xml_path.name}: calculated continuous output {output.get('name')} must not declare start")
        if cosim.get("canGetAndSetFMUState","false").lower()!="false":
            raise AssertionError(f"{xml_path.name}: unqualified FMU state capability advertised")
        if cosim.get("canSerializeFMUState","false").lower()!="false":
            raise AssertionError(f"{xml_path.name}: unqualified serialized state capability advertised")

    print("FMI API symbol/capability closure: PASS")
    print(f"shared_libraries={len(source_units)} pinned_symbols_per_library={len(expected)} implemented={len(implemented)} fail_closed_unsupported={len(unsupported)}")
    print("FMU-state and serialization capabilities: not advertised")
    return 0

if __name__=="__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"FMI API symbol/capability closure: FAIL: {error}",file=sys.stderr)
        raise
