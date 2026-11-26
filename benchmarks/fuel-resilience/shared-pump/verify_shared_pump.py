#!/usr/bin/env python3
"""Independent standard-library verifier for the SAFRB shared-pump fixture."""
import copy, hashlib, json, sys
from pathlib import Path

def canonical(v):
    return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()

def max_flow(s, capacity_key):
    edges=s["edges"]; adj={n["id"]:[] for n in s["nodes"]}; cap={}
    for e in edges:
        u,v=e["from"],e["to"]; c=e[capacity_key]
        if u not in adj or v not in adj or c < 0: raise ValueError("bad edge")
        cap[(u,v)]=cap.get((u,v),0)+c
        adj[u].append(v); adj[v].append(u)
    flow=0; residual=dict(cap); source=s["source_node"]; sink=s["sink_node"]
    while True:
        parent={source:None}; queue=[source]
        for u in queue:
            for v in adj[u]:
                if v not in parent and residual.get((u,v),0)>0:
                    parent[v]=u; queue.append(v)
                    if v==sink: break
            if sink in parent: break
        if sink not in parent: break
        aug=10**18; v=sink
        while parent[v] is not None:
            u=parent[v]; aug=min(aug,residual[(u,v)]); v=u
        v=sink
        while parent[v] is not None:
            u=parent[v]; residual[(u,v)]-=aug; residual[(v,u)]=residual.get((v,u),0)+aug; v=u
        flow+=aug
    reachable={source}; queue=[source]
    for u in queue:
        for v in adj[u]:
            if v not in reachable and residual.get((u,v),0)>0: reachable.add(v); queue.append(v)
    cut=[e["id"] for e in edges if e["from"] in reachable and e["to"] not in reachable]
    cut_capacity=sum(e[capacity_key] for e in edges if e["id"] in cut)
    return flow,reachable,cut,cut_capacity

def verify(doc):
    s=doc["scenario"]; c=doc["certificate"]
    assert s["case_id"]=="SAFRB-SHARED-PUMP-2ROUTE-001"
    assert s["evidence_maturity"]=="synthetic_known_truth"
    assert s["claim_scope"]=="FUEL-DELIVERABILITY"
    assert hashlib.sha256(canonical(s)).hexdigest()==c["scenario_sha256"], "scenario digest mismatch"
    node_ids=[n["id"] for n in s["nodes"]]; edge_ids=[e["id"] for e in s["edges"]]
    assert len(node_ids)==len(set(node_ids)) and len(edge_ids)==len(set(edge_ids)), "duplicate identity"
    pump=[e for e in s["edges"] if e["resource_id"]=="physical:pump-01"]
    assert len(pump)==1 and pump[0]["id"]=="pump-01-gate", "shared pump must be one capacity gate"
    aliases=s["resource_aliases"]
    assert len(aliases)==2 and {a["physical_resource_id"] for a in aliases}=={"physical:pump-01"}, "route aliases split shared resource"
    lower,reachable,cut,cutcap=max_flow(s,"capacity_lower")
    upper,_,_,_=max_flow(s,"capacity_upper")
    assert lower==c["lower_bound_max_flow"]==8
    assert upper==c["upper_bound_max_flow"]==12
    assert set(c["source_side_nodes"])==reachable, "residual source-side mismatch"
    assert set(c["cut_edge_ids"])==set(cut)=={"pump-01-gate"}, "cut identity mismatch"
    assert cutcap==c["lower_bound_min_cut_capacity"]==lower, "max-flow/min-cut mismatch"
    witness=c["flow_witness"]; balance={n:0 for n in node_ids}
    by_id={e["id"]:e for e in s["edges"]}
    assert set(witness)==set(by_id), "incomplete witness"
    for eid,amount in witness.items():
        e=by_id[eid]; assert isinstance(amount,int) and 0<=amount<=e["capacity_lower"], "witness exceeds lower bound"
        balance[e["from"]]-=amount; balance[e["to"]]+=amount
    for n,b in balance.items():
        expected=-lower if n==s["source_node"] else lower if n==s["sink_node"] else 0
        assert b==expected, f"flow conservation failed at {n}"
    assert s["required_delivery"]-lower==c["lower_bound_service_deficit"]==4
    assert c["lower_bound_verdict"]=="indeterminate"
    return lower,upper,cutcap

def main():
    path=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).with_name("SAFRB_SHARED_PUMP_KNOWN_ANSWER_V0_1.json")
    doc=json.loads(path.read_text(encoding="utf-8"))
    lower,upper,cut=verify(doc)
    attacks=[]
    bad=copy.deepcopy(doc); bad["scenario"]["edges"][4]["capacity_lower"]=9; attacks.append(bad)
    bad=copy.deepcopy(doc); bad["scenario"]["resource_aliases"][1]["physical_resource_id"]="physical:pump-02"; bad["certificate"]["scenario_sha256"]=hashlib.sha256(canonical(bad["scenario"])).hexdigest(); attacks.append(bad)
    bad=copy.deepcopy(doc); bad["certificate"]["flow_witness"]["pump-01-gate"]=9; attacks.append(bad)
    bad=copy.deepcopy(doc); bad["certificate"]["cut_edge_ids"]=[]; attacks.append(bad)
    rejected=0
    for attack in attacks:
        try: verify(attack)
        except (AssertionError,KeyError,ValueError): rejected+=1
    assert rejected==len(attacks), "hostile mutation escaped verifier"
    print(f"PASS: lower max-flow/min-cut={lower} t; upper max-flow={upper} t; shortfall=4 t; hostile mutations rejected={rejected}")

if __name__=="__main__":
    main()
