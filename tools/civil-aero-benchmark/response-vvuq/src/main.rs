use aero_response_vvuq::{evaluate, ResponseVectorRequest};

const FIXTURE: &str =
    include_str!("../data/known-answer-response-vector-v0.1.json");

fn main() {
    let request: ResponseVectorRequest = match serde_json::from_str(FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("response_vvuq_fixture_error={error}");
            std::process::exit(2);
        }
    };
    let result = match evaluate(&request) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("response_vvuq_error={error}");
            std::process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("response_vvuq_serialize_error={error}");
            std::process::exit(2);
        }
    }
}
