#!/usr/bin/env python3
from __future__ import annotations
import argparse, datetime as dt, json, os, urllib.parse, urllib.request
from collections import defaultdict
from dataclasses import asdict, dataclass
UTC = dt.timezone.utc
ACTIVE_WORKFLOW_STATES = ("queued", "in_progress")
PER_PAGE = 100
MAX_ACTIVE_RUNS = 1000
MAX_ATTEMPTS_PER_RUN = 20

@dataclass(frozen=True)
class JobRow:
    run_id:int; run_number:int|None; run_attempt:int; current_attempt:bool
    workflow:str; workflow_status:str; branch:str; head_sha:str
    job_id:int; job_name:str; labels:list[str]; status:str; conclusion:str|None
    runner_id:int; runner_name:str; created_at:str; started_at:str|None
    completed_at:str|None; steps_count:int; allocation_wait_hours:float; stale:bool

def api_get(url, token):
    h={"Accept":"application/vnd.github+json","X-GitHub-Api-Version":"2022-11-28","User-Agent":"symtropy-ci-runner-allocation-report"}
    if token: h["Authorization"]=f"Bearer {token}"
    with urllib.request.urlopen(urllib.request.Request(url,headers=h),timeout=30) as r: return json.load(r)

def parse_time(v):
    return None if not v else dt.datetime.fromisoformat(v.replace("Z","+00:00")).astimezone(UTC)

def paged_items(base_url,item_key,token,max_items=None):
    items=[]; page=1
    while True:
        sep="&" if "?" in base_url else "?"
        url=f"{base_url}{sep}{urllib.parse.urlencode({'per_page':PER_PAGE,'page':page})}"
        batch=api_get(url,token).get(item_key,[])
        if not batch: break
        items.extend(batch)
        if max_items is not None and len(items)>=max_items: return items[:max_items]
        if len(batch)<PER_PAGE: break
        page+=1
    return items

def fetch_active_runs(owner,name,token):
    by_id={}
    for status in ACTIVE_WORKFLOW_STATES:
        q=urllib.parse.urlencode({"status":status})
        for run in paged_items(f"https://api.github.com/repos/{owner}/{name}/actions/runs?{q}","workflow_runs",token,MAX_ACTIVE_RUNS):
            by_id[int(run["id"])]=run
    if len(by_id)>=MAX_ACTIVE_RUNS: raise SystemExit("active workflow population reached safety cap")
    return sorted(by_id.values(),key=lambda r:str(r.get("created_at","")))

def fetch_attempt_jobs(owner,name,run_id,attempt,token):
    return paged_items(f"https://api.github.com/repos/{owner}/{name}/actions/runs/{run_id}/attempts/{attempt}/jobs","jobs",token)

def pool_name(labels):
    for label in labels:
        if label.endswith("-latest") or label=="ubuntu-slim": return label
    return ",".join(labels) if labels else "unlabeled"

def allocation_wait_hours(job,now):
    created=parse_time(job.get("created_at"))
    if not created:return 0.0
    rid=int(job.get("runner_id") or 0)
    started=parse_time(job.get("started_at")); completed=parse_time(job.get("completed_at"))
    end=started if rid and started else completed if completed else now
    return max(0.0,(end-created).total_seconds()/3600)

def main():
    p=argparse.ArgumentParser(description="Inspect active GitHub Actions jobs by runner pool and attempt.")
    p.add_argument("--repo",default=os.getenv("GITHUB_REPOSITORY","Luminous-Dynamics/symtropy"))
    p.add_argument("--limit",type=int,default=0)
    p.add_argument("--stale-after-hours",type=float,default=24.0)
    p.add_argument("--json",action="store_true")
    a=p.parse_args()
    if "/" not in a.repo: raise SystemExit("--repo must be owner/repo")
    if a.limit<0 or a.stale_after_hours<0: raise SystemExit("limits must be >= 0")
    owner,name=a.repo.split("/",1); token=os.getenv("GITHUB_TOKEN"); now=dt.datetime.now(UTC)
    all_runs=fetch_active_runs(owner,name,token); runs=all_runs[:a.limit] if a.limit else all_runs
    rows=[]
    for run in runs:
        current=int(run.get("run_attempt") or 1)
        if current>MAX_ATTEMPTS_PER_RUN: raise SystemExit(f"run {run['id']} exceeds attempt safety cap")
        for attempt in range(1,current+1):
            for job in fetch_attempt_jobs(owner,name,int(run["id"]),attempt,token):
                rid=int(job.get("runner_id") or 0); status=str(job.get("status","")); wait=allocation_wait_hours(job,now); cur=attempt==current
                stale=cur and status=="queued" and rid==0 and wait>=a.stale_after_hours
                rows.append(JobRow(int(run["id"]),run.get("run_number"),attempt,cur,str(run.get("name","")),str(run.get("status","")),str(run.get("head_branch","")),str(run.get("head_sha","")),int(job["id"]),str(job.get("name","")),[str(x) for x in job.get("labels",[])],status,job.get("conclusion"),rid,str(job.get("runner_name") or ""),str(job.get("created_at","")),job.get("started_at"),job.get("completed_at"),len(job.get("steps") or []),round(wait,2),stale))
    current_rows=[r for r in rows if r.current_attempt]; hist=[r for r in rows if not r.current_attempt]
    run_states={}; pools=defaultdict(lambda:{"jobs":0,"unassigned":0,"stale":0,"oldest_unassigned_hours":0.0})
    for r in current_rows:
        st=run_states.setdefault(r.run_id,{"unassigned":0,"completed":0,"other":0}); un=r.status=="queued" and r.runner_id==0
        st["unassigned" if un else "completed" if r.status=="completed" else "other"]+=1
        ps=pools[pool_name(r.labels)]; ps["jobs"]+=1
        if un:
            ps["unassigned"]+=1; ps["oldest_unassigned_hours"]=max(ps["oldest_unassigned_hours"],r.allocation_wait_hours)
        if r.stale: ps["stale"]+=1
    summary={"repository":a.repo,"observed_at":now.isoformat(),"active_workflows_total":len(all_runs),"active_workflows_inspected":len(runs),"current_attempt_jobs":len(current_rows),"historical_attempt_jobs":len(hist),"zero_step_unassigned_jobs":sum(r.status=="queued" and r.runner_id==0 and r.steps_count==0 for r in current_rows),"stale_zero_step_jobs":sum(r.stale for r in current_rows),"stale_after_hours":a.stale_after_hours,"partially_drained_workflows":sum(st["unassigned"]>0 and (st["completed"]>0 or st["other"]>0) for st in run_states.values()),"pools":dict(sorted(pools.items()))}
    if a.json:
        print(json.dumps({"summary":summary,"jobs":[asdict(r) for r in rows]},indent=2,sort_keys=True)); return 0
    print("repo={repository} observed_at={observed_at} active_total={active_workflows_total} active_inspected={active_workflows_inspected} current_jobs={current_attempt_jobs} historical_jobs={historical_attempt_jobs} unassigned={zero_step_unassigned_jobs} stale={stale_zero_step_jobs} partial={partially_drained_workflows}".format(**summary))
    for pool,st in summary["pools"].items(): print(f"pool={pool} jobs={st['jobs']} unassigned={st['unassigned']} stale={st['stale']} oldest_unassigned_h={st['oldest_unassigned_hours']:.2f}")
    print("run/att/job".ljust(30),"wait(h)".rjust(8),"runner".ljust(13),"status".ljust(11),"scope".ljust(8),"pool/job")
    for r in sorted(rows,key=lambda x:(not x.current_attempt,-x.allocation_wait_hours,x.run_id,x.run_attempt,x.job_id)):
        runner=str(r.runner_id) if r.runner_id else "-"; scope="CURRENT" if r.current_attempt else "HIST"; marker=" STALE" if r.stale else ""
        print(f"{r.run_id}/a{r.run_attempt}/{r.job_id}".ljust(30),f"{r.allocation_wait_hours:8.2f}",runner.ljust(13),r.status.ljust(11),scope.ljust(8),f"{pool_name(r.labels)} :: {r.job_name}{marker}")
    return 0
if __name__=="__main__": raise SystemExit(main())
