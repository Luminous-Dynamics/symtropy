use serde_json::Value;
use std::{env,fs,process::ExitCode};
use symtropy_maritime_observability_adapter::evaluate_and_bind_execution;
fn read_json(path:&str,label:&str)->Result<Value,String>{
    let raw=fs::read_to_string(path).map_err(|e|format!("read {label}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e|format!("parse {label}: {e}"))
}
fn main()->ExitCode{
    let args:Vec<String>=env::args().collect();
    if args.len()!=10||args[2]!="--source-commit"||args[4]!="--case-sha256"
        ||args[6]!="--clock-qualification"||args[8]!="--clock-sha256"{
        eprintln!("usage: maritime-observability-adapter <case.json> --source-commit <40-hex> --case-sha256 <64-hex> --clock-qualification <clock.json> --clock-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    let case=match read_json(&args[1],"case"){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::FAILURE;}};
    let clock=match read_json(&args[7],"clock qualification"){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::FAILURE;}};
    match evaluate_and_bind_execution(&case,&clock,&args[3],&args[5],&args[9]){
        Ok(report)=>{println!("{}",serde_json::to_string_pretty(&report).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS},
        Err(errors)=>{for e in errors{eprintln!("{} {}: {}",e.code,e.path,e.detail);}ExitCode::FAILURE}
    }
}
