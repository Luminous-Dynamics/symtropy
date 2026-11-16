use phri_tac_transfer_oracle::validate_tracked;
use std::process::ExitCode;

fn main() -> ExitCode {
    match validate_tracked() {
        Ok(report) => {
            println!("suite=phri_tac_transfer_001_v0_1");
            println!("cases={}", report.case_count);
            println!("valid_splits={}", report.valid_splits);
            println!("leaked_splits={}", report.leaked_splits);
            println!("transfer_eligible={}", report.transfer_eligible);
            println!(
                "target_qualified_estimates={}",
                report.target_qualified_estimates
            );
            println!("model_estimate_only={}", report.model_estimate_only);
            println!(
                "invalid_measurement_claims={}",
                report.invalid_measurement_claims
            );
            println!("result=all_known_answers_matched");
            println!("claim_ceiling=synthetic_transfer_contract_only");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("result=invalid");
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
