use ccx_mms_p1_element_oracle::run_geometry_conditioning_sweep;
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

    let sweep = match run_geometry_conditioning_sweep() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("conditioning sweep failed to execute: {error}");
            return ExitCode::FAILURE;
        }
    };
    let geometry_conditioning_pass = sweep.windows(2).all(|pair| {
        pair[1].mean_ratio_quality < pair[0].mean_ratio_quality
            && pair[1].dimensionless_gradient_amplification
                > pair[0].dimensionless_gradient_amplification
            && pair[1].element_nonrigid_condition_number
                > pair[0].element_nonrigid_condition_number
    });
    let records = sweep
        .iter()
        .map(|point| {
            format!(
                concat!(
                    "{{\"height\":{:.17e},",
                    "\"mean_ratio_quality\":{:.17e},",
                    "\"dimensionless_gradient_amplification\":{:.17e},",
                    "\"element_nonrigid_condition_number\":{:.17e}}}"
                ),
                point.height,
                point.mean_ratio_quality,
                point.dimensionless_gradient_amplification,
                point.element_nonrigid_condition_number
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-geometry-conditioning-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_executed\":false,",
            "\"qualification_pass\":false,",
            "\"geometry_conditioning_pass\":{},",
            "\"sweep\":[{}]}}"
        ),
        source_sha,
        checkout_sha,
        geometry_conditioning_pass,
        records
    );
    if geometry_conditioning_pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
