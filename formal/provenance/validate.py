#!/usr/bin/env python3
"""Fail-closed semantic validator for Symtropy formal provenance graph v0.

The graph is provenance/evidence only. Proof soundness and qualification authority
remain outside this validator.
"""
from __future__ import annotations
import argparse, json, re
from collections import defaultdict
from dataclasses import dataclass
from typing import Any
from pathlib import PurePosixPath

SCHEMA_VERSION="luminous.formal-provenance.v0"
SHA256_RE=re.compile(r"^[0-9a-f]{64}$")
SEMANTIC_KINDS={"Invariant","Definition","Theorem","Lemma"}
NODE_KINDS=SEMANTIC_KINDS|{"ProofArtifact","ProofChecker","ProofExecution","Counterexample","SourceCommit","SourceTree","QualificationContract","QualificationEvidence","VerifierRelease"}
RELATIONS={"defines","refines","depends_on","proves","checked_by","derived_from","falsified_by","revalidated_by","qualifies","supersedes","invalidated_by"}
TERMINAL_EXECUTIONS={"Passed","Failed"}

@dataclass(frozen=True)
class Diagnostic:
    code:str; message:str; path:str
    def as_dict(self): return {"code":self.code,"path":self.path,"message":self.message}

class Validator:
    def __init__(self,graph:dict[str,Any])->None:
        self.graph=graph; self.errors=[]; self.nodes={}; self.edges={}
    def error(self,code,path,message): self.errors.append(Diagnostic(code,message,path))
    def validate(self):
        self._shape()
        if self.errors:return self.result()
        self._index(); self._relationships(); self._require_proof_bindings(); self._proof_dag(); self._qualification()
        return self.result()
    def result(self):
        codes={e.code for e in self.errors}
        if not self.errors: status="Valid"
        elif any(c.startswith("E_SCHEMA") for c in codes): status="SchemaInvalid"
        elif any(c.startswith("E_EVIDENCE") or c.startswith("E_STALE") for c in codes): status="EvidenceInvalid"
        elif any(c.startswith("E_AUTHORITY") for c in codes): status="AuthorityNotEstablished"
        else: status="GraphInvalid"
        return {"schema_version":SCHEMA_VERSION,"status":status,
                "errors":[e.as_dict() for e in sorted(self.errors,key=lambda x:(x.code,x.path,x.message))],
                "authority":"NotEstablished" if any(c.startswith("E_AUTHORITY") for c in codes) else "Unchanged"}
    def _shape(self):
        if not isinstance(self.graph,dict): self.error("E_SCHEMA_ROOT","$","graph must be an object"); return
        if self.graph.get("schema_version")!=SCHEMA_VERSION:self.error("E_SCHEMA_VERSION","$.schema_version","unsupported schema version")
        for key in ("nodes","edges"):
            if not isinstance(self.graph.get(key),list):self.error("E_SCHEMA_ROOT",f"$.{key}","must be an array")
        if self.errors:return
        for i,n in enumerate(self.graph["nodes"]):
            if not isinstance(n,dict):self.error("E_SCHEMA_NODE",f"$.nodes[{i}]","node must be an object");continue
            if n.get("schema_version")!=SCHEMA_VERSION:self.error("E_SCHEMA_VERSION",f"$.nodes[{i}].schema_version","invalid schema version")
            if n.get("kind") not in NODE_KINDS:self.error("E_SCHEMA_NODE",f"$.nodes[{i}].kind","unknown node kind")
            for f in ("id","label"):
                if not isinstance(n.get(f),str) or not n[f]:self.error("E_SCHEMA_NODE",f"$.nodes[{i}].{f}","must be non-empty")
            if n.get("semantic_digest") is not None and not SHA256_RE.fullmatch(str(n["semantic_digest"])):
                self.error("E_SCHEMA_DIGEST",f"$.nodes[{i}].semantic_digest","invalid SHA-256 digest")
            if n.get("content_digest") is not None and not SHA256_RE.fullmatch(str(n["content_digest"])):
                self.error("E_SCHEMA_DIGEST",f"$.nodes[{i}].content_digest","invalid SHA-256 digest")
        for i,e in enumerate(self.graph["edges"]):
            if not isinstance(e,dict):self.error("E_SCHEMA_EDGE",f"$.edges[{i}]","edge must be an object");continue
            if e.get("schema_version")!=SCHEMA_VERSION:self.error("E_SCHEMA_VERSION",f"$.edges[{i}].schema_version","invalid schema version")
            if e.get("relation") not in RELATIONS:self.error("E_SCHEMA_EDGE",f"$.edges[{i}].relation","unknown relation")
            for f in ("id","source","target"):
                if not isinstance(e.get(f),str) or not e[f]:self.error("E_SCHEMA_EDGE",f"$.edges[{i}].{f}","must be non-empty")
    def _index(self):
        for i,n in enumerate(self.graph["nodes"]):
            nid=n.get("id")
            if nid in self.nodes:self.error("E_NODE_DUPLICATE",f"$.nodes[{i}].id",f"duplicate node id {nid!r}")
            else:self.nodes[nid]=n
            if n.get("kind") in SEMANTIC_KINDS:
                d=n.get("semantic_digest")
                if not isinstance(d,str) or not SHA256_RE.fullmatch(d):
                    self.error("E_SEMANTIC_ID_MISSING",f"node:{nid}.semantic_digest","semantic nodes require content-addressed semantic_digest")
                if isinstance(d,str) and not nid.endswith(d):
                    self.error("E_SEMANTIC_ID_MISMATCH",f"node:{nid}.id","semantic node id must end with its semantic_digest")
            self._check_provenance(n,f"node:{nid}.provenance")
        for i,e in enumerate(self.graph["edges"]):
            eid=e.get("id")
            if eid in self.edges:self.error("E_EDGE_DUPLICATE",f"$.edges[{i}].id",f"duplicate edge id {eid!r}")
            else:self.edges[eid]=e
            if e.get("source") not in self.nodes:self.error("E_EDGE_ENDPOINT_MISSING",f"$.edges[{i}].source","source node does not exist")
            if e.get("target") not in self.nodes:self.error("E_EDGE_ENDPOINT_MISSING",f"$.edges[{i}].target","target node does not exist")
            self._check_provenance(e,f"edge:{eid}.provenance")
    def _check_provenance(self,r,p):
        q=r.get("provenance")
        if q is None:return
        if not isinstance(q,dict):self.error("E_PROVENANCE_MALFORMED",p,"provenance must be an object");return
        for f in ("commit_sha","tree_sha"):
            if not isinstance(q.get(f),str) or not SHA256_RE.fullmatch(q[f]):self.error("E_PROVENANCE_MALFORMED",f"{p}.{f}","must be lowercase SHA-256")
        if not isinstance(q.get("repository"),str) or not q["repository"] or any(c.isspace() for c in q["repository"]):self.error("E_PROVENANCE_MALFORMED",f"{p}.repository","repository identity is invalid")
        path=q.get("path")
        if path is not None and (not isinstance(path,str) or not path or path.startswith("/") or "\x00" in path or "//" in path or any(x in {".",".."} for x in PurePosixPath(path).parts)):
            self.error("E_PROVENANCE_MALFORMED",f"{p}.path","repository path is not normalized")
        if q.get("content_digest") is not None and (not isinstance(q["content_digest"],str) or not SHA256_RE.fullmatch(q["content_digest"])):
            self.error("E_PROVENANCE_MALFORMED",f"{p}.content_digest","must be lowercase SHA-256")
    def _relationships(self):
        for eid,e in self.edges.items():
            s,t=self.nodes.get(e["source"]),self.nodes.get(e["target"])
            if not s or not t:continue
            rel=e["relation"]
            if rel=="proves":
                if s["kind"]!="ProofArtifact":self.error("E_SEMANTIC_ID_MISMATCH",f"edge:{eid}","proves source must be ProofArtifact")
                if t["kind"] not in SEMANTIC_KINDS:self.error("E_SEMANTIC_ID_MISMATCH",f"edge:{eid}","proves target must be semantic theorem/lemma/invariant/definition")
                if s.get("metadata",{}).get("target_semantic_digest")!=t.get("semantic_digest"):
                    self.error("E_SEMANTIC_ID_MISMATCH",f"edge:{eid}.source","proof must bind exact target semantic_digest")
            if rel=="checked_by":
                if s["kind"] not in {"ProofArtifact","ProofExecution"}:self.error("E_SEMANTIC_ID_MISMATCH",f"edge:{eid}","checked_by source must be ProofArtifact or ProofExecution")
                if t["kind"]!="ProofChecker":self.error("E_SEMANTIC_ID_MISMATCH",f"edge:{eid}","checked_by target must be ProofChecker")
            if rel=="qualifies" and t["kind"]!="QualificationContract":self.error("E_AUTHORITY_QUALIFIES_TARGET",f"edge:{eid}","qualifies target must be QualificationContract")
            if rel=="checked_by" and s["kind"]=="ProofArtifact" and t["kind"]=="ProofChecker":
                pass
            ev=e.get("evidence",[])
            if not isinstance(ev,list) or any(not isinstance(x,str) or x not in self.nodes for x in ev):self.error("E_EVIDENCE_REFERENCE",f"edge:{eid}.evidence","all evidence references must name existing node IDs")
            if rel=="qualifies":
                if not ev:self.error("E_AUTHORITY_UNPROVEN",f"edge:{eid}","qualifies requires QualificationEvidence")
                if any(self.nodes.get(x,{}).get("kind")!="QualificationEvidence" for x in ev):self.error("E_AUTHORITY_UNPROVEN",f"edge:{eid}.evidence","qualifies evidence must be QualificationEvidence")
    def _require_proof_bindings(self):
        checked={}; proven={}
        for e in self.edges.values():
            if e["relation"]=="checked_by" and self.nodes.get(e["source"],{}).get("kind")=="ProofArtifact":
                checked.setdefault(e["source"],[]).append(e["target"])
            if e["relation"]=="proves" and self.nodes.get(e["source"],{}).get("kind")=="ProofArtifact":
                proven.setdefault(e["source"],[]).append(e["target"])
        for nid,n in self.nodes.items():
            if n["kind"]!="ProofArtifact":continue
            if len(checked.get(nid,[]))!=1:self.error("E_CHECKER_BINDING",f"node:{nid}","ProofArtifact must have exactly one checked_by ProofChecker edge")
            if len(proven.get(nid,[]))!=1:self.error("E_SEMANTIC_ID_MISMATCH",f"node:{nid}","ProofArtifact must have exactly one proves edge")
        for eid,e in self.edges.items():
            if e["relation"]!="checked_by":continue
            s=self.nodes.get(e["source"]);t=self.nodes.get(e["target"])
            if s and t and s["kind"]=="ProofArtifact" and t["kind"]=="ProofChecker":
                if not isinstance(t.get("content_digest"),str) or not SHA256_RE.fullmatch(t["content_digest"]):
                    self.error("E_CHECKER_BINDING",f"node:{t['id']}.content_digest","ProofChecker must be content-addressed")
    def _proof_dag(self):
        a=defaultdict(list)
        for eid,e in self.edges.items():
            if e["relation"]=="depends_on" and self.nodes.get(e["source"],{}).get("kind")=="ProofArtifact" and self.nodes.get(e["target"],{}).get("kind")=="ProofArtifact":
                if e["source"]==e["target"]:self.error("E_PROOF_CYCLE",f"edge:{eid}","proof artifact cannot depend on itself")
                a[e["source"]].append(e["target"])
        visiting=set();visited=set()
        def visit(n):
            if n in visiting:self.error("E_PROOF_CYCLE",f"node:{n}","proof dependency graph contains a cycle");return
            if n in visited:return
            visiting.add(n)
            for c in a[n]:visit(c)
            visiting.remove(n);visited.add(n)
        for n in {k for k,v in self.nodes.items() if v["kind"]=="ProofArtifact"}:visit(n)
    def _qualification(self):
        for nid,n in self.nodes.items():
            if n["kind"]!="QualificationEvidence":continue
            m=n.get("metadata")
            if not isinstance(m,dict):self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata","qualification evidence requires binding metadata");continue
            for f in ("contract_id","verifier_release_id","subject_head","subject_tree","execution_id","result"):
                if f not in m:self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.{f}","missing qualification binding")
            for f in ("subject_head","subject_tree"):
                if f in m and (not isinstance(m[f],str) or not SHA256_RE.fullmatch(m[f])):self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.{f}","must be lowercase SHA-256")
            if m.get("result")=="QualifiedPass":self.error("E_AUTHORITY_NOT_LOCAL",f"node:{nid}","QualifiedPass cannot be manufactured by the provenance graph")
            c=self.nodes.get(m.get("contract_id"));v=self.nodes.get(m.get("verifier_release_id"));x=self.nodes.get(m.get("execution_id"))
            if not c or c["kind"]!="QualificationContract":self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.contract_id","must reference QualificationContract")
            if not v or v["kind"]!="VerifierRelease":self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.verifier_release_id","must reference VerifierRelease")
            if not x or x["kind"]!="ProofExecution":self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.execution_id","must reference ProofExecution")
            if not x:continue
            xm=x.get("metadata",{})
            if not isinstance(xm,dict):xm={}
            if xm.get("status") not in TERMINAL_EXECUTIONS:self.error("E_AUTHORITY_UNPROVEN",f"node:{nid}","qualification requires terminal Passed/Failed execution")
            for f in ("artifact_id","checker_id","source_commit_id","source_tree_id"):
                ref=xm.get(f); expected={"artifact_id":"ProofArtifact","checker_id":"ProofChecker","source_commit_id":"SourceCommit","source_tree_id":"SourceTree"}[f]
                if not isinstance(ref,str) or self.nodes.get(ref,{}).get("kind")!=expected:self.error("E_EVIDENCE_BINDING",f"node:{x.get('id')}.metadata.{f}",f"must reference {expected}")
            if xm.get("subject_head")!=m.get("subject_head"):self.error("E_STALE_EVIDENCE",f"node:{nid}.metadata.subject_head","qualification and execution subject heads differ")
            if xm.get("subject_tree")!=m.get("subject_tree"):self.error("E_STALE_EVIDENCE",f"node:{nid}.metadata.subject_tree","qualification and execution subject trees differ")
            if xm.get("result")!=m.get("result"):self.error("E_EVIDENCE_BINDING",f"node:{nid}.metadata.result","qualification result differs from execution result")
        for eid,e in self.edges.items():
            if e["relation"]!="qualifies":continue
            for ref in e.get("evidence",[]):
                r=self.nodes.get(ref)
                if not r or r["kind"]!="QualificationEvidence":continue
                result=r.get("metadata",{}).get("result")
                if result not in {"Pass","Passed","Fail","Failed"}:self.error("E_AUTHORITY_UNPROVEN",f"edge:{eid}","qualification evidence has no ordinary terminal result")
                if result=="QualifiedPass":self.error("E_AUTHORITY_NOT_LOCAL",f"edge:{eid}","graph cannot manufacture QualifiedPass")
def validate_file(path):
    try:
        with open(path,encoding="utf-8") as h:return Validator(json.load(h)).validate()
    except (OSError,json.JSONDecodeError,UnicodeError) as exc:return {"schema_version":SCHEMA_VERSION,"status":"SchemaInvalid","errors":[{"code":"E_SCHEMA_INPUT","path":"$","message":str(exc)}],"authority":"NotEstablished"}
def main():
    p=argparse.ArgumentParser();p.add_argument("graph");a=p.parse_args();r=validate_file(a.graph);print(json.dumps(r,sort_keys=True,separators=(",",":")));return 0 if r["status"]=="Valid" else 1
if __name__=="__main__":raise SystemExit(main())
