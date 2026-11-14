// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-DESIGN-SEARCH-001";
const BLOCK_COUNT: usize = 5;
const MAX_EXACT_BLOCKS: usize = 12;
const EVIDENCE_CEILING: &str = "synthetic independent-block subset search arithmetic only";

type Matrix = Vec<Vec<f64>>;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {key}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?.as_str().ok_or_else(|| format!("{key} must be text"))
}
fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?.as_f64().filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key} must be a finite number"))
}
fn usize_value(value: &Value, key: &str) -> Result<usize, String> {
    field(value, key)?.as_u64().and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?.as_array().ok_or_else(|| format!("{key} must be an array"))
}
fn matrix(value: &Value, key: &str, n: usize) -> Result<Matrix, String> {
    let rows = field(value, key)?.as_array().ok_or_else(|| format!("{key} must be a matrix"))?;
    if rows.len() != n { return Err(format!("{key} row count mismatch")); }
    let mut output = Vec::with_capacity(n);
    for row in rows {
        let values = row.as_array().ok_or_else(|| format!("{key} row must be an array"))?;
        if values.len() != n { return Err(format!("{key} column count mismatch")); }
        output.push(values.iter().map(|v| v.as_f64().filter(|x| x.is_finite())
            .ok_or_else(|| format!("{key} contains a non-finite value")))
            .collect::<Result<Vec<_>, _>>()?);
    }
    Ok(output)
}
fn add_matrix(left: &Matrix, right: &Matrix) -> Matrix {
    left.iter().zip(right).map(|(a,b)| a.iter().zip(b).map(|(x,y)| x+y).collect()).collect()
}
fn symmetric_eigenvalues(mut a: Matrix) -> Result<Vec<f64>, String> {
    let n=a.len();
    if n==0 || a.iter().any(|r|r.len()!=n) { return Err("eigenvalue matrix must be square".into()); }
    let scale=a.iter().flatten().map(|x|x.abs()).fold(0.0_f64,f64::max).max(1.0e-300);
    for i in 0..n { for j in 0..n {
        if (a[i][j]-a[j][i]).abs()>1.0e-10*scale { return Err("information matrix is not symmetric".into()); }
    }}
    if n==1 { return Ok(vec![a[0][0]]); }
    let mut converged=false;
    for _ in 0..(100*n*n) {
        let mut p=0usize; let mut q=1usize; let mut largest=0.0_f64;
        for i in 0..n { for j in (i+1)..n { if a[i][j].abs()>largest {largest=a[i][j].abs();p=i;q=j;} }}
        if largest<=1.0e-13*scale {converged=true;break;}
        let app=a[p][p];let aqq=a[q][q];let apq=a[p][q];
        let tau=(aqq-app)/(2.0*apq);
        let sign=if tau>=0.0 {1.0}else{-1.0};
        let t=sign/(tau.abs()+(1.0+tau*tau).sqrt());
        let c=1.0/(1.0+t*t).sqrt();let s=t*c;
        for k in 0..n {
            if k!=p&&k!=q {
                let akp=a[k][p];let akq=a[k][q];
                a[k][p]=c*akp-s*akq;a[p][k]=a[k][p];
                a[k][q]=s*akp+c*akq;a[q][k]=a[k][q];
            }
        }
        a[p][p]=app-t*apq;a[q][q]=aqq+t*apq;a[p][q]=0.0;a[q][p]=0.0;
    }
    if !converged {return Err("symmetric Jacobi eigensolver did not converge".into());}
    let mut values=(0..n).map(|i|a[i][i]).collect::<Vec<_>>();
    values.sort_by(|x,y|y.total_cmp(x));
    Ok(values)
}
fn cholesky(a:&Matrix)->Result<Matrix,String>{
    let n=a.len();
    if n==0||a.iter().any(|r|r.len()!=n){return Err("nuisance information must be square".into());}
    let scale=a.iter().enumerate().map(|(i,r)|r[i].abs()).fold(0.0_f64,f64::max);
    if scale<=0.0{return Err("nuisance information has no positive scale".into());}
    let mut l=vec![vec![0.0;n];n];
    for i in 0..n {for j in 0..=i {
        let mut s=a[i][j];
        for k in 0..j{s-=l[i][k]*l[j][k];}
        if i==j {
            if !s.is_finite()||s<=1.0e-12*scale{return Err("nuisance information is singular".into());}
            l[i][j]=s.sqrt();
        } else {l[i][j]=s/l[j][j];}
    }}
    Ok(l)
}
fn solve_spd(l:&Matrix,b:&[f64])->Result<Vec<f64>,String>{
    let n=l.len();if b.len()!=n{return Err("solve dimension mismatch".into());}
    let mut y=vec![0.0;n];
    for i in 0..n {let mut v=b[i];for j in 0..i{v-=l[i][j]*y[j];}y[i]=v/l[i][i];}
    let mut x=vec![0.0;n];
    for i in (0..n).rev(){let mut v=y[i];for j in (i+1)..n{v-=l[j][i]*x[j];}x[i]=v/l[i][i];}
    if x.iter().any(|v|!v.is_finite()){return Err("non-finite solve result".into());}
    Ok(x)
}
fn submatrix(a:&Matrix,rs:usize,re:usize,cs:usize,ce:usize)->Matrix{
    (rs..re).map(|i|(cs..ce).map(|j|a[i][j]).collect()).collect()
}
fn adjusted_target_information(f:&Matrix,target:usize)->Result<Option<Matrix>,String>{
    let p=f.len();if target==0||target>=p{return Err("target_parameter_count must be in 1..parameter_count".into());}
    let ftt=submatrix(f,0,target,0,target);
    let ftn=submatrix(f,0,target,target,p);
    let fnn=submatrix(f,target,p,target,p);
    let fnt=submatrix(f,target,p,0,target);
    let l=match cholesky(&fnn){Ok(x)=>x,Err(_)=>return Ok(None)};
    let nuisance=p-target;let mut out=ftt;
    for col in 0..target{
        let rhs=(0..nuisance).map(|r|fnt[r][col]).collect::<Vec<_>>();
        let solved=solve_spd(&l,&rhs)?;
        for row in 0..target{
            let correction=(0..nuisance).map(|k|ftn[row][k]*solved[k]).sum::<f64>();
            out[row][col]-=correction;
        }
    }
    for i in 0..target{for j in 0..i{let m=0.5*(out[i][j]+out[j][i]);out[i][j]=m;out[j][i]=m;}}
    Ok(Some(out))
}
#[derive(Clone)]
struct Metrics{
    rank:usize,min_singular:f64,condition:Option<f64>,logdet:Option<f64>,
}
fn metrics(info:&Matrix,tolerance:f64)->Result<Metrics,String>{
    let eig=symmetric_eigenvalues(info.clone())?;
    let max=eig.first().copied().unwrap_or(0.0).max(0.0);
    let singular=eig.iter().map(|v|v.max(0.0).sqrt()).collect::<Vec<_>>();
    let rank=if max<=0.0{0}else{singular.iter().filter(|s|**s>max*tolerance).count()};
    let full=rank==singular.len()&&singular.last().copied().unwrap_or(0.0)>0.0;
    let condition=if full{Some(max.sqrt()/singular[singular.len()-1])}else{None};
    let logdet=if full{Some(eig.iter().map(|v|v.ln()).sum())}else{None};
    Ok(Metrics{rank,min_singular:if full{singular[singular.len()-1]}else{0.0},condition,logdet})
}
fn regularized_joint_logdet(f:&Matrix)->Result<f64,String>{
    let mut a=f.clone();for i in 0..a.len(){a[i][i]+=1.0e-8;}
    Ok(symmetric_eigenvalues(a)?.iter().map(|v|v.max(1.0e-300).ln()).sum())
}
fn hex_digest(value:&str)->bool{
    value.len()==64&&value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
#[derive(Clone)]
struct Block{
    id:String, information:Matrix, separation:BTreeMap<String,f64>,
    cost:f64,duration:f64,calibration:f64,sensors:BTreeSet<String>,
}
fn validate(input:&Value,oracle:&Value,manifest:&Value)->Result<Vec<Block>,String>{
    if text(input,"schema_version")?!="mixing-design-search-t0-input-v0.1"{return Err("unexpected input schema".into());}
    if text(oracle,"schema_version")?!="mixing-design-search-t0-oracle-v0.1"{return Err("unexpected oracle schema".into());}
    for x in [input,oracle,manifest]{if text(x,"programme_id")?!=PROGRAMME{return Err("programme identity mismatch".into());}}
    for key in ["physical_qualification_established","potability_claim_permitted","release_authorized"]{
        if field(oracle,key)?.as_bool()!=Some(false){return Err(format!("oracle may not set {key} true"));}
    }
    let p=usize_value(input,"parameter_count")?;
    let target=usize_value(input,"target_parameter_count")?;
    if target==0||target>=p{return Err("invalid target/nuisance dimension".into());}
    let rows=array(input,"blocks")?;let expected=array(oracle,"blocks")?;
    let ids=array(manifest,"block_ids")?;
    if rows.len()!=BLOCK_COUNT||expected.len()!=BLOCK_COUNT||usize_value(manifest,"block_count")?!=BLOCK_COUNT{
        return Err("frozen block count mismatch".into());
    }
    if rows.len()>MAX_EXACT_BLOCKS{return Err("exact subset limit exceeded".into());}
    let pairs=array(input,"model_pair_ids")?.iter().map(|x|x.as_str().map(str::to_owned).ok_or_else(||"model pair id must be text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
    if pairs.is_empty(){return Err("model_pair_ids cannot be empty".into());}
    let mut out=Vec::new();let mut seen=BTreeSet::new();
    for row in rows{
        let id=text(row,"block_id")?.to_owned();
        if !seen.insert(id.clone()){return Err("duplicate block_id".into());}
        if text(row,"cross_block_noise_contract")?!="independent"{return Err("cross-block covariance unknown: independence must be explicit".into());}
        let digest=text(row,"information_receipt_sha256")?;
        if !hex_digest(digest){return Err(format!("block {id} lacks a valid information receipt digest"));}
        let mut receipt_material=row.clone();
        receipt_material.as_object_mut().ok_or_else(||"block must be object".to_owned())?
            .remove("information_receipt_sha256");
        let canonical=serde_json::to_vec(&receipt_material).map_err(|e|format!("serialize block receipt: {e}"))?;
        if sha256(&canonical)!=digest{return Err(format!("block {id} information receipt digest mismatch"));}
        let fim=matrix(row,"information_matrix",p)?;
        let eig=symmetric_eigenvalues(fim.clone())?;
        if eig.iter().any(|v|*v < -1.0e-9*eig[0].abs().max(1.0)){
            return Err(format!("block {id} information matrix is not positive semidefinite"));
        }
        let raw=field(row,"model_separation_squared")?.as_object().ok_or_else(||"model_separation_squared must be an object".to_owned())?;
        let mut separation=BTreeMap::new();
        for pair in &pairs{
            let value=raw.get(pair).and_then(Value::as_f64).filter(|v|v.is_finite()&&*v>=0.0)
                .ok_or_else(||format!("block {id} missing/invalid model pair {pair}"))?;
            separation.insert(pair.clone(),value);
        }
        let sensors=array(row,"sensor_ids")?.iter().map(|x|x.as_str().filter(|s|!s.trim().is_empty()).map(str::to_owned)
            .ok_or_else(||"sensor id must be non-empty text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
        if sensors.is_empty(){return Err(format!("block {id} has no sensor identity"));}
        out.push(Block{
            id,information:fim,separation,
            cost:number(row,"cost_units")?,duration:number(row,"duration_minutes")?,
            calibration:number(row,"calibration_units")?,sensors,
        });
    }
    let oracle_ids=expected.iter().map(|x|text(x,"block_id").map(str::to_owned)).collect::<Result<BTreeSet<_>,_>>()?;
    let manifest_ids=ids.iter().map(|x|x.as_str().map(str::to_owned).ok_or_else(||"manifest block id must be text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
    if seen!=oracle_ids||seen!=manifest_ids{return Err("input/oracle/manifest block identity mismatch".into());}
    Ok(out)
}
fn compute_subset(blocks:&[Block],indices:&[usize],p:usize,target:usize,tolerance:f64,
    budget:f64,sensor_budget:usize,duration_budget:f64,calibration_budget:f64,
    max_condition:f64,min_separation:f64)->Result<Value,String>{
    let mut f=vec![vec![0.0;p];p];
    let mut cost=0.0;let mut duration=0.0;let mut calibration=0.0;
    let mut sensors=BTreeSet::new();let mut ids=Vec::new();
    let mut separations:BTreeMap<String,f64>=BTreeMap::new();
    for &index in indices{
        let b=blocks.get(index).ok_or_else(||"subset index out of range".to_owned())?;
        f=add_matrix(&f,&b.information);cost+=b.cost;duration+=b.duration;calibration+=b.calibration;
        sensors.extend(b.sensors.iter().cloned());ids.push(b.id.clone());
        for (pair,value) in &b.separation{*separations.entry(pair.clone()).or_insert(0.0)+=*value;}
    }
    ids.sort();
    if !cost.is_finite()||!duration.is_finite()||!calibration.is_finite(){return Err("resource sum overflow".into());}
    let nuisance_adjusted=adjusted_target_information(&f,target)?;
    let nuisance_ok=nuisance_adjusted.is_some();
    let info=nuisance_adjusted.unwrap_or_else(||vec![vec![0.0;target];target]);
    let metric=metrics(&info,tolerance)?;
    let pair_separation=separations.iter().map(|(pair,sq)|(pair.clone(),sq.max(0.0).sqrt())).collect::<BTreeMap<_,_>>();
    let min_pair=pair_separation.values().copied().fold(f64::INFINITY,f64::min);
    let within=cost<=budget&&sensors.len()<=sensor_budget&&duration<=duration_budget&&calibration<=calibration_budget;
    let state=if !within{"OVER_BUDGET"}else if !nuisance_ok{"NUISANCE_UNRESOLVED"}
        else if metric.rank<target{"UNIDENTIFIABLE"}
        else if metric.condition.is_none_or(|x|x>max_condition){"ILL_CONDITIONED"}
        else if min_pair<min_separation{"MODEL_DISCRIMINATION_INSUFFICIENT"}
        else{"ELIGIBLE_DESIGN"};
    Ok(json!({
        "selected_block_ids":ids,"cost_units":cost,"unique_sensor_count":sensors.len(),"sensor_ids":sensors,
        "duration_minutes":duration,"calibration_units":calibration,"within_resource_budgets":within,
        "target_rank_after_aggregate_nuisance_adjustment":metric.rank,
        "minimum_target_singular_value":metric.min_singular,"target_condition_number":metric.condition,
        "target_log_determinant":metric.logdet,
        "pairwise_model_separation":pair_separation,"minimum_pairwise_model_separation":min_pair,
        "nuisance_adjustment_state":if nuisance_ok{"APPLIED_AFTER_AGGREGATION"}else{"NUISANCE_UNRESOLVED"},
        "subset_state":state,
        "physical_qualification_established":false,"potability_claim_permitted":false,"release_authorized":false
    }))
}
fn dominates(a:&Value,b:&Value)->bool{
    let metric=|x:&Value,k:&str,default:f64|x[k].as_f64().unwrap_or(default);
    let a_log=metric(a,"target_log_determinant",f64::NEG_INFINITY);
    let b_log=metric(b,"target_log_determinant",f64::NEG_INFINITY);
    let a_min=metric(a,"minimum_target_singular_value",0.0);
    let b_min=metric(b,"minimum_target_singular_value",0.0);
    let a_sep=metric(a,"minimum_pairwise_model_separation",0.0);
    let b_sep=metric(b,"minimum_pairwise_model_separation",0.0);
    let a_cost=metric(a,"cost_units",f64::INFINITY);
    let b_cost=metric(b,"cost_units",f64::INFINITY);
    let a_sensors=metric(a,"unique_sensor_count",f64::INFINITY);
    let b_sensors=metric(b,"unique_sensor_count",f64::INFINITY);
    let a_duration=metric(a,"duration_minutes",f64::INFINITY);
    let b_duration=metric(b,"duration_minutes",f64::INFINITY);
    let a_cal=metric(a,"calibration_units",f64::INFINITY);
    let b_cal=metric(b,"calibration_units",f64::INFINITY);
    a_log>=b_log&&a_min>=b_min&&a_sep>=b_sep&&a_cost<=b_cost&&a_sensors<=b_sensors
        &&a_duration<=b_duration&&a_cal<=b_cal
        &&(a_log>b_log||a_min>b_min||a_sep>b_sep||a_cost<b_cost||a_sensors<b_sensors||a_duration<b_duration||a_cal<b_cal)
}
fn exact_search(blocks:&[Block],p:usize,target:usize,tol:f64,budget:f64,sensor_budget:usize,
    duration_budget:f64,cal_budget:f64,max_condition:f64,min_sep:f64)->Result<Vec<Value>,String>{
    if blocks.len()>MAX_EXACT_BLOCKS{return Err("exact subset limit exceeded".into());}
    let total=1usize<<blocks.len();
    let mut results=Vec::with_capacity(total-1);
    for mask in 1..total{
        let ids=(0..blocks.len()).filter(|i|(mask&(1usize<<i))!=0).collect::<Vec<_>>();
        results.push(compute_subset(blocks,&ids,p,target,tol,budget,sensor_budget,duration_budget,cal_budget,max_condition,min_sep)?);
    }
    Ok(results)
}
fn score_joint(blocks:&[Block],indices:&[usize],p:usize)->Result<f64,String>{
    let mut f=vec![vec![0.0;p];p];
    for &i in indices{f=add_matrix(&f,&blocks[i].information);}
    regularized_joint_logdet(&f)
}
fn greedy_d_opt(blocks:&[Block],p:usize,budget:f64,sensor_budget:usize,duration_budget:f64,cal_budget:f64)->Result<Vec<usize>,String>{
    let mut selected=Vec::new();let mut remaining=(0..blocks.len()).collect::<BTreeSet<_>>();
    loop{
        let current=score_joint(blocks,&selected,p)?;
        let mut best:Option<(usize,f64)>=None;
        for &i in &remaining{
            let mut trial=selected.clone();trial.push(i);
            let cost: f64=trial.iter().map(|j|blocks[*j].cost).sum();
            let duration:f64=trial.iter().map(|j|blocks[*j].duration).sum();
            let calibration:f64=trial.iter().map(|j|blocks[*j].calibration).sum();
            let mut sensors=BTreeSet::new();for &j in &trial{sensors.extend(blocks[j].sensors.iter().cloned());}
            if cost>budget||duration>duration_budget||calibration>cal_budget||sensors.len()>sensor_budget{continue;}
            let gain=score_joint(blocks,&trial,p)?-current;
            let efficiency=gain/blocks[i].cost.max(1.0e-12);
            if best.is_none_or(|(best_i,best_score)|efficiency>best_score+1.0e-12
                ||((efficiency-best_score).abs()<=1.0e-12&&blocks[i].id<blocks[best_i].id)){
                best=Some((i,efficiency));
            }
        }
        if let Some((i,_))=best{selected.push(i);remaining.remove(&i);}else{break;}
        if selected.len()==blocks.len(){break;}
    }
    Ok(selected)
}
fn next_random(state:&mut u64)->u64{
    let mut x=*state;x^=x<<13;x^=x>>7;x^=x<<17;*state=x;x
}
fn random_search(blocks:&[Block],p:usize,target:usize,tol:f64,budget:f64,sensor_budget:usize,
    duration_budget:f64,cal_budget:f64,max_condition:f64,min_sep:f64,seed:u64,trials:usize)->Result<Option<Value>,String>{
    let mut state=seed.max(1);let mut best:Option<Value>=None;
    for _ in 0..trials{
        let mut order=(0..blocks.len()).collect::<Vec<_>>();
        for i in (1..order.len()).rev(){let j=(next_random(&mut state) as usize)%(i+1);order.swap(i,j);}
        let mut chosen=Vec::new();
        for i in order{
            let mut trial=chosen.clone();trial.push(i);
            let cost:f64=trial.iter().map(|j|blocks[*j].cost).sum();
            let duration:f64=trial.iter().map(|j|blocks[*j].duration).sum();
            let calibration:f64=trial.iter().map(|j|blocks[*j].calibration).sum();
            let mut sensors=BTreeSet::new();for &j in &trial{sensors.extend(blocks[j].sensors.iter().cloned());}
            if cost<=budget&&duration<=duration_budget&&calibration<=cal_budget&&sensors.len()<=sensor_budget{chosen.push(i);}
        }
        if chosen.is_empty(){continue;}
        let result=compute_subset(blocks,&chosen,p,target,tol,budget,sensor_budget,duration_budget,cal_budget,max_condition,min_sep)?;
        if result["subset_state"]=="ELIGIBLE_DESIGN"{
            let replace=best.as_ref().is_none_or(|old|{
                let score=|v:&Value|v["target_log_determinant"].as_f64().unwrap_or(f64::NEG_INFINITY);
                score(&result)>score(old)+1.0e-12
            });
            if replace{best=Some(result);}
        }
    }
    Ok(best)
}
fn expected_matches(actual:&Value,expected:&Value)->bool{
    match (actual,expected){
        (Value::Number(a),Value::Number(b))=>matches!((a.as_f64(),b.as_f64()),(Some(x),Some(y)) if (x-y).abs()<=1.0e-8*x.abs().max(y.abs()).max(1.0)),
        (Value::Array(a),Value::Array(b))=>a.len()==b.len()&&a.iter().zip(b).all(|(x,y)|expected_matches(x,y)),
        (Value::Object(a),Value::Object(b))=>a.len()==b.len()&&a.iter().all(|(k,v)|b.get(k).is_some_and(|w|expected_matches(v,w))),
        _=>actual==expected
    }
}
pub fn evaluate(input:&Value,oracle:&Value,manifest:&Value)->Result<Value,String>{
    let blocks=validate(input,oracle,manifest)?;
    let p=usize_value(input,"parameter_count")?;let target=usize_value(input,"target_parameter_count")?;
    let tol=number(input,"rank_relative_tolerance")?;
    let budget=number(input,"cost_budget")?;
    let sensor_budget=usize_value(input,"sensor_budget")?;
    let duration_budget=number(input,"duration_budget_minutes")?;
    let cal_budget=number(input,"calibration_budget")?;
    let max_condition=number(input,"maximum_condition_number")?;
    let min_sep=number(input,"minimum_model_separation")?;
    let trials=usize_value(input,"random_trials")?;
    let seed=field(input,"random_seed")?.as_u64().ok_or_else(||"random_seed must be u64".to_owned())?;
    if tol<=0.0||tol>=0.1||budget<0.0||duration_budget<0.0||cal_budget<0.0||max_condition<=1.0||min_sep<0.0||trials==0{
        return Err("invalid threshold or budget".into());
    }
    let mut subsets=exact_search(&blocks,p,target,tol,budget,sensor_budget,duration_budget,cal_budget,max_condition,min_sep)?;
    subsets.sort_by(|a,b|{
        let ai=a["selected_block_ids"].as_array().unwrap();let bi=b["selected_block_ids"].as_array().unwrap();
        ai.len().cmp(&bi.len()).then_with(||ai.cmp(bi))
    });
    let eligible=subsets.iter().filter(|x|x["subset_state"]=="ELIGIBLE_DESIGN").cloned().collect::<Vec<_>>();
    let pareto=eligible.iter().enumerate().filter(|(i,x)|!eligible.iter().enumerate().any(|(j,y)|i!=&j&&dominates(y,x)))
        .map(|(_,x)|x.clone()).collect::<Vec<_>>();
    let exact_best=eligible.iter().max_by(|a,b|{
        a["target_log_determinant"].as_f64().unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&b["target_log_determinant"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .then_with(||b["cost_units"].as_f64().unwrap_or(f64::INFINITY).total_cmp(&a["cost_units"].as_f64().unwrap_or(f64::INFINITY)))
    }).cloned();
    let greedy_indices=greedy_d_opt(&blocks,p,budget,sensor_budget,duration_budget,cal_budget)?;
    let greedy=if greedy_indices.is_empty(){None}else{Some(compute_subset(&blocks,&greedy_indices,p,target,tol,budget,sensor_budget,duration_budget,cal_budget,max_condition,min_sep)?)};
    let random=random_search(&blocks,p,target,tol,budget,sensor_budget,duration_budget,cal_budget,max_condition,min_sep,seed,trials)?;
    let expected=field(oracle,"expected")?;
    let exact_best_ids=field(expected,"exact_best_block_ids")?;
    let exact_matches=exact_best.as_ref().is_some_and(|x|expected_matches(&x["selected_block_ids"],exact_best_ids));
    let eligible_ids=eligible.iter().map(|x|x["selected_block_ids"].clone()).collect::<Vec<_>>();
    let expected_eligible=field(expected,"exact_eligible_subset_ids")?;
    let eligible_matches=expected_matches(&json!(eligible_ids),expected_eligible);
    let pareto_ids=pareto.iter().map(|x|x["selected_block_ids"].clone()).collect::<Vec<_>>();
    let expected_pareto=field(expected,"pareto_frontier_block_sets")?;
    let pareto_matches=expected_matches(&json!(pareto_ids),expected_pareto);
    let greedy_state=greedy.as_ref().map(|x|x["subset_state"].clone()).unwrap_or(Value::Null);
    let greedy_ids=greedy.as_ref().map(|x|x["selected_block_ids"].clone()).unwrap_or(Value::Null);
    let greedy_matches=expected_matches(&greedy_state,field(expected,"greedy_expected_state")?)
        &&expected_matches(&greedy_ids,field(expected,"greedy_expected_block_ids")?);
    let random_ids=random.as_ref().map(|x|x["selected_block_ids"].clone()).unwrap_or(Value::Null);
    let random_state=random.as_ref().map(|x|x["subset_state"].clone()).unwrap_or(Value::Null);
    let random_matches=expected_matches(&random_ids,field(expected,"random_best_block_ids")?)
        &&expected_matches(&random_state,field(expected,"random_best_state")?);
    Ok(json!({
        "schema_version":"mixing-design-search-t0-evaluation-v0.1",
        "verification_state":if exact_matches&&eligible_matches&&pareto_matches&&greedy_matches&&random_matches{"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "candidate_block_count":blocks.len(),"enumerated_nonempty_subsets":subsets.len(),
        "eligible_subset_count":eligible.len(),"pareto_frontier":pareto,
        "exact_best_by_nuisance_adjusted_D_optimality":exact_best,
        "greedy_joint_D_optimality_subset":greedy,
        "random_feasible_best":random,
        "random_seed":seed,"random_trials":trials,
        "oracle_checks":{"exact_best_subset_match":exact_matches,"all_eligible_subsets_match":eligible_matches,"pareto_frontier_match":pareto_matches,"greedy_subset_match":greedy_matches,"random_subset_match":random_matches},
        "all_subset_results":subsets,
        "nonclaims":[
            "Information matrices are precomputed sufficient statistics; this search does not validate their upstream sensitivity/covariance derivation.",
            "Cross-block measurement independence is an explicit assumption; correlated blocks require a joint covariance and are rejected here.",
            "The exhaustive oracle applies only to this finite synthetic candidate pool and declared limits.",
            "No result qualifies physical mixing, a sensor, tracer, tank, junction, potability or release authority."
        ]
    }))
}
fn sha256(bytes:&[u8])->String{Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn read_json(path:&Path)->Result<(Vec<u8>,Value),String>{
    let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
    Ok((bytes,value))
}
fn valid_revision(value:&str)->bool{value.len()==40&&value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn run()->Result<i32,String>{
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4{return Err("usage: mixing-design-search-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let ip=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-search-t0-cases-v0.1.json"));
    let op=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-search-t0-oracle-v0.1.json"));
    let mp=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-search-t0-manifest-v0.1.json"));
    let rp=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-design-search-t0/receipt.json"));
    let (ib,iv)=read_json(&ip)?;let (ob,ov)=read_json(&op)?;let (mb,mv)=read_json(&mp)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let success=evaluation["verification_state"]=="MatchedFrozenOracle";
    let revision=env::var("WATER_MIXING_SEARCH_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|v|valid_revision(v));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"mixing-design-search-t0-run-v0.1",
        "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
        "implementation_revision":revision,
        "input_sha256":sha256(&ib),"oracle_sha256":sha256(&ob),"manifest_sha256":sha256(&mb),
        "cargo_lock_sha256":sha256(&lock),
        "case_count":evaluation["enumerated_nonempty_subsets"],
        "eligible_subset_count":evaluation["eligible_subset_count"],
        "oracle_checks":evaluation["oracle_checks"],
        "exact_best":evaluation["exact_best_by_nuisance_adjusted_D_optimality"],
        "greedy":evaluation["greedy_joint_D_optimality_subset"],
        "random_best":evaluation["random_feasible_best"],
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "physical_qualification_established":false,"potability_claim_permitted":false,"release_authorized":false
    });
    if let Some(parent)=rp.parent(){if !parent.as_os_str().is_empty(){fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;}}
    let bytes=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
    fs::write(&rp,bytes.as_bytes()).map_err(|e|format!("write {}: {e}",rp.display()))?;
    println!("{bytes}");
    Ok(if success{0}else{2})
}
fn main(){match run(){Ok(code)=>std::process::exit(code),Err(e)=>{eprintln!("WATER_MIXING_SEARCH_T0_ERROR: {e}");std::process::exit(1);}}}

#[cfg(test)]
mod tests{
    use super::*;
    fn load(name:&str)->Value{
        let raw=match name{
            "input"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-search-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-search-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-search-t0-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(raw).expect("search fixture parses")
    }
    fn result()->Value{evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("search evaluator runs")}
    #[test]fn exact_enumeration_checks_all_nonempty_subsets(){assert_eq!(result()["enumerated_nonempty_subsets"],31);}
    #[test]fn exact_search_finds_model_discriminating_plan(){assert_eq!(result()["exact_best_by_nuisance_adjusted_D_optimality"]["selected_block_ids"],json!(["A-inlet-one","B-inlet-two","C-nuisance-one-calibration","D-nuisance-two-calibration"]));}
    #[test]fn d_optimal_greedy_is_trapped_by_high_information_low_discrimination_block(){assert_eq!(result()["greedy_joint_D_optimality_subset"]["selected_block_ids"],json!(["E-high-information-low-discrimination"]));assert_eq!(result()["greedy_joint_D_optimality_subset"]["subset_state"],"MODEL_DISCRIMINATION_INSUFFICIENT");}
    #[test]fn nuisance_information_is_aggregated_before_schur_adjustment(){
        let exact=result()["exact_best_by_nuisance_adjusted_D_optimality"].clone();
        assert_eq!(exact["nuisance_adjustment_state"],"APPLIED_AFTER_AGGREGATION");
        assert!((exact["minimum_target_singular_value"].as_f64().unwrap()-0.5_f64.sqrt()).abs()<1.0e-9);
    }
    #[test]fn individually_singular_nuisance_blocks_combine_into_estimable_nuisance_information(){
        let blocks=validate(&load("input"),&load("oracle"),&load("manifest")).unwrap();
        let indices=vec![0,1,2,3];
        let combo=compute_subset(&blocks,&indices,4,2,1.0e-8,4.0,4,40.0,4.0,100.0,0.35).unwrap();
        assert_eq!(combo["nuisance_adjustment_state"],"APPLIED_AFTER_AGGREGATION");
        assert_eq!(combo["subset_state"],"ELIGIBLE_DESIGN");
    }
    #[test]fn shared_sensor_ids_are_counted_once(){
        let exact=result()["exact_best_by_nuisance_adjusted_D_optimality"].clone();
        assert_eq!(exact["unique_sensor_count"],3);
    }
    #[test]fn random_baseline_is_deterministic_and_seeded(){
        let first=result();let second=result();
        assert_eq!(first["random_feasible_best"]["selected_block_ids"],second["random_feasible_best"]["selected_block_ids"]);
    }
    #[test]fn exact_oracle_checks_all_eligible_subsets_and_pareto_membership(){
        let v=result();
        assert_eq!(v["oracle_checks"]["all_eligible_subsets_match"],true);
        assert_eq!(v["oracle_checks"]["pareto_frontier_match"],true);
    }
    #[test]fn information_block_receipt_digest_is_recomputed(){
        let mut input=load("input");
        input["blocks"][0]["information_matrix"][0][0]=json!(2.0);
        assert!(validate(&input,&load("oracle"),&load("manifest")).is_err());
    }
    #[test]fn all_selected_blocks_require_independent_cross_block_noise_contract(){
        let mut input=load("input");input["blocks"][1]["cross_block_noise_contract"]=json!("unknown");
        assert!(validate(&input,&load("oracle"),&load("manifest")).is_err());
    }
    #[test]fn exact_search_refuses_oversized_candidate_pools(){
        let mut input=load("input");input["blocks"]=json!(vec![input["blocks"][0].clone();13]);
        assert!(validate(&input,&load("oracle"),&load("manifest")).is_err());
    }
    #[test]fn resource_budgets_are_hard_constraints(){
        let exact=result()["exact_best_by_nuisance_adjusted_D_optimality"].clone();
        assert_eq!(exact["within_resource_budgets"],true);
        assert!(exact["cost_units"].as_f64().unwrap()<=4.0);
        assert!(exact["duration_minutes"].as_f64().unwrap()<=40.0);
    }
}
