// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use water_service_t0::water_matched_experiment::evaluate;

fn sha256(bytes:&[u8])->String{Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn read_json(path:&PathBuf)->Result<(Vec<u8>,Value),String>{
 let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
 let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
 Ok((bytes,value))
}
fn valid_revision(v:&str)->bool{v.len()==40&&v.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn run()->Result<i32,String>{
 let args:Vec<String>=env::args().skip(1).collect();
 if args.len()>4{return Err("usage: water-matched-experiment [PAIRS] [SCHEMA] [ORACLE] [RECEIPT]".to_owned());}
 let pairs_path=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-service-engineering/matched-experiments/pairs-v0.1.json"));
 let schema_path=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-service-engineering/matched-experiments/pairs-v0.1.schema.json"));
 let oracle_path=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-service-engineering/matched-experiments/admission-oracle-v0.1.json"));
 let receipt_path=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-matched-experiment/receipt.json"));
 let (pairs_bytes,pairs)=read_json(&pairs_path)?;
 let (schema_bytes,schema)=read_json(&schema_path)?;
 let (oracle_bytes,oracle)=read_json(&oracle_path)?;
 let evaluation=evaluate(&pairs,&schema,&oracle).map_err(|e|e.join("\n"))?;
 let root=PathBuf::from(env!("CARGO_MANIFEST_DIR"));
 let manifest_path=root.join("Cargo.toml");
 let lock_path=root.join("../../Cargo.lock");
 let manifest=fs::read(&manifest_path).map_err(|e|format!("read {}: {e}",manifest_path.display()))?;
 let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
 let revision=env::var("WATER_MATCHED_EXPERIMENT_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|v|valid_revision(v));
 let receipt=json!({
  "schema_version":"water-matched-experiment-run-v0.1",
  "verification_state":evaluation["verification_state"],
  "qualification_status":"NotRun",
  "implementation_revision":revision,
  "pairs_sha256":sha256(&pairs_bytes),
  "schema_sha256":sha256(&schema_bytes),
  "oracle_sha256":sha256(&oracle_bytes),
  "cargo_manifest_sha256":sha256(&manifest),
  "cargo_lock_sha256":sha256(&lock),
  "evaluation":evaluation
 });
 if let Some(parent)=receipt_path.parent(){if !parent.as_os_str().is_empty(){fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;}}
 let serialized=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
 fs::write(&receipt_path,serialized.as_bytes()).map_err(|e|format!("write {}: {e}",receipt_path.display()))?;
 println!("{serialized}");
 Ok(0)
}
fn main(){match run(){Ok(code)=>std::process::exit(code),Err(e)=>{eprintln!("WATER_MATCHED_EXPERIMENT_ERROR: {e}");std::process::exit(1);}}}
