#!/usr/bin/env python3
"""Deterministic fail-closed revalidation planning for PKG v0."""
from __future__ import annotations
import argparse
import json
from typing import Any
from impact import IMPACT_VERSION, analyze
from validate import SCHEMA_VERSION, Validator
PLAN_VERSION="luminous.formal-revalidation-plan.v0"
PROOF_EXECUTION_FIELDS=("artifact_id","checker_id","source_commit_id","source_tree_id","source_commit_sha","source_tree_sha")

def _validate_graph(graph):
    result=Validator(graph).validate()
    if result["status"]!="Valid": raise ValueError("input provenance graph is not semantically valid: "+json.dumps(result,sort_keys=True,separators=(",",":")))

def _history_invalidations(graph):
    superseded=set(); invalidated=set()
    for edge in graph["edges"]:
        if edge["relation"]=="supersedes": superseded.add(edge["target"])
        elif edge["relation"]=="invalidated_by": invalidated.add(edge["source"])
    return superseded,invalidated

def _reference_bindings(node,nodes):
    metadata=node.get("metadata")
    if not isinstance(metadata,dict): return [],[]
    expected={"artifact_id":"ProofArtifact","checker_id":"ProofChecker","source_commit_id":"SourceCommit","source_tree_id":"SourceTree","execution_id":"ProofExecution","contract_id":"QualificationContract","verifier_release_id":"VerifierRelease","subject_commit_id":"SourceCommit","subject_tree_id":"SourceTree"}
    missing=[]; contradictory=[]
    for field,kind in expected.items():
        if field not in metadata: continue
        ref=metadata[field]
        if not isinstance(ref,str) or ref not in nodes: missing.append(field); continue
        if nodes[ref]["kind"]!=kind: contradictory.append(f"{field}:expected:{kind}:actual:{nodes[ref]['kind']}")
    return sorted(missing),sorted(contradictory)

def _proof_artifact_has_checker(node_id,graph):
    nodes={n["id"]:n for n in graph["nodes"]}
    return any(e["relation"]=="checked_by" and e["source"]==node_id and nodes.get(e["target"],{}).get("kind")=="ProofChecker" for e in graph["edges"])

def _required_bindings(node,graph,nodes):
    missing,contradictory=_reference_bindings(node,nodes)
    if node["kind"]=="ProofArtifact" and not _proof_artifact_has_checker(node["id"],graph): missing.append("checked_by")
    if node["kind"]=="ProofExecution":
        metadata=node.get("metadata") if isinstance(node.get("metadata"),dict) else {}
        for field in PROOF_EXECUTION_FIELDS:
            if not isinstance(metadata.get(field),str) or not metadata[field]: missing.append(field)
    return sorted(set(missing)),sorted(set(contradictory))

def _canonical_impact(graph,changed_ids): return analyze(graph,changed_ids)

def _authenticated_impact(graph,impact):
    if not isinstance(impact,dict): raise ValueError("impact result must be an object")
    if impact.get("schema_version")!=SCHEMA_VERSION: raise ValueError("impact schema_version does not match provenance schema")
    if impact.get("impact_version")!=IMPACT_VERSION: raise ValueError("unsupported impact_version")
    if impact.get("impact_status") not in {"NoImpact","RevalidationRequired"}: raise ValueError("unsupported impact_status")
    changed_ids=impact.get("changed_node_ids")
    if not isinstance(changed_ids,list) or not changed_ids or any(not isinstance(x,str) or not x for x in changed_ids): raise ValueError("changed_node_ids must be a non-empty-string array")
    if len(changed_ids)!=len(set(changed_ids)): raise ValueError("changed_node_ids must not contain duplicates")
    canonical=_canonical_impact(graph,changed_ids)
    if canonical!=impact: raise ValueError("impact payload does not match canonical impact analysis: "+json.dumps({"expected":canonical,"actual":impact},sort_keys=True,separators=(",",":")))
    return canonical

def build_plan(graph,impact):
    _validate_graph(graph); authenticated=_authenticated_impact(graph,impact)
    nodes={n["id"]:n for n in graph["nodes"]}; superseded,invalidated=_history_invalidations(graph)
    plans=[]; blockers=[]
    for record in authenticated["records"]:
        node_id=record["node_id"]; node=nodes[node_id]; missing,contradictory=_required_bindings(node,graph,nodes); historical=[]
        if node_id in superseded: historical.append("SUPERSEDED")
        if node_id in invalidated: historical.append("INVALIDATED")
        if missing or contradictory or historical:
            blockers.append({"node_id":node_id,"kind":node["kind"],"status":"Blocked","missing_bindings":missing,"contradictory_bindings":contradictory,"historical_flags":sorted(historical)}); continue
        if node["kind"] in {"Theorem","Lemma","Invariant","ProofArtifact","ProofChecker"}: action="RECHECK_PROOF"
        elif node["kind"]=="ProofExecution": action="RECHECK_EXECUTION"
        elif node["kind"]=="QualificationEvidence": action="RECHECK_QUALIFICATION_EVIDENCE"
        else: action="RECHECK_DEPENDENCY"
        plans.append({"node_id":node_id,"kind":node["kind"],"action":action,"reason_code":record["reason_code"],"via_edges":list(record["via_edges"])})
    status="Blocked" if blockers else "Ready"
    if not authenticated["records"]: status="NoRevalidationRequired"
    return {"schema_version":SCHEMA_VERSION,"plan_version":PLAN_VERSION,"plan_status":status,"records":plans,"blockers":blockers}

def main():
    parser=argparse.ArgumentParser(description="Build a PKG revalidation plan"); parser.add_argument("graph"); parser.add_argument("impact"); args=parser.parse_args()
    try:
        with open(args.graph,encoding="utf-8") as f: graph=json.load(f)
        with open(args.impact,encoding="utf-8") as f: impact=json.load(f)
        result=build_plan(graph,impact)
    except (OSError,UnicodeError,json.JSONDecodeError,ValueError) as exc:
        result={"schema_version":SCHEMA_VERSION,"plan_version":PLAN_VERSION,"plan_status":"Invalid","records":[],"blockers":[],"error":str(exc)}
    print(json.dumps(result,sort_keys=True,separators=(",",":"))); return 1 if result["plan_status"]=="Invalid" else 0
if __name__=="__main__": raise SystemExit(main())
