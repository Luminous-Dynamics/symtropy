use serde_json::Value;
use std::{env,fs,process::ExitCode};
use symtropy_maritime_observability_adapter::evaluate_and_bind_execution;
fn main()->ExitCode{
    let args:Vec<String>=env::args().collect();
    if args.len()!=6||args[2]!="--source-commit"||args[4]!="--case-sha256"{
        eprintln!("usage: maritime-observability-adapter <case.json> --source-commit <40-hex> --case-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    let raw=match fs::read_to_string(&args[1]){Ok(v)=>v,Err(e)=>{eprintln!("read case: {e}");return ExitCode::FAILURE;}};
    let case:Value=match serde_json::from_str(&raw){Ok(v)=>v,Err(e)=>{eprintln!("parse case: {e}");return ExitCode::FAILURE;}};
    match evaluate_and_bind_execution(&case,&args[3],&args[5]){
        Ok(report)=>{println!("{}",serde_json::to_string_pretty(&report).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS},
        Err(errors)=>{for e in errors{eprintln!("{} {}: {}",e.code,e.path,e.detail);}ExitCode::FAILURE}
    }
}
