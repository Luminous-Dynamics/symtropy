#![forbid(unsafe_code)]

use construction_vda5050_observer::{TopicIdentity, decode_vda_message};
use serde_json::json;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args[1] != "decode" {
        eprintln!(
            "usage: vda5050-observe decode TOPIC_PATH PAYLOAD_FILE [--manufacturer NAME --serial-number SERIAL]"
        );
        std::process::exit(2);
    }

    let mut manufacturer: Option<String> = None;
    let mut serial_number: Option<String> = None;
    let mut index = 4;
    while index < args.len() {
        match args[index].as_str() {
            "--manufacturer" if index + 1 < args.len() => {
                manufacturer = Some(args[index + 1].clone());
                index += 2;
            }
            "--serial-number" if index + 1 < args.len() => {
                serial_number = Some(args[index + 1].clone());
                index += 2;
            }
            other => {
                eprintln!("unknown or incomplete argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let identity = match (manufacturer, serial_number) {
        (Some(manufacturer), Some(serial_number)) => Some(TopicIdentity {
            manufacturer,
            serial_number,
        }),
        (None, None) => None,
        _ => {
            eprintln!("--manufacturer and --serial-number must be provided together");
            std::process::exit(2);
        }
    };

    let payload = match fs::read(&args[3]) {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("{}", json!({"status":"rejected","error":error.to_string()}));
            std::process::exit(1);
        }
    };

    match decode_vda_message(&args[2], identity, &payload) {
        Ok(decoded) => println!("{}", decoded.summary_json()),
        Err(error) => {
            eprintln!("{}", json!({"status":"rejected","error":error.to_string()}));
            std::process::exit(1);
        }
    }
}
