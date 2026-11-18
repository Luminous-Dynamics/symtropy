use pump_verified_service_economics::{evaluate_corpus,render_report,CASES,AREAS,EXPECTED};
use std::process::ExitCode;

fn main()->ExitCode{
    let rows=match evaluate_corpus(CASES,AREAS,EXPECTED){
        Ok(v)=>v,
        Err(e)=>{eprintln!("pump-verified-service-economics: {e}");return ExitCode::FAILURE;}
    };
    match std::env::args().nth(1).as_deref(){
        None|Some("--report")=>print!("{}",render_report(&rows)),
        Some(arg)=>{eprintln!("unsupported argument: {arg}");return ExitCode::FAILURE;}
    }
    eprintln!("scenarios={} qualified={}",rows.len(),rows.iter().filter(|r|r.disposition.as_str()=="QUALIFIED_COST_PER_SERVICE").count());
    ExitCode::SUCCESS
}
