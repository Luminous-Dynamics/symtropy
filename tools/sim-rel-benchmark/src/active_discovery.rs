//! Active intervention selection over finite synthetic causal-model candidates.
//! The policy sees candidates, prior observations and available actions only;
//! evaluator truth labels are supplied through a separate TSV input.
//! This is a model-selection prototype, not a grammar learner or physics result.
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::Command};

const RANDOM_REPETITIONS: usize = 64;
const SHA256_K: [u32; 64] = [
0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];

#[derive(Clone, Debug)]
struct Candidate { id: String, outputs: Vec<u8> }
#[derive(Clone, Debug)]
struct Problem {
    id: String, split: String, actions: Vec<u8>,
    initial: Vec<(u8,u8)>, candidates: Vec<Candidate>,
}
#[derive(Clone, Debug)]
struct Task { problem: Problem, truth: String }
#[derive(Clone, Copy, Debug)]
enum Policy { Active, Fixed, Random }
impl Policy {
    fn name(self) -> &'static str { match self {
        Self::Active=>"active_greedy_max_split", Self::Fixed=>"fixed_ascending_action",
        Self::Random=>"seeded_random_action"
    }}
}
#[derive(Clone, Debug)]
struct Episode {
    task_id:String, split:String, target:String, policy:String, actions:Vec<u8>,
    remaining:Vec<String>, identified:bool, correct:bool, ambiguous:bool, truth_retained:bool,
}
struct Case {
    id:String, split:String, candidate_count:usize, initial_viable:usize,
    active:Episode, fixed:Episode, random:Vec<Episode>,
}

fn valid_id(s:&str)->bool { !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b==b'_' || b==b'-') }
fn parse_bits(s:&str)->Result<Vec<u8>,String>{
    if s.len()!=8 || !s.bytes().all(|b|b==b'0'||b==b'1'){return Err(format!("signature must be eight bits: {s:?}"));}
    Ok(s.bytes().map(|b|u8::from(b==b'1')).collect())
}
fn parse_actions(s:&str)->Result<Vec<u8>,String>{
    let mut out=Vec::new(); let mut seen=BTreeSet::new();
    if s.is_empty(){return Err("empty action set".into());}
    for part in s.split(','){
        let n:u8=part.parse().map_err(|_|format!("invalid action {part:?}"))?;
        if n>7 || !seen.insert(n){return Err(format!("out-of-range or duplicate action {n}"));}
        out.push(n);
    }
    if out.windows(2).any(|w|w[0]>=w[1]){return Err("actions must be strictly ascending".into());}
    Ok(out)
}
fn parse_initial(s:&str)->Result<Vec<(u8,u8)>,String>{
    if s=="none"{return Ok(Vec::new());}
    let mut out=Vec::new(); let mut seen=BTreeSet::new();
    for part in s.split(','){
        let (a,y)=part.split_once(':').ok_or_else(||format!("invalid observation {part:?}"))?;
        let a:u8=a.parse().map_err(|_|"invalid observation action")?;
        let y:u8=y.parse().map_err(|_|"invalid observation output")?;
        if a>7||y>1||!seen.insert(a){return Err(format!("invalid/duplicate observation {part:?}"));}
        out.push((a,y));
    }
    out.sort_unstable_by_key(|x|x.0); Ok(out)
}
fn parse_candidates(s:&str)->Result<Vec<Candidate>,String>{
    let mut out=Vec::new(); let mut ids=BTreeSet::new();
    for spec in s.split(';'){
        let (id,bits)=spec.split_once('=').ok_or_else(||format!("invalid candidate {spec:?}"))?;
        if !valid_id(id)||!ids.insert(id.to_owned()){return Err(format!("invalid/duplicate candidate ID {id:?}"));}
        out.push(Candidate{id:id.into(),outputs:parse_bits(bits)?});
    }
    if out.len()<2{return Err("need at least two candidates".into());}
    Ok(out)
}
fn parse_problems(raw:&str)->Result<Vec<Problem>,String>{
    let mut out=Vec::new(); let mut ids=BTreeSet::new();
    for (line_no,raw_line) in raw.lines().enumerate(){
        let line=raw_line.trim(); if line.is_empty()||line.starts_with('#'){continue;}
        let f:Vec<_>=line.split('|').collect();
        if f.len()!=5{return Err(format!("line {} expected 5 fields",line_no+1));}
        if !valid_id(f[0])||!ids.insert(f[0].to_owned()){return Err(format!("invalid/duplicate task ID on line {}",line_no+1));}
        if !matches!(f[1],"train"|"validation"|"heldout"){return Err(format!("bad split on line {}",line_no+1));}
        let actions=parse_actions(f[2]).map_err(|e|format!("line {}: {e}",line_no+1))?;
        let initial=parse_initial(f[3]).map_err(|e|format!("line {}: {e}",line_no+1))?;
        if initial.iter().any(|(a,_)|actions.contains(a)){return Err(format!("line {} repeats initially observed action",line_no+1));}
        let candidates=parse_candidates(f[4]).map_err(|e|format!("line {}: {e}",line_no+1))?;
        out.push(Problem{id:f[0].into(),split:f[1].into(),actions,initial,candidates});
    }
    for split in ["train","validation","heldout"]{if !out.iter().any(|p|p.split==split){return Err(format!("missing split {split}"));}}
    Ok(out)
}
fn parse_truth(raw:&str)->Result<BTreeMap<String,String>,String>{
    let mut out=BTreeMap::new();
    for (line_no,raw_line) in raw.lines().enumerate(){
        let line=raw_line.trim(); if line.is_empty()||line.starts_with('#'){continue;}
        let f:Vec<_>=line.split('|').collect();
        if f.len()!=2||!valid_id(f[0])||!valid_id(f[1]){return Err(format!("bad truth row {}",line_no+1));}
        if out.insert(f[0].into(),f[1].into()).is_some(){return Err(format!("duplicate truth row {}",line_no+1));}
    }
    if out.is_empty(){return Err("empty truth fixture".into());} Ok(out)
}
fn viable(candidates:&[Candidate],obs:&[(u8,u8)])->Vec<usize>{
    candidates.iter().enumerate().filter_map(|(i,c)|
        obs.iter().all(|(a,y)|c.outputs[*a as usize]==*y).then_some(i)
    ).collect()
}
fn assemble(problems:Vec<Problem>,truth:BTreeMap<String,String>)->Result<Vec<Task>,String>{
    let pids:BTreeSet<_>=problems.iter().map(|p|p.id.clone()).collect();
    let tids:BTreeSet<_>=truth.keys().cloned().collect();
    if pids!=tids{return Err("problem and evaluator truth ID sets differ".into());}
    let mut tasks=Vec::new();
    for problem in problems{
        let target=truth.get(&problem.id).ok_or("missing truth label")?.clone();
        let target_candidate=problem.candidates.iter().find(|c|c.id==target)
            .ok_or_else(||format!("truth {target} not in candidate set for {}",problem.id))?;
        if problem.initial.iter().any(|(a,y)|target_candidate.outputs[*a as usize]!=*y){
            return Err(format!("initial evidence contradicts truth for {}",problem.id));
        }
        if !viable(&problem.candidates,&problem.initial).iter().any(|i|problem.candidates[*i].id==target){
            return Err(format!("initial evidence removes truth for {}",problem.id));
        }
        tasks.push(Task{problem,truth:target});
    }
    Ok(tasks)
}
fn next_rng(state:&mut u64)->u64{
    if *state==0{*state=0x9e3779b97f4a7c15;}
    let mut x=*state; x^=x<<13; x^=x>>7; x^=x<<17; *state=x; x
}
fn stable_seed(s:&str)->u64{
    let mut h=0xcbf29ce484222325u64;
    for b in s.bytes(){h^=u64::from(b);h=h.wrapping_mul(0x100000001b3);} h
}
fn choose_action(candidates:&[Candidate],survivors:&[usize],available:&[u8],policy:Policy,rng:&mut u64)->Option<u8>{
    match policy{
        Policy::Fixed=>available.first().copied(),
        Policy::Random=>if available.is_empty(){None}else{Some(available[(next_rng(rng) as usize)%available.len()])},
        Policy::Active=>{
            let mut best=None; let mut best_score=0usize;
            for a in available{
                let ones=survivors.iter().filter(|i|candidates[**i].outputs[*a as usize]==1).count();
                let score=ones.min(survivors.len()-ones);
                if score>best_score{best_score=score;best=Some(*a);}
            }
            best
        }
    }
}
fn run_episode(p:&Problem,target:&str,policy:Policy,seed:u64)->Result<Episode,String>{
    let target_i=p.candidates.iter().position(|c|c.id==target).ok_or("unknown target")?;
    let mut survivors=viable(&p.candidates,&p.initial);
    if survivors.is_empty()||!survivors.contains(&target_i){return Err(format!("invalid initial candidates for {}",p.id));}
    let mut observations=p.initial.clone();
    let mut available:Vec<u8>=p.actions.iter().copied().filter(|a|!observations.iter().any(|(b,_)|a==b)).collect();
    let mut chosen=Vec::new(); let mut rng=seed;
    loop{
        if survivors.len()<=1{break;}
        // Important information-flow boundary: truth is not an input to choose_action.
        let Some(action)=choose_action(&p.candidates,&survivors,&available,policy,&mut rng) else{break;};
        let output=p.candidates[target_i].outputs[action as usize];
        observations.push((action,output)); chosen.push(action);
        available.retain(|a|*a!=action);
        survivors.retain(|i|p.candidates[*i].outputs[action as usize]==output);
        if survivors.is_empty(){return Err(format!("truth eliminated by {}",policy.name()));}
    }
    let remaining:Vec<String>=survivors.iter().map(|i|p.candidates[*i].id.clone()).collect();
    let identified=remaining.len()==1; let correct=identified&&remaining[0]==target;
    let truth_retained=remaining.iter().any(|id|id==target);
    Ok(Episode{task_id:p.id.clone(),split:p.split.clone(),target:target.into(),policy:policy.name().into(),
        actions:chosen,remaining,identified,correct,ambiguous:!identified,truth_retained})
}
fn sha256_hex(input:&[u8])->String{
    let mut data=input.to_vec(); let bits=(data.len() as u64).wrapping_mul(8); data.push(0x80);
    while data.len()%64!=56{data.push(0);} data.extend_from_slice(&bits.to_be_bytes());
    let mut h=[0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    for chunk in data.chunks_exact(64){
        let mut w=[0u32;64];
        for (i,b) in chunk.chunks_exact(4).take(16).enumerate(){w[i]=u32::from_be_bytes([b[0],b[1],b[2],b[3]]);}
        for i in 16..64{
            let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3);
            let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10);
            w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut z]=h;
        for i in 0..64{
            let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25); let ch=(e&f)^(!e&g);
            let t1=z.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
            let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22); let maj=(a&b)^(a&c)^(b&c);
            let t2=s0.wrapping_add(maj); z=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2);
        }
        h[0]=h[0].wrapping_add(a);h[1]=h[1].wrapping_add(b);h[2]=h[2].wrapping_add(c);h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e);h[5]=h[5].wrapping_add(f);h[6]=h[6].wrapping_add(g);h[7]=h[7].wrapping_add(z);
    }
    let mut out=String::with_capacity(64); for word in h{use std::fmt::Write as _;let _=write!(out,"{word:08x}");}out
}
fn esc(s:&str)->String{
    let mut o=String::new();for c in s.chars(){match c{
        '\\'=>o.push_str("\\\\"),'"'=>o.push_str("\\\""),'\n'=>o.push_str("\\n"),'\r'=>o.push_str("\\r"),'\t'=>o.push_str("\\t"),
        c if c.is_control()=>o.push_str(&format!("\\u{:04x}",c as u32)),c=>o.push(c)}}o
}
fn str_array(v:&[String])->String{format!("[{}]",v.iter().map(|s|format!("\"{}\"",esc(s))).collect::<Vec<_>>().join(","))}
fn percentile(v:&[usize],p:f64)->usize{if v.is_empty(){0}else{v[((v.len()-1)as f64*p).ceil() as usize].min(*v.last().unwrap())}}
fn summary(policy:&str,split:&str,episodes:&[&Episode])->String{
    let n=episodes.len().max(1);let identified=episodes.iter().filter(|e|e.identified).count();
    let correct=episodes.iter().filter(|e|e.correct).count();let ambiguous=episodes.iter().filter(|e|e.ambiguous).count();
    let lost=episodes.iter().filter(|e|!e.truth_retained).count();
    let mut q:Vec<usize>=episodes.iter().map(|e|e.actions.len()).collect();q.sort_unstable();
    let mean=q.iter().sum::<usize>() as f64/n as f64;
    format!("{{\"policy\":\"{}\",\"split\":\"{}\",\"episodes\":{},\"identified\":{},\"correct_identifications\":{},\"ambiguous\":{},\"truth_lost\":{},\"identification_rate\":{:.8},\"ambiguity_rate\":{:.8},\"truth_retention_rate\":{:.8},\"mean_interventions\":{:.8},\"median_interventions\":{},\"p90_interventions\":{},\"max_interventions\":{}}}",
        esc(policy),esc(split),episodes.len(),identified,correct,ambiguous,lost,identified as f64/n as f64,ambiguous as f64/n as f64,
        (episodes.len()-lost) as f64/n as f64,mean,percentile(&q,0.5),percentile(&q,0.9),q.last().copied().unwrap_or(0))
}
fn render_episode(e:&Episode)->String{
    format!("{{\"policy\":\"{}\",\"identified\":{},\"correct\":{},\"ambiguous\":{},\"truth_retained\":{},\"intervention_count\":{},\"actions\":[{}],\"remaining_models\":{}}}",
        esc(&e.policy),e.identified,e.correct,e.ambiguous,e.truth_retained,e.actions.len(),
        e.actions.iter().map(u8::to_string).collect::<Vec<_>>().join(","),str_array(&e.remaining))
}
fn render_report(head:&str,problem_hash:&str,truth_hash:&str,cases:&[Case],episodes:&[Episode])->String{
    let mut summaries=Vec::new();
    for policy in [Policy::Active.name(),Policy::Fixed.name(),Policy::Random.name()]{
        for split in ["all","train","validation","heldout"]{
            let selected:Vec<&Episode>=episodes.iter().filter(|e|e.policy==policy&&(split=="all"||e.split==split)).collect();
            summaries.push(summary(policy,split,&selected));
        }
    }
    let case_rows:Vec<String>=cases.iter().map(|c|{
        let random:Vec<&Episode>=c.random.iter().collect();
        format!("{{\"task_id\":\"{}\",\"split\":\"{}\",\"evaluator_target_model\":\"{}\",\"candidate_count\":{},\"initially_viable_candidates\":{},\"active\":{},\"fixed\":{},\"random_repetitions\":{},\"random_summary\":{}}}",
            esc(&c.id),esc(&c.split),esc(&c.active.target),c.candidate_count,c.initial_viable,
            render_episode(&c.active),render_episode(&c.fixed),c.random.len(),summary(Policy::Random.name(),&c.split,&random))
    }).collect();
    format!("{{\n  \"schema_version\":1,\n  \"benchmark_id\":\"SIM-REL-ACTIVE-DISCOVERY-001-v1\",\n  \"synthetic_only\":true,\n  \"exact_head\":\"{}\",\n  \"problem_fixture_sha256\":\"{}\",\n  \"evaluator_truth_fixture_sha256\":\"{}\",\n  \"task_count\":{},\n  \"random_repetitions_per_task\":{},\n  \"policy_summaries\":[\n    {}\n  ],\n  \"cases\":[\n    {}\n  ],\n  \"claim_ceiling\":\"finite synthetic binary causal-model selection only; no grammar discovery, physical validation, Chain Theory validation, or SOTA superiority claim\"\n}}\n",
        esc(head),problem_hash,truth_hash,cases.len(),RANDOM_REPETITIONS,summaries.join(",\n    "),case_rows.join(",\n    "))
}
fn git_head()->Result<String,String>{
    let o=Command::new("git").args(["rev-parse","HEAD"]).output().map_err(|e|format!("git command failed: {e}"))?;
    if !o.status.success(){return Err("git rev-parse HEAD failed".into());}
    String::from_utf8(o.stdout).map(|s|s.trim().to_owned()).map_err(|e|format!("git stdout is not UTF-8: {e}"))
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    let(mut problems_path,mut truth_path,mut expected,mut out)=(None,None,None,None);
    let mut args=env::args().skip(1);
    while let Some(a)=args.next(){match a.as_str(){
        "--problems"=>problems_path=args.next(),"--truth"=>truth_path=args.next(),"--expected-head"=>expected=args.next(),"--out"=>out=args.next(),
        "--help"|"-h"=>{println!("sim-rel-active-discovery --problems PATH --truth PATH --expected-head SHA --out PATH");return Ok(());},
        _=>return Err(format!("unknown argument {a:?}").into())
    }}
    let problems_path=problems_path.ok_or("--problems required")?;let truth_path=truth_path.ok_or("--truth required")?;
    let expected=expected.ok_or("--expected-head required")?;let out=out.ok_or("--out required")?;
    let head=git_head().map_err(|e|format!("identity gate: {e}"))?;
    if head!=expected{return Err(format!("exact-head mismatch: expected {expected}, got {head}").into());}
    let pb=fs::read(problems_path)?;let tb=fs::read(truth_path)?;
    let ph=sha256_hex(&pb);let th=sha256_hex(&tb);
    let ps=std::str::from_utf8(&pb)?;let ts=std::str::from_utf8(&tb)?;
    let tasks=assemble(parse_problems(ps).map_err(|e|format!("problem corpus: {e}"))?,
        parse_truth(ts).map_err(|e|format!("truth corpus: {e}"))?).map_err(|e|format!("task assembly: {e}"))?;
    let mut cases=Vec::new();let mut all=Vec::new();
    for t in &tasks{
        let p=&t.problem;let initial_count=viable(&p.candidates,&p.initial).len();
        let active=run_episode(p,&t.truth,Policy::Active,stable_seed(&p.id)).map_err(|e|format!("active {}: {e}",p.id))?;
        let fixed=run_episode(p,&t.truth,Policy::Fixed,stable_seed(&p.id)).map_err(|e|format!("fixed {}: {e}",p.id))?;
        let mut random=Vec::new();
        for i in 0..RANDOM_REPETITIONS{random.push(run_episode(p,&t.truth,Policy::Random,stable_seed(&p.id)^((i as u64)+1))
            .map_err(|e|format!("random {} run {i}: {e}",p.id))?);}
        all.push(active.clone());all.push(fixed.clone());all.extend(random.iter().cloned());
        cases.push(Case{id:p.id.clone(),split:p.split.clone(),candidate_count:p.candidates.len(),initial_viable:initial_count,active,fixed,random});
    }
    let report=render_report(&head,&ph,&th,&cases,&all);
    if out=="-"{print!("{report}");}else{fs::write(out,report)?;}
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(2);}}

#[cfg(test)]
mod tests{
    use super::*;
    const PROBLEMS:&str=include_str!("../fixtures/active-discovery-problems-v1.tsv");
    const TRUTH:&str=include_str!("../fixtures/active-discovery-truth-v1.tsv");
    fn tasks()->Vec<Task>{assemble(parse_problems(PROBLEMS).unwrap(),parse_truth(TRUTH).unwrap()).unwrap()}

    #[test]fn sha256_known_vector(){assert_eq!(sha256_hex(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");}
    #[test]fn fixture_split_counts_are_frozen(){
        let t=tasks();assert_eq!(t.len(),24);
        assert_eq!(t.iter().filter(|x|x.problem.split=="train").count(),6);
        assert_eq!(t.iter().filter(|x|x.problem.split=="validation").count(),6);
        assert_eq!(t.iter().filter(|x|x.problem.split=="heldout").count(),12);
    }
    #[test]fn active_policy_selects_a_discriminating_action(){
        let c=vec![Candidate{id:"left".into(),outputs:parse_bits("00001111").unwrap()},Candidate{id:"right".into(),outputs:parse_bits("00110011").unwrap()}];
        let mut seed=7;assert_eq!(choose_action(&c,&[0,1],&[1,2],Policy::Active,&mut seed),Some(2));
    }
    #[test]fn observationally_equivalent_models_cause_abstention(){
        let t=tasks().into_iter().find(|t|t.problem.id=="control_alias_x").unwrap();
        let e=run_episode(&t.problem,&t.truth,Policy::Active,7).unwrap();
        assert!(e.ambiguous);assert!(!e.identified);assert!(e.truth_retained);
        assert!(e.actions.is_empty()||e.remaining.len()>1);
    }
    #[test]fn all_policies_preserve_truth(){
        for t in tasks(){for p in [Policy::Active,Policy::Fixed,Policy::Random]{for seed in 1..=8{
            let e=run_episode(&t.problem,&t.truth,p,stable_seed(&t.problem.id)^seed).unwrap();
            assert!(e.truth_retained,"{} / {}",t.problem.id,p.name());
            if e.identified{assert!(e.correct,"false unique identification for {}",t.problem.id);}
        }}}
    }
    #[test]fn seeded_random_is_reproducible(){
        let t=tasks().remove(0);let a=run_episode(&t.problem,&t.truth,Policy::Random,1234).unwrap();
        let b=run_episode(&t.problem,&t.truth,Policy::Random,1234).unwrap();
        assert_eq!(a.actions,b.actions);assert_eq!(a.remaining,b.remaining);
    }
    #[test]fn malformed_signature_and_unknown_truth_fail_closed(){
        assert!(parse_candidates("bad=101").is_err());
        let p=parse_problems(PROBLEMS).unwrap();let mut truth=parse_truth(TRUTH).unwrap();
        truth.insert("train_a_x".into(),"absent_model".into());assert!(assemble(p,truth).is_err());
    }
    #[test]fn duplicate_ids_and_actions_rejected(){
        let line=PROBLEMS.lines().find(|x|x.starts_with("train_a_x|")).unwrap();
        assert!(parse_problems(&format!("{line}\n{line}")).is_err());assert!(parse_actions("1,2,2").is_err());
    }
}