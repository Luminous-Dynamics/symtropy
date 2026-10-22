use ccx_mms_p1_element_oracle::run_material_conditioning_sweep;
use std::env;
use std::process::ExitCode;

fn is_lowercase_sha40(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn main() -> ExitCode {
    let source_sha = env::var("MMS_BENCH_SOURCE_SHA").unwrap_or_default();
    let checkout_sha = env::var("MMS_BENCH_CHECKOUT_SHA").unwrap_or_default();
    if !is_lowercase_sha40(&source_sha) {
        eprintln!("MMS_BENCH_SOURCE_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }
    if !is_lowercase_sha40(&checkout_sha) {
        eprintln!("MMS_BENCH_CHECKOUT_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }

    let sweep = match run_material_conditioning_sweep() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("material conditioning sweep failed to execute: {error}");
            return ExitCode::FAILURE;
        }
    };
    let material_conditioning_pass = sweep.windows(2).all(|pair| {
        pair[1].poisson_ratio > pair[0].poisson_ratio
            && pair[1].lambda_over_mu > pair[0].lambda_over_mu
            && pair[1].element_nonrigid_condition_number
                > pair[0].element_nonrigid_condition_number
    });
    let records = sweep
        .iter()
        .map(|point| {
            format!(
                concat!(
                    "{{\"poisson_ratio\":{:.17e},",
                    "\"lambda_over_mu\":{:.17e},",
                    "\"smallest_positive_eigenvalue\":{:.17e},",
                    "\"largest_positive_eigenvalue\":{:.17e},",
                    "\"element_nonrigid_condition_number\":{:.17e}}}"
                ),
                point.poisson_ratio,
                point.lambda_over_mu,
                point.smallest_positive_eigenvalue,
                point.largest_positive_eigenvalue,
                point.element_nonrigid_condition_number
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-material-conditioning-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_executed\":false,",
            "\"volumetric_locking_measured\":false,",
            "\"qualification_pass\":false,",
            "\"material_conditioning_pass\":{},",
            "\"sweep\":[{}]}}"
        ),
        source_sha,
        checkout_sha,
        material_conditioning_pass,
        records
    );
    if material_conditioning_pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
