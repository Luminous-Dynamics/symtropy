#!/usr/bin/env python3
"""Independent standard-library oracle for acquisition co-design v0.1."""
import hashlib, itertools, json, sys
from pathlib import Path

SCHEMA="luminous.orbital-acquisition-codesign.corpus.v0.1"
RECEIPT="luminous.orbital-acquisition-codesign.receipt.v0.1"
BENCHMARK="ORBITAL-ACQUISITION-CODESIGN-001"
CLAIM="synthetic_exact_finite_schedule_feasibility_and_pareto_frontier_only"
MAX=(1<<64)-1
HEX=set("0123456789abcdefABCDEF")

def text_ok(v): return isinstance(v,str) and bool(v.strip())
def digest_ok(v): return isinstance(v,str) and len(v)==64 and all(c in HEX for c in v)
def add(a,b,code):
    x=a+b
    if x>MAX: raise ValueError(code)
    return x
def mul(a,b,code):
    x=a*b
    if x>MAX: raise ValueError(code)
    return x
def count_schedules(c):
    n=1
    for s in c["segments"]: n=mul(n,len(s["speed_choices"]),"SCHEDULE_COUNT_OVERFLOW")
    for e in c["events"]:
        if e["required"]: n=mul(n,len(e["options"]),"SCHEDULE_COUNT_OVERFLOW")
    return n
def validate(c):
    if not text_ok(c["case_id"]) or not c["segments"] or not c["events"] or c["max_schedules"]<=0:
        raise ValueError("CASE_ID_OR_REQUIRED_COLLECTION_INVALID")
    speed_sets={}
    for s in c["segments"]:
        sid=s["segment_id"]
        if not text_ok(sid) or not text_ok(s["path_revision"]) or not digest_ok(s["path_feasibility_receipt_sha256"]) or not s["speed_choices"] or sid in speed_sets:
            raise ValueError("PATH_SEGMENT_INVALID")
        ids=set()
        for v in s["speed_choices"]:
            if not text_ok(v["speed_id"]) or not digest_ok(v["motion_profile_receipt_sha256"]) or not (0<v["duration_ms"]<=MAX) or not (0<=v["motion_energy_mj"]<=MAX) or v["speed_id"] in ids:
                raise ValueError("SPEED_CHOICE_INVALID")
            ids.add(v["speed_id"])
        speed_sets[sid]=ids
    event_ids=set()
    for e in c["events"]:
        if not text_ok(e["event_id"]) or not text_ok(e["characteristic_id"]) or e["event_id"] in event_ids or (e["required"] and not e["options"]):
            raise ValueError("ACQUISITION_EVENT_INVALID")
        event_ids.add(e["event_id"])
        option_ids=set()
        for o in e["options"]:
            if (not text_ok(o["option_id"]) or o["option_id"] in option_ids
                or not text_ok(o["segment_id"]) or o["segment_id"] not in speed_sets
                or not text_ok(o["speed_id"]) or o["speed_id"] not in speed_sets[o["segment_id"]]
                or not text_ok(o["illumination_mode_id"]) or not o["gates"]):
                raise ValueError("ACQUISITION_OPTION_INVALID")
            option_ids.add(o["option_id"])
            gate_ids=set()
            for g in o["gates"]:
                if not text_ok(g["gate_id"]) or g["gate_id"] in gate_ids or not digest_ok(g["receipt_sha256"]):
                    raise ValueError("ACQUISITION_GATE_INVALID")
                gate_ids.add(g["gate_id"])
            for key in ("exposure_ns","dwell_ms","illumination_energy_mj","projector_energy_mj","retakes"):
                if not 0<=o[key]<=MAX: raise ValueError("ACQUISITION_OPTION_RANGE_INVALID")
    transitions=set()
    for t in c["allowed_transitions"]:
        if (not text_ok(t["from_segment_id"]) or t["from_segment_id"] not in speed_sets
            or not text_ok(t["to_segment_id"]) or t["to_segment_id"] not in speed_sets
            or t["from_speed_id"] not in speed_sets[t["from_segment_id"]]
            or t["to_speed_id"] not in speed_sets[t["to_segment_id"]]
            or not digest_ok(t["transition_receipt_sha256"])):
            raise ValueError("TRANSITION_INVALID")
        key=(t["from_segment_id"],t["from_speed_id"],t["to_segment_id"],t["to_speed_id"])
        if key in transitions: raise ValueError("TRANSITION_DUPLICATE")
        transitions.add(key)
    b=c["budget"]
    for k in ("robot_energy_capacity_mj","required_return_reserve_mj","illumination_energy_budget_mj","projector_energy_budget_mj"):
        if not 0<=b[k]<=MAX: raise ValueError("BUDGET_RANGE_INVALID")
    if b["required_return_reserve_mj"]>b["robot_energy_capacity_mj"]:
        raise ValueError("RETURN_RESERVE_EXCEEDS_CAPACITY")

def make_id(speeds,options):
    return "S["+";".join(f"{k}={v}" for k,v in sorted(speeds.items()))+"]O["+";".join(f"{k}={v}" for k,v in sorted(options.items()))+"]"
def dominates(a,b):
    keys=("elapsed_ms","robot_motion_energy_mj","illumination_energy_mj","projector_energy_mj","retakes")
    av=[a[k] for k in keys]; bv=[b[k] for k in keys]
    return all(x<=y for x,y in zip(av,bv)) and any(x<y for x,y in zip(av,bv))

def eval_combo(c,speeds,options):
    reason=set(); hard=False; unknown=False; evidence=set()
    obj={"elapsed_ms":0,"robot_motion_energy_mj":0,"illumination_energy_mj":0,"projector_energy_mj":0,"retakes":0}
    for seg in c["segments"]:
        v=next(x for x in seg["speed_choices"] if x["speed_id"]==speeds[seg["segment_id"]])
        evidence.add(seg["path_feasibility_receipt_sha256"]); evidence.add(v["motion_profile_receipt_sha256"])
        obj["elapsed_ms"]=add(obj["elapsed_ms"],v["duration_ms"],"ELAPSED_OVERFLOW")
        obj["robot_motion_energy_mj"]=add(obj["robot_motion_energy_mj"],v["motion_energy_mj"],"ROBOT_ENERGY_OVERFLOW")
    events={e["event_id"]:e for e in c["events"]}
    for eid,oid in options.items():
        o=next(x for x in events[eid]["options"] if x["option_id"]==oid)
        if speeds.get(o["segment_id"])!=o["speed_id"]:
            hard=True; reason.add("OPTION_SPEED_DOES_NOT_MATCH_SEGMENT")
        for g in o["gates"]:
            evidence.add(g["receipt_sha256"])
            if g["state"]=="FAIL": hard=True; reason.add("GATE_FAILED:"+g["gate_id"])
            elif g["state"]=="UNKNOWN": unknown=True; reason.add("GATE_UNKNOWN:"+g["gate_id"])
        obj["elapsed_ms"]=add(obj["elapsed_ms"],o["dwell_ms"],"ELAPSED_OVERFLOW")
        obj["illumination_energy_mj"]=add(obj["illumination_energy_mj"],o["illumination_energy_mj"],"ILLUMINATION_ENERGY_OVERFLOW")
        obj["projector_energy_mj"]=add(obj["projector_energy_mj"],o["projector_energy_mj"],"PROJECTOR_ENERGY_OVERFLOW")
        obj["retakes"]=add(obj["retakes"],o["retakes"],"RETAKE_OVERFLOW")
    for a,b in zip(c["segments"],c["segments"][1:]):
        match=next((t for t in c["allowed_transitions"] if t["from_segment_id"]==a["segment_id"] and t["from_speed_id"]==speeds[a["segment_id"]] and t["to_segment_id"]==b["segment_id"] and t["to_speed_id"]==speeds[b["segment_id"]]),None)
        if match is None: hard=True; reason.add("PATH_SPEED_TRANSITION_NOT_ADMITTED")
        else: evidence.add(match["transition_receipt_sha256"])
    budget=c["budget"]
    if obj["robot_motion_energy_mj"]>budget["robot_energy_capacity_mj"]-budget["required_return_reserve_mj"]:
        hard=True; reason.add("ROBOT_ENERGY_OR_RETURN_RESERVE_EXCEEDED")
    if obj["illumination_energy_mj"]>budget["illumination_energy_budget_mj"]:
        hard=True; reason.add("ILLUMINATION_ENERGY_BUDGET_EXCEEDED")
    if obj["projector_energy_mj"]>budget["projector_energy_budget_mj"]:
        hard=True; reason.add("PROJECTOR_ENERGY_BUDGET_EXCEEDED")
    sid=make_id(speeds,options)
    if hard: return {"disposition":"INFEASIBLE","schedule_id":sid,"reason_codes":sorted(reason),"schedule":None}
    if unknown: return {"disposition":"INDETERMINATE","schedule_id":sid,"reason_codes":sorted(reason),"schedule":None}
    return {"disposition":"FEASIBLE","schedule_id":sid,"reason_codes":[],"schedule":{
        "schedule_id":sid,"selected_speed_ids":dict(sorted(speeds.items())),
        "selected_option_ids":dict(sorted(options.items())),"evidence_receipt_sha256":sorted(evidence),"objective":obj}}

def evaluate_case(c):
    validate(c); n=count_schedules(c)
    if n>c["max_schedules"]:
        return {"case_id":c["case_id"],"status":"SEARCH_LIMIT_NOT_PROVEN","theoretical_schedule_count":n,"evaluated_schedule_count":0,"feasible_schedule_count":0,"indeterminate_schedule_count":0,"infeasible_schedule_count":0,"pareto_frontier":[],"dispositions":[],"exactness_proven":False}
    dims=[("segment",s["segment_id"],[v["speed_id"] for v in s["speed_choices"]]) for s in c["segments"]]
    dims += [("event",e["event_id"],[o["option_id"] for o in e["options"]]) for e in c["events"] if e["required"]]
    feasible=[]; dispositions=[]; fc=ic=xc=0
    for choices in itertools.product(*(d[2] for d in dims)):
        speeds={}; options={}
        for d,v in zip(dims,choices):
            (speeds if d[0]=="segment" else options)[d[1]]=v
        result=eval_combo(c,speeds,options)
        dispositions.append({"schedule_id":result["schedule_id"],"disposition":result["disposition"],"reason_codes":result["reason_codes"]})
        if result["disposition"]=="FEASIBLE": fc+=1; feasible.append(result["schedule"])
        elif result["disposition"]=="INDETERMINATE": ic+=1
        else: xc+=1
    feasible.sort(key=lambda x:x["schedule_id"])
    unique=[]
    for item in feasible:
        if not any(x["objective"]==item["objective"] for x in unique): unique.append(item)
    frontier=[x for x in unique if not any(y["schedule_id"]!=x["schedule_id"] and dominates(y["objective"],x["objective"]) for y in unique)]
    frontier.sort(key=lambda x:x["schedule_id"])
    if ic: status="INDETERMINATE"; exact=False
    elif frontier: status="OPTIMAL_WITHIN_DECLARED_FINITE_UNIVERSE"; exact=True
    else: status="INFEASIBLE"; exact=True
    return {"case_id":c["case_id"],"status":status,"theoretical_schedule_count":n,"evaluated_schedule_count":n,"feasible_schedule_count":fc,"indeterminate_schedule_count":ic,"infeasible_schedule_count":xc,"pareto_frontier":frontier,"dispositions":dispositions,"exactness_proven":exact}

def main():
    if len(sys.argv)!=3: raise SystemExit("usage: reference_oracle.py <fixture.json> <40-hex-source-commit>")
    raw=Path(sys.argv[1]).read_bytes(); sha=sys.argv[2]
    if len(sha)!=40 or any(c not in HEX for c in sha): raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    c=json.loads(raw)
    if c.get("schema")!=SCHEMA or c.get("benchmark_id")!=BENCHMARK or c.get("evidence_class")!="synthetic" or c.get("claim_ceiling")!=CLAIM or not c.get("cases") or len(c["cases"])>256:
        raise SystemExit("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    ids=[x["case_id"] for x in c["cases"]]
    if len(ids)!=len(set(ids)): raise SystemExit("CASE_ID_DUPLICATE")
    results=[evaluate_case(x) for x in c["cases"]]
    matches=sum(r["status"]==src["expected_status"] and [s["schedule_id"] for s in r["pareto_frontier"]]==src["expected_frontier_schedule_ids"] for r,src in zip(results,c["cases"]))
    receipt={"schema":RECEIPT,"benchmark_id":BENCHMARK,"source_commit":sha.lower(),"input_sha256":hashlib.sha256(raw).hexdigest(),"evidence_class":"synthetic","claim_ceiling":CLAIM,"case_count":len(results),"expected_outcome_matches":matches,"expected_outcome_mismatches":len(results)-matches,"authority_granted":False,"physical_validation":"NOT_ASSESSED","cases":results}
    print(json.dumps(receipt,indent=2)+"\n")
if __name__=="__main__": main()
