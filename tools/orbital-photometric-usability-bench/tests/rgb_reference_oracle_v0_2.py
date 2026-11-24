#!/usr/bin/env python3
"""Independent standard-library oracle for RGB camera/projector precondition and range controls."""
import hashlib
import json
import math
import sys
from pathlib import Path

SCHEMA="luminous.orbital-photometric-usability.rgb-corpus.v0.2"
RECEIPT_SCHEMA="luminous.orbital-photometric-usability.rgb-receipt.v0.2"
BENCHMARK_ID="ORBITAL-PHOTO-RGB-002"
CLAIM="synthetic_rgb_range_and_declared_camera_projector_preconditions_only"
CASE_COUNT=10
SCALE=1_000_000.0
GATES={"PASS","FAIL","INDETERMINATE"}


def q(value):
    if not math.isfinite(value) or not 0.0 <= value <= 1.0:
        raise ValueError("RGB_CHANNEL_OR_THRESHOLD_INVALID")
    return math.floor(value*SCALE+0.5)


def evaluate_case(case):
    if not case["case_id"] or not case["camera_id"] or not case["projector_id"] or not case["samples"] or not case["modes"]:
        raise ValueError("RGB_CASE_BOUNDS_INVALID")
    sample_ids=[s["sample_id"] for s in case["samples"]]
    mode_ids=[m["mode_id"] for m in case["modes"]]
    if len(sample_ids)!=len(set(sample_ids)) or len(mode_ids)!=len(set(mode_ids)):
        raise ValueError("RGB_ID_DUPLICATE")
    if set(e["mode_id"] for e in case["expected"])!=set(mode_ids) or len(case["expected"])!=len(mode_ids):
        raise ValueError("RGB_EXPECTED_MODE_SET_INVALID")
    for sample in case["samples"]:
        if any(sample[field] not in GATES for field in ("camera_fov","projector_fov","camera_depth_of_field","camera_line_of_sight")):
            raise ValueError("RGB_GATE_INVALID")
        if not sample["sample_id"] or len(sample["rgb_linear"])!=3:
            raise ValueError("RGB_SAMPLE_INVALID")
        for channel in sample["rgb_linear"]:
            q(channel)
    for mode in case["modes"]:
        dark=q(mode["darkness_threshold"]); sat=q(mode["saturation_threshold"])
        if not mode["mode_id"] or dark>=sat:
            raise ValueError("RGB_MODE_INVALID")

    mode_reports=[]
    for mode in case["modes"]:
        dark=q(mode["darkness_threshold"]); sat=q(mode["saturation_threshold"])
        results=[]
        for sample in case["samples"]:
            gates=[sample["camera_fov"],sample["projector_fov"],sample["camera_depth_of_field"],sample["camera_line_of_sight"]]
            geometry="FAIL" if "FAIL" in gates else "INDETERMINATE" if "INDETERMINATE" in gates else "PASS"
            if not sample["rgb_response_asserted"]:
                rgb=None; rgb_status="INDETERMINATE"; joint="INDETERMINATE"
            else:
                rgb=[q(x) for x in sample["rgb_linear"]]
                if any(x>sat for x in rgb):
                    rgb_status="SATURATED"
                elif not any(x>=dark for x in rgb):
                    rgb_status="UNDEREXPOSED"
                else:
                    rgb_status="USABLE"
                if geometry=="FAIL":
                    joint="GEOMETRY_REJECTED"
                elif geometry=="INDETERMINATE":
                    joint="INDETERMINATE"
                else:
                    joint={"USABLE":"USABLE","UNDEREXPOSED":"UNDEREXPOSED","SATURATED":"SATURATED","INDETERMINATE":"INDETERMINATE"}[rgb_status]
            results.append({
                "sample_id":sample["sample_id"],
                "geometry_gate":geometry,
                "rgb_micro_units":rgb,
                "rgb_status":rgb_status,
                "joint_status":joint
            })
        n=len(results)
        usable=sum(x["joint_status"]=="USABLE" for x in results)
        geom_rej=sum(x["joint_status"]=="GEOMETRY_REJECTED" for x in results)
        under=sum(x["joint_status"]=="UNDEREXPOSED" for x in results)
        sat_count=sum(x["joint_status"]=="SATURATED" for x in results)
        ind=sum(x["joint_status"]=="INDETERMINATE" for x in results)
        summary="INDETERMINATE" if ind else "FULLY_USABLE" if usable==n else "PARTIAL" if usable else "NOT_USABLE"
        acquisition={"FULLY_USABLE":"USABLE","PARTIAL":"PARTIAL","NOT_USABLE":"UNUSABLE","INDETERMINATE":"INDETERMINATE"}[summary]
        mode_reports.append({
            "mode_id":mode["mode_id"],"sample_count":n,"usable_count":usable,
            "geometry_rejected_count":geom_rej,"underexposed_count":under,"saturated_count":sat_count,
            "indeterminate_count":ind,"summary":summary,"acquisition":acquisition,"samples":results
        })
    byid={m["mode_id"]:m for m in mode_reports}
    matches=sum(byid[e["mode_id"]]["summary"]==e["summary"] and byid[e["mode_id"]]["acquisition"]==e["acquisition"] for e in case["expected"])
    return {"case_id":case["case_id"],"expected":case["expected"],"expected_matches":matches,
        "expected_mismatches":len(case["expected"])-matches,"modes":mode_reports}


def main():
    if len(sys.argv)!=3:
        raise SystemExit("usage: rgb_reference_oracle.py <corpus.json> <40-hex-source-commit>")
    raw=Path(sys.argv[1]).read_bytes()
    sha=sys.argv[2]
    if len(sha)!=40 or any(c not in "0123456789abcdefABCDEF" for c in sha):
        raise SystemExit("SOURCE_COMMIT_INVALID")
    corpus=json.loads(raw)
    if corpus.get("schema")!=SCHEMA or corpus.get("benchmark_id")!=BENCHMARK_ID or corpus.get("evidence_class")!="synthetic" or corpus.get("claim_ceiling")!=CLAIM or len(corpus.get("cases",[]))!=CASE_COUNT:
        raise SystemExit("RGB_CORPUS_IDENTITY_INVALID")
    cases=[evaluate_case(c) for c in corpus["cases"]]
    ids=[c["case_id"] for c in cases]
    if len(ids)!=len(set(ids)):
        raise SystemExit("RGB_DUPLICATE_CASE_ID")
    matches=sum(c["expected_matches"] for c in cases)
    total=sum(len(c["expected"]) for c in cases)
    receipt={"schema":RECEIPT_SCHEMA,"benchmark_id":BENCHMARK_ID,"source_commit":sha.lower(),
        "fixture_sha256":hashlib.sha256(raw).hexdigest(),"evidence_class":"synthetic","claim_ceiling":CLAIM,
        "case_count":len(cases),"expected_matches":matches,"expected_mismatches":total-matches,
        "authority_granted":False,"structural_acceptance_authorized":False,
        "physical_validation":"NOT_ASSESSED","cases":cases}
    print(json.dumps(receipt,indent=2,sort_keys=True))


if __name__=="__main__":
    main()
