#!/usr/bin/env python3
"""Independent test-only verifier for SAFRB time-expanded shared-pump fixture."""
import copy, hashlib, json, sys
from pathlib import Path

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")

def fail(condition, message):
    if not condition:
        raise AssertionError(message)

def solve(scenario, capacity_field):
    nodes={n["id"]:n for n in scenario["nodes"]}
    adjacency={n:[] for n in nodes}
    residual=[]
    original_edges=scenario["edges"]
    for edge in original_edges:
        capacity=edge[capacity_field]
        forward=len(residual); reverse=forward+1
        residual.append([edge["to"],capacity,reverse,edge["id"],True])
        residual.append([edge["from"],0,forward,edge["id"],False])
        adjacency[edge["from"]].append(forward)
        adjacency[edge["to"]].append(reverse)
    flow=0
    witness={e["id"]:0 for e in original_edges}
    while True:
        source=scenario["source_node"]; sink=scenario["sink_node"]
        parent={source:None}; queue=[source]
        for u in queue:
            for edge_index in adjacency[u]:
                v,capacity,reverse,eid,is_forward=residual[edge_index]
                if capacity>0 and v not in parent:
                    parent[v]=(u,edge_index); queue.append(v)
                    if v==sink: break
            if sink in parent: break
        if sink not in parent: break
        augment=10**30; v=sink
        while parent[v] is not None:
            u,edge_index=parent[v]; augment=min(augment,residual[edge_index][1]); v=u
        v=sink
        while parent[v] is not None:
            u,edge_index=parent[v]
            _,_,reverse,eid,is_forward=residual[edge_index]
            residual[edge_index][1]-=augment
            residual[reverse][1]+=augment
            witness[eid]+=augment if is_forward else -augment
            v=u
        flow+=augment
    reachable={scenario["source_node"]}; queue=[scenario["source_node"]]
    for u in queue:
        for edge_index in adjacency[u]:
            v,capacity,_,_,_=residual[edge_index]
            if capacity>0 and v not in reachable:
                reachable.add(v); queue.append(v)
    cut=[e["id"] for e in original_edges if e["from"] in reachable and e["to"] not in reachable]
    cut_capacity=sum(e[capacity_field] for e in original_edges if e["id"] in cut)
    return flow, sorted(reachable), sorted(cut), cut_capacity, witness

def verify(doc):
    s=doc["scenario"]; c=doc["certificate"]
    fail(s["case_id"]=="SAFRB-TWO-WINDOW-SHARED-PUMP-001","wrong case identity")
    fail(s["evidence_maturity"]=="synthetic_known_truth","non-synthetic evidence admitted")
    fail(s["claim_scope"]=="FUEL-DELIVERABILITY","scope promotion")
    fail(hashlib.sha256(canonical(s)).hexdigest()==c["scenario_sha256"],"scenario digest mismatch")
    node_ids=[n["id"] for n in s["nodes"]]
    edge_ids=[e["id"] for e in s["edges"]]
    fail(len(node_ids)==len(set(node_ids)),"duplicate node identity")
    fail(len(edge_ids)==len(set(edge_ids)),"duplicate edge identity")
    by_node={n["id"]:n for n in s["nodes"]}
    for e in s["edges"]:
        fail(e["from"] in by_node and e["to"] in by_node,"dangling edge")
        fail(type(e["capacity_lower"]) is int and type(e["capacity_upper"]) is int,"non-integer capacity")
        fail(0<=e["capacity_lower"]<=e["capacity_upper"],"invalid capacity interval")
        fail(e["unit"]==s["commodity"]["quantity_unit"],"unit mismatch")
        fail(by_node[e["from"]]["time_index"]<=by_node[e["to"]]["time_index"],"backwards time edge")
    fail(sum(e["capacity_lower"] for e in s["edges"] if e["id"] in {"supply-a","supply-b"})==s["total_opening_stock"]["quantity"],"opening stock does not reconcile")
    evidence={e["time_index"]:e for e in s["capacity_evidence"]}
    pump_edges=[e for e in s["edges"] if e["resource_id"]=="physical:pump-01"]
    fail(len(pump_edges)==2 and {e["time_index"] for e in pump_edges}=={0,1},"shared pump must have one capacity gate per interval")
    for edge in pump_edges:
        ev=evidence[edge["time_index"]]
        fail(ev["physical_resource_id"]==edge["resource_id"],"capacity evidence resource mismatch")
        fail(ev["lower"]==edge["capacity_lower"] and ev["upper"]==edge["capacity_upper"],"capacity evidence interval mismatch")
        fail(ev["status"]=="synthetic_admitted","capacity evidence not admitted for fixture")
        fail(ev["valid_interval"]==f'[{edge["time_index"]},{edge["time_index"]+1})',"capacity evidence freshness/window mismatch")
    gate_ids={e["id"] for e in pump_edges}
    aliases=s["resource_aliases"]
    fail({a["logical_route_id"] for a in aliases}=={"route-a","route-b"},"route aliases missing")
    for a in aliases:
        fail(a["physical_resource_id"]=="physical:pump-01","alias split shared physical resource")
        fail(set(a["capacity_gate_edge_ids"])==gate_ids,"route alias did not consume the same per-window pump gates")
    fail(sum(o["required_lower"] for o in s["service_obligations"])==12,"service obligation total mismatch")
    obligations={o["time_index"]:o for o in s["service_obligations"]}
    low=solve(s,"capacity_lower"); high=solve(s,"capacity_upper")
    fail(low[0]==c["lower_bound_max_flow"]==9,"lower-bound max flow mismatch")
    fail(low[3]==c["lower_bound_min_cut_capacity"]==9==low[0],"lower-bound max-flow/min-cut mismatch")
    fail(high[0]==c["upper_bound_max_flow"]==12,"upper-bound max flow mismatch")
    fail(high[3]==c["upper_bound_min_cut_capacity"]==12==high[0],"upper-bound max-flow/min-cut mismatch")
    fail(low[1]==sorted(c["source_side_nodes"]),"source-side certificate mismatch")
    fail(low[2]==sorted(c["cut_edge_ids"])==["pump-physical-t0","pump-physical-t1"],"cut-edge certificate mismatch")
    fail(low[4]==c["flow_witness"],"independently recomputed flow witness mismatch")
    for e in s["edges"]:
        used=low[4][e["id"]]
        fail(0<=used<=e["capacity_lower"],"flow exceeds conservative capacity")
    for t,obligation in obligations.items():
        delivered=low[4][f"service-obligation-t{t}"]
        fail(delivered<=obligation["required_lower"],"service flow exceeds obligation cap")
        window=c["per_window"][f"window_{t}"]
        fail(window["lower_bound_delivered"]==delivered,"per-window lower delivery mismatch")
        fail(window["required"]==obligation["required_lower"],"per-window requirement mismatch")
    fail(c["per_window"]["window_1"]["lower_bound_delivered"]==6,"window 1 is not met")
    fail(c["per_window"]["window_2"]["lower_bound_delivered"]==3,"window 2 is not met")
    fail(c["lower_bound_service_deficit"]==3,"service deficit mismatch")
    fail(c["lower_bound_verdict"]=="indeterminate_not_guaranteed","invalid final disposition")
    return low,high

def main():
    path=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).with_name("SAFRB_TWO_WINDOW_SHARED_PUMP_V0_1.json")
    doc=json.loads(path.read_text(encoding="utf-8"))
    low,high=verify(doc)
    attacks=[]
    # Duplicate the one physical gate under a second edge ID.
    a=copy.deepcopy(doc); edge=copy.deepcopy(next(e for e in a["scenario"]["edges"] if e["id"]=="pump-physical-t0")); edge["id"]="pump-physical-t0-duplicate"; a["scenario"]["edges"].append(edge); a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    # A route alias may not silently point at a second physical pump.
    a=copy.deepcopy(doc); a["scenario"]["resource_aliases"][1]["physical_resource_id"]="physical:pump-02"; a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    # Expired evidence cannot support the operation window.
    a=copy.deepcopy(doc); a["scenario"]["capacity_evidence"][1]["valid_interval"]="[0,1)"; a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    # No flow can move backwards in time to satisfy an earlier service window.
    a=copy.deepcopy(doc); e=next(e for e in a["scenario"]["edges"] if e["id"]=="pump-t1-route-a"); e["to"]="route-a-arrival-t1"; a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    # Upper capacity may not be copied into the conservative lower-bound field.
    a=copy.deepcopy(doc); next(e for e in a["scenario"]["edges"] if e["id"]=="pump-physical-t1")["capacity_lower"]=8; a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    # Duplicate source capacity under the same physical stock identity is not a second inventory lot.
    a=copy.deepcopy(doc); dup=copy.deepcopy(next(e for e in a["scenario"]["edges"] if e["id"]=="supply-a")); dup["id"]="supply-a-replay"; a["scenario"]["edges"].append(dup); a["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(a["scenario"])).hexdigest(); attacks.append(a)
    rejected=0
    for attack in attacks:
        try: verify(attack)
        except (AssertionError,KeyError,ValueError): rejected+=1
    fail(rejected==len(attacks),"a hostile mutation escaped independent verifier")
    print(f"PASS: lower flow/cut={low[0]}/{low[3]} t; upper flow/cut={high[0]}/{high[3]} t; window deliveries=6 t, 3 t; shortfall=3 t; hostile mutations rejected={rejected}")

if __name__=="__main__":
    main()
