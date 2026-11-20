#!/usr/bin/env python3
"""Test-only fixture audit; does not load or execute any FMU."""
import hashlib, json, math, struct, sys
from pathlib import Path
from jsonschema import Draft202012Validator, FormatChecker

ROOT=Path(__file__).resolve().parents[1]
def fail(msg): raise AssertionError(msg)
def load(path):
    try: return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc: fail(f"cannot load {path}: {exc}")
def bits(value): return struct.pack(">d",float(value)).hex()
def unbits(value): return struct.unpack(">d",bytes.fromhex(value))[0]

def audit_artifacts(ledger):
    root=ROOT.resolve(strict=True); result={}; paths=set()
    for a in ledger["artifacts"]:
        aid=a["artifact_id"]; rel=Path(a["path"])
        if aid in result: fail(f"duplicate artifact id: {aid}")
        if rel.is_absolute() or ".." in rel.parts: fail(f"unsafe artifact path: {rel}")
        p=ROOT/rel
        if any(x.is_symlink() for x in [p,*p.parents] if x!=ROOT.parent): fail(f"symlink in artifact path: {rel}")
        p=p.resolve(strict=True)
        if not p.is_relative_to(root): fail(f"artifact escapes root: {rel}")
        if str(rel) in paths: fail(f"duplicate artifact path: {rel}")
        paths.add(str(rel)); raw=p.read_bytes()
        if len(raw)!=a["byte_length"] or hashlib.sha256(raw).hexdigest()!=a["sha256"]: fail(f"artifact integrity mismatch: {aid}")
        if ledger["evidence_class"]=="synthetic_fixture" and (a["executable"] or not a["synthetic_stub"]): fail(f"synthetic artifact is not a non-executable stub: {aid}")
        result[aid]=a
    return result

def audit_fixture(d, arts):
    feds={f["federate_id"]:f for f in d["federates"]}
    if len(feds)!=len(d["federates"]): fail("duplicate federate id")
    vars={}
    for fid,f in feds.items():
        rolemap={"package":"fmu_package","model_description":"model_description","platform_binary":"platform_binary"}
        for role,aid in f["artifacts"].items():
            if aid not in arts or arts[aid]["role"]!=rolemap[role]: fail(f"artifact role mismatch: {fid}/{role}")
        caps=f["capabilities"]
        if caps["can_serialize_fmu_state"] and not caps["can_get_and_set_fmu_state"]: fail(f"invalid state capability implication: {fid}")
        seen=set()
        for v in f["variables"]:
            vr=v["value_reference"]
            if vr in seen: fail(f"duplicate value reference {fid}/{vr}")
            if v["port_role"] not in ("coupling_source", "coupling_sink", "terminal_sink", "state_observation"):
                fail(f"unsupported port role: {fid}/{vr}")
            seen.add(vr); vars[(fid,vr)]=v
    life={fid:{"created":False,"entered":False,"initialized":False,"terminated":False,"freed":False} for fid in feds}
    steps={}; successful=set(); values=[]; event_ids=set()
    for seq,e in enumerate(d["events"]):
        fid=e["federate_id"]; k=e["kind"]; p=e["payload"]; eid=e["event_id"]; state=life.get(fid)
        if e["sequence"]!=seq or eid in event_ids or state is None: fail(f"event order/id/federate error at {eid}")
        event_ids.add(eid)
        if e["status"]!="ok": fail(f"positive fixture contains non-ok status: {eid}")
        expected={"instance_created":"fmi3InstantiateCoSimulation","initialization_mode":None,"set_float64":"fmi3SetFloat64","get_float64":"fmi3GetFloat64","do_step":"fmi3DoStep","terminate":"fmi3Terminate","free_instance":"fmi3FreeInstance"}
        if expected[k] and e["call_symbol"]!=expected[k]: fail(f"call-symbol mismatch at {eid}")
        if k=="instance_created":
            if state["created"] or p.get("interface_type")!="CoSimulation" or not p.get("instance_name") or p.get("visible") is not False or p.get("logging_on") is not False: fail(f"bad instantiate record: {eid}")
            if not isinstance(p.get("required_intermediate_variables"),list): fail(f"missing required intermediate variables: {eid}")
            state["created"]=True
        elif k=="initialization_mode":
            if not state["created"] or state["terminated"]: fail(f"initialization outside lifetime: {eid}")
            if p.get("transition")=="enter":
                if state["entered"]: fail(f"duplicate init enter: {fid}")
                start=unbits(p["start_time_bits"]); stop=unbits(p["stop_time_bits"])
                if not(math.isfinite(start) and math.isfinite(stop) and start<stop): fail(f"invalid initialization range: {eid}")
                if p.get("tolerance_defined") is not False or p.get("tolerance_bits") is not None or p.get("stop_time_defined") is not True: fail(f"invalid tolerance/stop-time arguments: {eid}")
                state["entered"]=True
            elif p.get("transition")=="exit":
                if not state["entered"] or state["initialized"]: fail(f"bad init exit: {fid}")
                state["initialized"]=True
            else: fail(f"unknown initialization transition: {eid}")
        elif k in ("set_float64","get_float64"):
            if not state["initialized"] or state["terminated"]: fail(f"I/O outside initialized state: {eid}")
            if k=="get_float64" and fid not in successful: fail(f"output read before successful doStep: {eid}")
            if k=="set_float64" and fid in successful: fail(f"input written after doStep: {eid}")
            iv=p["interval"]
            if iv["start_master_ns"]>=iv["end_master_ns"]: fail(f"bad interval: {eid}")
            vals=p["values"]; refs=[x["value_reference"] for x in vals]
            if refs!=p["value_references"] or p["n_value_references"]!=len(refs): fail(f"VR list/count mismatch: {eid}")
            if not vals: fail(f"empty value call: {eid}")
            for x in vals:
                v=vars.get((fid,x["value_reference"]))
                if v is None: fail(f"undeclared value reference: {eid}")
                if v["causality"]!=("input" if k=="set_float64" else "output"): fail(f"wrong variable causality: {eid}")
                if v["port_id"]!=x["port_id"] or x["source_interval_id"]!=iv["source_interval_id"]: fail(f"port or interval mismatch: {eid}")
                value_semantics=x.get("sampling_semantics",iv["sampling_semantics"])
                if value_semantics not in ("constant_over_declared_half_open_interval","sample_at_interval_end","sample_at_interval_start","interval_average"):
                    fail(f"invalid per-value sampling semantics: {eid}")
                if v["port_role"]=="state_observation" and (
                    k!="get_float64" or value_semantics!="sample_at_interval_end" or "transfer_id" in x
                ):
                    fail(f"state observation must be a non-transfer output sampled at interval end: {eid}")
                y=float(x["value"])
                if not math.isfinite(y) or bits(y)!=x["raw_f64_bits"] or not math.isfinite(unbits(x["raw_f64_bits"])): fail(f"Float64 exact-bit mismatch: {eid}")
                values.append((k,fid,v,x,iv))
        elif k=="do_step":
            if not state["initialized"] or state["terminated"]: fail(f"doStep outside initialized state: {eid}")
            aid=p["attempt_id"]
            if aid in steps: fail(f"duplicate attempt: {aid}")
            cp=unbits(p["current_communication_point_bits"]); dt=unbits(p["communication_step_size_bits"]); last=unbits(p["last_successful_time_bits"])
            if not all(math.isfinite(x) for x in (cp,dt,last)) or dt<=0 or not(cp<last<=cp+dt): fail(f"invalid successful-step times: {eid}")
            if p["early_return"] or p["event_handling_needed"] or p["terminate_simulation"]: fail(f"unsupported step outcome in positive fixture: {eid}")
            if p.get("no_set_fmu_state_prior_to_current_point") is not True: fail(f"missing doStep precondition: {eid}")
            duration_ns=(last-cp)*1_000_000_000
            if duration_ns!=round(duration_ns) or p["master_end_ns"]-p["master_start_ns"]!=round(duration_ns): fail(f"master and FMI times differ: {eid}")
            steps[aid]={"federate_id":fid,"start_master_ns":p["master_start_ns"],"end_master_ns":p["master_end_ns"],"status":"accepted"}
            successful.add(fid)
        elif k=="terminate":
            if not state["initialized"] or state["terminated"]: fail(f"bad terminate: {fid}")
            state["terminated"]=True
        elif k=="free_instance":
            if not state["terminated"] or state["freed"]: fail(f"free without terminate: {fid}")
            state["freed"]=True
        elif k in ("state_snapshot","state_restore"):
            caps=feds[fid]["capabilities"]
            if not(caps["can_get_and_set_fmu_state"] and caps["can_serialize_fmu_state"]): fail(f"state API used without capability: {eid}")
            fail("positive fixture must not exercise state APIs")
    for fid,s in life.items():
        if not all(s.values()): fail(f"incomplete FMI lifecycle for {fid}: {s}")
    if set(steps)!={"geo-step-0","hx-step-0"}: fail("positive fixture has unexpected doStep attempts")
    # One source getFloat64 and one receiver setFloat64, byte-for-byte identical.
    reads={}; writes={}
    for kind,fid,var,x,iv in values:
        tid=x.get("transfer_id")
        if tid:
            key=(x["source_interval_id"],x["raw_f64_bits"],iv["start_master_ns"],iv["end_master_ns"])
            target=reads if kind=="get_float64" else writes
            if tid in target: fail(f"duplicate transfer leg: {tid}")
            target[tid]=(key,fid,var,iv)
    state_observations=[record for record in values if record[2]["port_role"]=="state_observation"]
    if len(state_observations)!=1: fail("expected exactly one declared state observation")
    state_kind,state_fid,state_var,state_value,state_interval=state_observations[0]
    if (state_kind,state_fid,state_var["port_id"],state_var["unit"],state_value["value"],
        state_value.get("sampling_semantics"),state_interval["end_master_ns"]) != (
        "get_float64","heat-exchanger-buffer","hx.stored_energy","J",10000,
        "sample_at_interval_end",1000000000
    ):
        fail("stored-energy state observation identity/value/time mismatch")
    if state_value["raw_f64_bits"]!=bits(10000.0):
        fail("stored-energy state observation exact bits mismatch")
    if set(reads)!={"thermal-transfer-0"} or set(writes)!={"thermal-transfer-0"}: fail("missing or duplicate transfer leg")
    rk,rf,rv,ri=reads["thermal-transfer-0"]; wk,wf,wv,wi=writes["thermal-transfer-0"]
    if rk!=wk or rv["port_role"]!="coupling_source" or wv["port_role"]!="coupling_sink": fail("source/receiver transfer identity mismatch")
    if (rv["carrier"],rv["unit"],rv["boundary_id"])!=(wv["carrier"],wv["unit"],wv["boundary_id"]): fail("transfer carrier/unit/boundary mismatch")
    # Match exact normalized trace attempts and common commit barrier.
    trace=json.loads((ROOT/arts[d["normalized_trace_artifact_id"]]["path"]).read_text())
    tm={x["attempt_id"]:x for x in trace["attempts"]}
    if set(tm)!=set(steps): fail("trace and doStep attempt sets differ")
    for aid,s in steps.items():
        t=tm[aid]
        if (s["federate_id"],s["start_master_ns"],s["end_master_ns"],s["status"])!=(t["federate_id"],t["start_master_ns"],t["end_master_ns"],t["status"]): fail(f"trace disagreement: {aid}")
    used=set()
    for b in d["commit_barriers"]:
        if len(b["federate_ids"])!=len(b["attempt_ids"]): fail("barrier participant count mismatch")
        parts=[]
        for aid in b["attempt_ids"]:
            if aid not in steps or aid in used: fail(f"bad barrier attempt: {aid}")
            used.add(aid); parts.append(steps[aid])
        if {x["federate_id"] for x in parts}!=set(b["federate_ids"]) or any(x["end_master_ns"]!=b["master_time_ns"] for x in parts): fail("barrier membership/time mismatch")
    if used!=set(steps): fail("unbarriered doStep attempt")
    energy=json.loads((ROOT/arts[d["energy_contract_artifact_id"]]["path"]).read_text())
    if energy["interval_id"]!="thermal-interval-0": fail("energy interval mismatch")
    got={x[3]["port_id"]:x[3]["value"] for x in values}
    for port,value in energy["expected"].items():
        if got.get(port)!=value: fail(f"energy/API ledger mismatch: {port}")
    residual=energy["expected"]["hx.thermal_in"]-energy["expected"]["hx.thermal_service"]-energy["expected"]["hx.heat_loss"]-energy["expected"]["hx.storage_charge"]
    if residual!=energy["residual_w"] or residual!=0.0: fail("synthetic energy residual does not recompute")
    st=d["state_checkpointing"]
    if st["status"]!="unavailable" or st["snapshot_artifact_ids"]: fail("fixture incorrectly implies state snapshot evidence")
    if any(f["capabilities"]["can_serialize_fmu_state"] and not f["capabilities"]["can_get_and_set_fmu_state"] for f in feds.values()): fail("canSerializeFMUState requires canGetAndSetFMUState")
    if any(f["capabilities"]["can_get_and_set_fmu_state"] and f["capabilities"]["can_serialize_fmu_state"] for f in feds.values()): fail("serialized state snapshot is unavailable despite both required capabilities")
    if st["reason_code"] != "SERIALIZATION_CAPABILITY_NOT_ADVERTISED": fail("incorrect state serialization unavailability reason")

def audit_cases():
    lines=(ROOT/"fixtures/frozen-cases.tsv").read_text().splitlines()
    if lines[0]!="case_id\texpected_disposition\tdescription": fail("bad TSV header")
    ids=set()
    for n,line in enumerate(lines[1:],2):
        c=line.split("\t")
        if len(c)!=3 or not all(c): fail(f"bad TSV row {n}")
        if c[0] in ids or c[1] not in ("REJECT","NOT_EVALUABLE"): fail(f"duplicate/invalid TSV row {n}")
        ids.add(c[0])
    if len(ids)!=25: fail(f"expected 25 frozen negative-control rows, got {len(ids)}")
    return len(ids)

def main():
    schema=load(ROOT/"run-ledger.schema.json"); d=load(ROOT/"fixtures/positive-two-fmu-ledger.json")
    Draft202012Validator.check_schema(schema)
    errors=list(Draft202012Validator(schema,format_checker=FormatChecker()).iter_errors(d))
    if errors:
        for e in errors: print(f"SCHEMA {list(e.path)}: {e.message}",file=sys.stderr)
        return 1
    arts=audit_artifacts(d); audit_fixture(d,arts); n=audit_cases()
    print("GeoFusion FMI API ledger fixture audit: PASS")
    print(f"artifacts={len(arts)} (all non-executable stubs); federates={len(d['federates'])}; API events={len(d['events'])}")
    print("doStep attempts=2; exchanged values=5; barriers=1; normalized trace links=2; synthetic residual=0 W")
    print(f"negative controls inventoried={n} (mutation execution pending)")
    print("FMI runtime/FMUs/solver comparisons: NOT RUN")
    return 0
if __name__=="__main__": raise SystemExit(main())
