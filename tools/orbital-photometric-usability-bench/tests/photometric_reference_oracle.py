#!/usr/bin/env python3
"""Independent standard-library oracle for synthetic BRDF/sensor dynamic-range controls."""
import hashlib
import json
import math
import sys
from pathlib import Path

PI=math.pi
SCALE=1_000_000.0
SCHEMA="luminous.orbital-photometric-usability.corpus.v0.1"
RECEIPT_SCHEMA="luminous.orbital-photometric-usability.receipt.v0.1"
BENCHMARK_ID="ORBITAL-PHOTO-USABILITY-001"
CLAIM="synthetic_declared_brdf_signal_and_sensor_dynamic_range_only"

def dot(a,b):
    return sum(x*y for x,y in zip(a,b))

def norm(a):
    return math.sqrt(dot(a,a))

def unit(a):
    return len(a)==3 and all(math.isfinite(x) for x in a) and abs(norm(a)-1.0)<=1e-6

def normalize(a):
    n=norm(a)
    if not math.isfinite(n) or n<=1e-12:
        raise ValueError("VECTOR_INVALID")
    return [x/n for x in a]

def fresnel(f0,c):
    return f0+(1.0-f0)*(1.0-min(1.0,max(0.0,c)))**5

def smith(n,a2):
    if n<=0.0:
        return 0.0
    return 2.0*n/(n+math.sqrt(a2+(1.0-a2)*n*n))

def brdf(b,n,l,v):
    nl=max(0.0,min(1.0,dot(n,l)))
    nv=max(0.0,min(1.0,dot(n,v)))
    if nl<=0.0 or nv<=0.0:
        return 0.0
    try:
        h=normalize([x+y for x,y in zip(l,v)])
    except ValueError:
        return 0.0
    nh=max(0.0,min(1.0,dot(n,h)))
    vh=max(0.0,min(1.0,dot(v,h)))
    if b["model"]=="LAMBERTIAN":
        return b["albedo"]/PI
    if b["model"]=="BLINN_PHONG":
        return b["albedo"]/PI+b["specular"]*(b["shininess"]+2.0)/(2.0*PI)*nh**b["shininess"]
    alpha=b["roughness"]**2
    a2=alpha**2
    den=nh*nh*(a2-1.0)+1.0
    d=a2/(PI*den*den)
    g=smith(nl,a2)*smith(nv,a2)
    f=fresnel(b["f0"],vh)
    return d*g*f/max(4.0*nl*nv,1.0e-12)+(1.0-b["metallic"])*(1.0-f)*b["albedo"]/PI

def quantize(x):
    if not math.isfinite(x) or x<0.0:
        raise ValueError("SIGNAL_INVALID")
    y=x*SCALE
    if not math.isfinite(y) or y>2**64-1:
        raise ValueError("SIGNAL_OVERFLOW")
    return math.floor(y+0.5)

def evaluate_case(c):
    light=c["light"]
    if not unit(light["direction"]) or not 0<=light["illumination_factor"]<=1:
        raise ValueError("LIGHT_INVALID")
    if len({s["id"] for s in c["samples"]})!=len(c["samples"]):
        raise ValueError("DUPLICATE_SAMPLE_ID")
    reports=[]
    for mode in c["modes"]:
        samples=[]
        for s in c["samples"]:
            if not light["asserted"] or not s["material_asserted"] or not mode["asserted"]:
                samples.append({"sample_id":s["id"],"geometry":s["geometry"],"signal_micro_units":None,"status":"INDETERMINATE"})
                continue
            n=normalize(s["normal"])
            l=normalize(light["direction"])
            v=normalize(s["view_direction"])
            nl=max(0.0,dot(n,l))
            signal=brdf(s["brdf"],n,l,v)*light["incident_radiance"]*light["illumination_factor"]*nl*mode["exposure_gain"]
            q=quantize(signal)
            lo=quantize(mode["noise_floor"])
            hi=quantize(mode["saturation"])
            status="UNDEREXPOSED" if q<lo else "SATURATED" if q>hi else "USABLE"
            samples.append({"sample_id":s["id"],"geometry":s["geometry"],"signal_micro_units":q,"status":status})
        count=len(samples)
        usable=sum(s["status"]=="USABLE" for s in samples)
        under=sum(s["status"]=="UNDEREXPOSED" for s in samples)
        sat=sum(s["status"]=="SATURATED" for s in samples)
        ind=sum(s["status"]=="INDETERMINATE" for s in samples)
        gp=sum(s["geometry"]=="PASS" for s in samples)
        gf=sum(s["geometry"]=="FAIL" for s in samples)
        gi=sum(s["geometry"]=="INDETERMINATE" for s in samples)
        summary="INDETERMINATE" if ind else "FULLY_USABLE" if usable==count else "PARTIAL" if usable else "NOT_USABLE"
        acquisition="GEOMETRY_REJECTED" if gf else "INDETERMINATE" if gi or summary=="INDETERMINATE" else {
            "FULLY_USABLE":"USABLE","PARTIAL":"PARTIAL","NOT_USABLE":"UNUSABLE","INDETERMINATE":"INDETERMINATE"
        }[summary]
        reports.append({
            "mode_id":mode["id"],"sample_count":count,"usable_count":usable,
            "underexposed_count":under,"saturated_count":sat,"indeterminate_count":ind,
            "geometry_pass_count":gp,"geometry_fail_count":gf,"geometry_indeterminate_count":gi,
            "summary":summary,"acquisition":acquisition,"samples":samples
        })
    byid={m["mode_id"]:m for m in reports}
    matches=sum(byid[e["mode_id"]]["summary"]==e["summary"] and byid[e["mode_id"]]["acquisition"]==e["acquisition"] for e in c["expected"])
    return {"case_id":c["case_id"],"expected":c["expected"],"expected_matches":matches,
        "expected_mismatches":len(c["expected"])-matches,"modes":reports}

def main():
    if len(sys.argv)!=3:
        raise SystemExit("usage: photometric_reference_oracle.py <corpus.json> <40-hex-source-commit>")
    raw=Path(sys.argv[1]).read_bytes()
    sha=sys.argv[2]
    if len(sha)!=40 or any(x not in "0123456789abcdefABCDEF" for x in sha):
        raise SystemExit("SOURCE_COMMIT_INVALID")
    corpus=json.loads(raw)
    if corpus.get("schema")!=SCHEMA or corpus.get("benchmark_id")!=BENCHMARK_ID or corpus.get("evidence_class")!="synthetic" or corpus.get("claim_ceiling")!=CLAIM or len(corpus.get("cases",[]))!=8:
        raise SystemExit("CORPUS_IDENTITY_INVALID")
    cases=[evaluate_case(c) for c in corpus["cases"]]
    ids=[c["case_id"] for c in cases]
    if len(ids)!=len(set(ids)):
        raise SystemExit("DUPLICATE_CASE_ID")
    matches=sum(c["expected_matches"] for c in cases)
    total=sum(len(c["expected"]) for c in cases)
    result={"schema":RECEIPT_SCHEMA,"benchmark_id":BENCHMARK_ID,"source_commit":sha.lower(),
        "fixture_sha256":hashlib.sha256(raw).hexdigest(),"evidence_class":"synthetic","claim_ceiling":CLAIM,
        "case_count":len(cases),"expected_matches":matches,"expected_mismatches":total-matches,
        "authority_granted":False,"structural_acceptance_authorized":False,
        "physical_validation":"NOT_ASSESSED","cases":cases}
    print(json.dumps(result,indent=2,sort_keys=True))

if __name__=="__main__":
    main()
