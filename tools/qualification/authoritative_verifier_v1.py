#!/usr/bin/env python3
"""QUAL-001B verifier-owned execution engine v1.

This engine can emit TheoremExecutedPass/Fail only. Promotion to QualifiedPass
requires separate admission against an approved verifier release identity.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, NoReturn, Sequence

QUAL_DIR = Path(__file__).resolve().parent
if str(QUAL_DIR) not in sys.path:
    sys.path.insert(0, str(QUAL_DIR))
import qualification_contract_v1 as contract_v1  # noqa: E402
import rust_workspace_package_profile_v1 as rust_profile_v1  # noqa: E402
import validate_manifest_v1 as manifest_base  # noqa: E402
import validate_manifest_v1_a2 as manifest_a2  # noqa: E402

SUPPORTED_SUITE = "rust-workspace-package-v1"
SUPPORTED_TOOLCHAIN = "rust-1.96.0"
STEP_IDS = ("subject_identity","toolchain_identity","format","focused_test","strict_clippy","locked_check","immutable_postflight")

class VerificationError(RuntimeError):
    def __init__(self, step: str, message: str): super().__init__(message); self.step=step
class InfrastructureError(RuntimeError):
    def __init__(self, step: str, message: str): super().__init__(message); self.step=step
@dataclass
class StepResult:
    status: str="not_reached"; exit_code: int|None=None; log_sha256: str|None=None
@dataclass
class Evidence:
    verifier_commit_sha: str=""; verifier_tree_sha: str=""; contract_commit_sha: str=""; contract_tree_sha: str=""
    contract_sha256: str=""; contract_id: str=""; subject_repository: str=""; subject_head_sha: str=""; subject_tree_sha: str=""
    manifest_sha256: str=""; manifest_profile_id: str=""; suite_profile_sha256: str=""; suite_id: str=""; suite_revision: str="rust-workspace-package-v1/1"; toolchain_id: str=""
    steps: dict[str,StepResult]=field(default_factory=lambda:{s:StepResult() for s in STEP_IDS})
    first_failing_step_id: str|None=None; first_failing_step_result: str|None=None; final_result: str="InfrastructureFailure"
    def to_json(self):
        return {"schema_id":"luminous.qualification-execution-evidence.v1","verifier_commit_sha":self.verifier_commit_sha,"verifier_tree_sha":self.verifier_tree_sha,"contract_commit_sha":self.contract_commit_sha,"contract_tree_sha":self.contract_tree_sha,"contract_sha256":self.contract_sha256,"contract_id":self.contract_id,"subject_repository":self.subject_repository,"subject_head_sha":self.subject_head_sha,"subject_tree_sha":self.subject_tree_sha,"manifest_sha256":self.manifest_sha256,"manifest_profile_id":self.manifest_profile_id,"suite_profile_sha256":self.suite_profile_sha256,"suite_id":self.suite_id,"suite_revision":self.suite_revision,"toolchain_id":self.toolchain_id,"expanded_step_ids":list(STEP_IDS),"executed_step_ids":[k for k,v in self.steps.items() if v.status!="not_reached"],"steps":{k:vars(v) for k,v in self.steps.items()},"first_failing_step_id":self.first_failing_step_id,"first_failing_step_result":self.first_failing_step_result,"final_result":self.final_result}
Runner=Callable[[Sequence[str],Path],subprocess.CompletedProcess[str]]
def _fail(step,message)->NoReturn: raise VerificationError(step,message)
def _run(argv,cwd):
    try: return subprocess.run(list(argv),cwd=cwd,check=False,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=os.environ.copy())
    except FileNotFoundError as exc: raise InfrastructureError("toolchain_identity",f"missing executable: {argv[0]}") from exc
    except OSError as exc: raise InfrastructureError("subject_identity",f"cannot execute {argv[0]}: {exc}") from exc
def _git(runner,root,*args):
    r=runner(["git",*args],root)
    if r.returncode!=0: raise InfrastructureError("subject_identity",f"git {' '.join(args)} failed: {(r.stdout or '').strip()}")
    return (r.stdout or "").strip()
def _repo_baseline(runner,root,expected_head,label):
    head=_git(runner,root,"rev-parse","HEAD"); tree=_git(runner,root,"rev-parse","HEAD^{tree}")
    if expected_head is not None and head!=expected_head: _fail("subject_identity",f"{label} HEAD does not match expected exact commit")
    if _git(runner,root,"status","--porcelain=v1","--untracked-files=all"): _fail("subject_identity",f"{label} checkout is not clean before execution")
    return {"head":head,"tree":tree}
def _repo_postflight(runner,root,baseline,label):
    if _git(runner,root,"rev-parse","HEAD")!=baseline["head"] or _git(runner,root,"rev-parse","HEAD^{tree}")!=baseline["tree"]: _fail("immutable_postflight",f"{label} HEAD/tree changed during execution")
    if _git(runner,root,"diff","--name-only"): _fail("immutable_postflight",f"{label} tracked working-tree files changed")
    if _git(runner,root,"diff","--cached","--name-only"): _fail("immutable_postflight",f"{label} index changed")
    if _git(runner,root,"ls-files","--others","--exclude-standard"): _fail("immutable_postflight",f"{label} gained untracked files")
def _sha256_file(path):
    h=hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda:f.read(1024*1024),b""): h.update(chunk)
    return h.hexdigest()
def _is_covered(specs,path):
    return any((s["kind"]=="file" and s["path"]==path) or (s["kind"]=="prefix" and (path==s["path"] or path.startswith(s["path"]+"/"))) for s in specs)
def _resolve_subject_regular(root,rel,step):
    rr=root.resolve(); c=root.joinpath(*rel.split("/"))
    try: r=c.resolve(strict=True); r.relative_to(rr)
    except (OSError,ValueError) as exc: _fail(step,f"subject path cannot be resolved safely: {rel}: {exc}")
    if c.is_symlink() or not r.is_file(): _fail(step,f"subject path must be a regular non-symlink file: {rel}")
    return r
def _check_linear_ancestry(runner,root,parent,head,count):
    lines=_git(runner,root,"rev-list","--parents","--reverse",f"{parent}..{head}").splitlines()
    if len(lines)!=count: _fail("subject_identity",f"required_commit_count mismatch: expected {count}, got {len(lines)}")
    expected=parent
    for line in lines:
        parts=line.split()
        if len(parts)!=2 or parts[1]!=expected: _fail("subject_identity","subject lineage must be exact and linear")
        expected=parts[0]
    if expected!=head: _fail("subject_identity","subject head is not terminal commit of exact lineage")
def _assert_parent_equal(runner,root,parent,head,spec):
    r=runner(["git","diff","--quiet",parent,head,"--",spec["path"]],root)
    if r.returncode not in {0,1}: raise InfrastructureError("subject_identity",f"git diff failed for parent_equal path {spec['path']}")
    if r.returncode==1: _fail("subject_identity",f"parent_equal path changed: {spec['path']}")
def _assert_safe_git_mode(runner,root,rel):
    out=_git(runner,root,"ls-files","-s","--",rel).splitlines()
    if len(out)!=1: _fail("subject_identity",f"expected one tracked entry for {rel}")
    mode=out[0].split()[0]
    if mode not in {"100644","100755"}: _fail("subject_identity",f"unsafe Git mode {mode} for {rel}")
def _load_bound_inputs(contract_root,contract_rel,expected_contract_sha256):
    rel=contract_v1.repo_path(contract_rel,"contract_path"); cp=contract_v1.resolve_regular_file(contract_root,rel,"contract_path")
    contract,cd=contract_v1.load_contract(cp); contract_v1.validate_contract(contract)
    if cd!=expected_contract_sha256: _fail("subject_identity","contract digest mismatch")
    mp=contract_v1.resolve_regular_file(contract_root,contract["manifest_path"],"manifest_path"); manifest,md=manifest_base.load_manifest(mp); manifest_a2.validate_manifest(manifest)
    if md!=contract["manifest_sha256"]: _fail("subject_identity","A2 manifest digest mismatch")
    pp=contract_v1.resolve_regular_file(contract_root,contract["suite_profile_path"],"suite_profile_path"); profile,pd=rust_profile_v1.load_profile(pp); rust_profile_v1.validate_profile(profile)
    if pd!=contract["suite_profile_sha256"]: _fail("subject_identity","suite profile digest mismatch")
    return contract,manifest,profile,cd,md,pd
def _subject_identity(runner,root,contract,manifest,profile):
    if manifest["qualification_suite_id"]!=SUPPORTED_SUITE: _fail("subject_identity",f"unsupported suite: {manifest['qualification_suite_id']}")
    if manifest["toolchain_id"]!=SUPPORTED_TOOLCHAIN: _fail("subject_identity",f"unsupported toolchain: {manifest['toolchain_id']}")
    head=_git(runner,root,"rev-parse","HEAD"); tree=_git(runner,root,"rev-parse","HEAD^{tree}")
    if head!=contract["subject_head_sha"] or tree!=contract["subject_tree_sha"]: _fail("subject_identity","subject HEAD/tree does not match contract")
    parent=manifest["exact_parent"]; _check_linear_ancestry(runner,root,parent,head,manifest["required_commit_count"])
    changed=[x for x in _git(runner,root,"diff","--name-only","--no-renames",parent,head,"--").splitlines() if x]
    if not changed: _fail("subject_identity","subject has no changed paths")
    unexpected=[p for p in changed if not _is_covered(manifest["owned_paths"],p)]
    if unexpected: _fail("subject_identity","changed paths outside owned_paths: "+", ".join(sorted(unexpected)))
    for spec in manifest["parent_equal_paths"]: _assert_parent_equal(runner,root,parent,head,spec)
    for item in manifest["profile_files"]:
        p=_resolve_subject_regular(root,item["path"],"subject_identity"); _assert_safe_git_mode(runner,root,item["path"])
        if p.stat().st_size!=item["byte_len"] or _sha256_file(p)!=item["sha256"]: _fail("subject_identity",f"profile file binding mismatch: {item['path']}")
    test_path=profile["test_path"]
    if not _is_covered(manifest["owned_paths"],test_path): _fail("subject_identity","suite test_path must be covered by owned_paths")
    _resolve_subject_regular(root,test_path,"subject_identity"); _assert_safe_git_mode(runner,root,test_path)
    cargo_lock=_resolve_subject_regular(root,"Cargo.lock","subject_identity"); _assert_safe_git_mode(runner,root,"Cargo.lock")
    if not _is_covered(manifest["parent_equal_paths"],"Cargo.lock"): _fail("subject_identity","rust suite requires Cargo.lock in parent_equal_paths")
    if _git(runner,root,"status","--porcelain=v1","--untracked-files=all"): _fail("subject_identity","subject checkout is not clean before execution")
    return {"head":head,"tree":tree,"cargo_lock_sha256":_sha256_file(cargo_lock)}
def _record_command(e,runner,step,argv,cwd,log_dir):
    r=runner(argv,cwd); log=r.stdout or ""; (log_dir/f"{step}.log").write_text(log,encoding="utf-8"); ok=r.returncode==0
    e.steps[step]=StepResult("pass" if ok else "fail",r.returncode,hashlib.sha256(log.encode()).hexdigest())
    if not ok and e.first_failing_step_id is None: e.first_failing_step_id=step; e.first_failing_step_result="fail"
    return ok
def _toolchain_identity(e,runner,root,log_dir):
    outs=[]
    for argv in (["rustc","--version"],["cargo","--version"]):
        r=runner(argv,root)
        if r.returncode!=0:
            e.steps["toolchain_identity"]=StepResult("fail",r.returncode,None); e.first_failing_step_id=e.first_failing_step_id or "toolchain_identity"; e.first_failing_step_result=e.first_failing_step_result or "fail"; return False
        outs.append((r.stdout or "").strip())
    log="\n".join(outs)+"\n"; (log_dir/"toolchain_identity.log").write_text(log,encoding="utf-8"); ok=outs[0].startswith("rustc 1.96.0 ") and outs[1].startswith("cargo 1.96.0 ")
    e.steps["toolchain_identity"]=StepResult("pass" if ok else "fail",0 if ok else 1,hashlib.sha256(log.encode()).hexdigest())
    if not ok and e.first_failing_step_id is None: e.first_failing_step_id="toolchain_identity"; e.first_failing_step_result="fail"
    return ok
def _postflight(e,runner,vr,cr,sr,vb,cb,sb,manifest):
    try:
        _repo_postflight(runner,vr,vb,"verifier"); _repo_postflight(runner,cr,cb,"contract")
        if _git(runner,sr,"rev-parse","HEAD")!=sb["head"] or _git(runner,sr,"rev-parse","HEAD^{tree}")!=sb["tree"]: _fail("immutable_postflight","subject HEAD/tree changed")
        if _sha256_file(sr/"Cargo.lock")!=sb["cargo_lock_sha256"]: _fail("immutable_postflight","Cargo.lock bytes changed")
        for item in manifest["profile_files"]:
            p=_resolve_subject_regular(sr,item["path"],"immutable_postflight")
            if p.stat().st_size!=item["byte_len"] or _sha256_file(p)!=item["sha256"]: _fail("immutable_postflight",f"profile file changed: {item['path']}")
        if _git(runner,sr,"diff","--name-only") or _git(runner,sr,"diff","--cached","--name-only"): _fail("immutable_postflight","subject tracked/index state changed")
        untracked=[p for p in _git(runner,sr,"ls-files","--others","--exclude-standard").splitlines() if p]
        bad=[p for p in untracked if not _is_covered(manifest["allow_transient_paths"],p)]
        if bad: _fail("immutable_postflight","unexpected untracked files: "+", ".join(sorted(bad)))
    except VerificationError:
        e.steps["immutable_postflight"]=StepResult("fail",1,None); e.first_failing_step_id=e.first_failing_step_id or "immutable_postflight"; e.first_failing_step_result=e.first_failing_step_result or "fail"; return False
    e.steps["immutable_postflight"]=StepResult("pass",0,None); return True
def verify(*,verifier_root,contract_root,subject_root,contract_path,expected_verifier_commit_sha,expected_contract_commit_sha,expected_contract_sha256,evidence_dir,runner=_run):
    evidence_dir.mkdir(parents=True,exist_ok=True); e=Evidence(); vb=cb=sb=manifest=None
    try:
        vb=_repo_baseline(runner,verifier_root,expected_verifier_commit_sha,"verifier"); cb=_repo_baseline(runner,contract_root,expected_contract_commit_sha,"contract")
        e.verifier_commit_sha=vb["head"]; e.verifier_tree_sha=vb["tree"]; e.contract_commit_sha=cb["head"]; e.contract_tree_sha=cb["tree"]
        contract,manifest,profile,cd,md,pd=_load_bound_inputs(contract_root,contract_path,expected_contract_sha256)
        e.contract_sha256=cd; e.contract_id=contract["contract_id"]; e.subject_repository=contract["subject_repository"]; e.subject_head_sha=contract["subject_head_sha"]; e.subject_tree_sha=contract["subject_tree_sha"]; e.manifest_sha256=md; e.manifest_profile_id=manifest["profile_id"]; e.suite_profile_sha256=pd; e.suite_id=manifest["qualification_suite_id"]; e.toolchain_id=manifest["toolchain_id"]
        sb=_subject_identity(runner,subject_root,contract,manifest,profile); e.steps["subject_identity"]=StepResult("pass",0,None)
        ok=_toolchain_identity(e,runner,subject_root,evidence_dir)
        commands=(("format",["rustfmt","--edition",profile["edition"],"--check",profile["test_path"]]),("focused_test",["cargo","test","--locked","-p",profile["package"],"--test",profile["integration_test"]]),("strict_clippy",["cargo","clippy","--locked","-p",profile["package"],"--test",profile["integration_test"],"--","-D","warnings"]),("locked_check",["cargo","check","--locked","-p",profile["package"],"--test",profile["integration_test"]]))
        if ok:
            for step,argv in commands:
                if not _record_command(e,runner,step,argv,subject_root,evidence_dir): ok=False; break
        post=_postflight(e,runner,verifier_root,contract_root,subject_root,vb,cb,sb,manifest); e.final_result="TheoremExecutedPass" if ok and post else "TheoremExecutedFail"; return e,0 if e.final_result=="TheoremExecutedPass" else 1
    except (contract_v1.ContractValidationError,rust_profile_v1.SuiteProfileValidationError,manifest_base.ManifestValidationError,VerificationError) as exc:
        step=getattr(exc,"step","subject_identity"); e.steps[step]=StepResult("fail",1,None); e.first_failing_step_id=e.first_failing_step_id or step; e.first_failing_step_result=e.first_failing_step_result or "fail"
        if vb is not None and cb is not None and sb is not None and manifest is not None: _postflight(e,runner,verifier_root,contract_root,subject_root,vb,cb,sb,manifest)
        e.final_result="TheoremExecutedFail"; return e,1
    except InfrastructureError as exc:
        e.steps[exc.step]=StepResult("infrastructure_failure",None,None); e.first_failing_step_id=e.first_failing_step_id or exc.step; e.first_failing_step_result=e.first_failing_step_result or "infrastructure_failure"; e.final_result="InfrastructureFailure"; return e,2
    finally:
        (evidence_dir/"qualification-execution-evidence-v1.json").write_text(json.dumps(e.to_json(),indent=2,sort_keys=True)+"\n",encoding="utf-8")
def main(argv=None):
    p=argparse.ArgumentParser(description=__doc__)
    for arg in ("verifier-root","contract-root","subject-root"): p.add_argument("--"+arg,type=Path,required=True)
    p.add_argument("--contract-path",required=True); p.add_argument("--expected-verifier-commit-sha",required=True); p.add_argument("--expected-contract-commit-sha",required=True); p.add_argument("--expected-contract-sha256",required=True); p.add_argument("--evidence-dir",type=Path,required=True)
    a=p.parse_args(argv); e,code=verify(verifier_root=a.verifier_root,contract_root=a.contract_root,subject_root=a.subject_root,contract_path=a.contract_path,expected_verifier_commit_sha=a.expected_verifier_commit_sha,expected_contract_commit_sha=a.expected_contract_commit_sha,expected_contract_sha256=a.expected_contract_sha256,evidence_dir=a.evidence_dir)
    print(f"final_result={e.final_result}");
    if e.first_failing_step_id: print(f"first_failing_step_id={e.first_failing_step_id}")
    return code
if __name__=="__main__": raise SystemExit(main())
