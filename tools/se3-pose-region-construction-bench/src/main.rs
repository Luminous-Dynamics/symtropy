use se3_pose_region_construction_bench::{
    ConformalProfile, RuntimePoseReceipt, RECEIPT_SCHEMA,
    evaluate_manifest,
};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;

const HEADER: &str = "case_id,expected,mutation,sample_id,tx,ty,tz,translation_variance,rotation_variance,cross_covariance,quantile_q32_32,sample_count,order_statistic_rank,heldout_coverage_lower_ci_ppm,required_coverage_lower_ci_ppm";

struct Case {
    expected: String,
    input: RuntimePoseReceipt,
}

fn parse<T: std::str::FromStr>(value: &str, name: &str) -> Result<T, Box<dyn Error>>
where
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn make_case(fields: &[&str], line: usize) -> Result<Case, Box<dyn Error>> {
    if fields.len() != 15 {
        return Err(format!("line {line}: expected 15 fields, got {}", fields.len()).into());
    }
    let case_id = fields[0];
    let mutation = fields[2];
    let sample_id = fields[3];
    let tx = parse(fields[4], "tx")?;
    let ty = parse(fields[5], "ty")?;
    let tz = parse(fields[6], "tz")?;
    let translation_variance: f64 = parse(fields[7], "translation_variance")?;
    let rotation_variance: f64 = parse(fields[8], "rotation_variance")?;
    let cross_covariance: f64 = parse(fields[9], "cross_covariance")?;
    let quantile: u64 = parse(fields[10], "quantile_q32_32")?;
    let sample_count: u64 = parse(fields[11], "sample_count")?;
    let order_rank: u64 = parse(fields[12], "order_statistic_rank")?;
    let coverage_lcb: u32 = parse(fields[13], "heldout_coverage_lower_ci_ppm")?;
    let required_lcb: u32 = parse(fields[14], "required_coverage_lower_ci_ppm")?;

    let profile = ConformalProfile {
        profile_id: "profile-1".to_owned(),
        profile_revision: "r1".to_owned(),
        profile_sha256: digest('a'),
        calibration_scores_sha256: digest('b'),
        profile_state: "CURRENT".to_owned(),
        dataset_sha256: digest('c'),
        model_sha256: digest('d'),
        model_config_sha256: digest('e'),
        camera_calibration_sha256: digest('f'),
        target_population_sha256: digest('1'),
        calibration_split_sha256: digest('2'),
        heldout_split_sha256: digest('3'),
        calibration_method: "SPLIT_CONFORMAL_SE3-v1".to_owned(),
        score_definition: "SQRT_MAHALANOBIS_BODY_LOCAL_SE3_V1".to_owned(),
        tangent_convention: "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ".to_owned(),
        perturbation_convention: "RIGHT_COMPOSED_BODY_LOCAL_SE3_EXP_V1".to_owned(),
        nominal_level_bps: 9_500,
        sample_count,
        order_statistic_rank: order_rank,
        quantile_radius_q32_32: quantile,
        heldout_coverage_lower_ci_ppm: coverage_lcb,
        required_coverage_lower_ci_ppm: required_lcb,
        source_bytes_authenticated: true,
    };

    let mut covariance = [0.0_f64; 36];
    for axis in 0..3 {
        covariance[axis * 6 + axis] = translation_variance;
        covariance[(axis + 3) * 6 + axis + 3] = rotation_variance;
    }
    covariance[3] = cross_covariance;
    covariance[18] = cross_covariance;

    let mut input = RuntimePoseReceipt {
        case_id: case_id.to_owned(),
        sample_id: sample_id.to_owned(),
        frame_id: format!("frame-{case_id}"),
        receipt_sha256: digest('4'),
        disposition: "CANDIDATE_STATISTICAL_REGION".to_owned(),
        candidate_only: true,
        deterministic_safety_bound: false,
        maneuver_authority_granted: false,
        upstream_source_bytes_authenticated: true,
        profile_id: "profile-1".to_owned(),
        profile_sha256: digest('a'),
        dataset_sha256: digest('c'),
        model_sha256: digest('d'),
        model_config_sha256: digest('e'),
        camera_calibration_sha256: digest('f'),
        target_population_sha256: digest('1'),
        frame_convention: "WORLD_RIGHT_HANDED_METRES".to_owned(),
        quaternion_order: "XYZW".to_owned(),
        tangent_convention: "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ".to_owned(),
        translation_unit: "metre".to_owned(),
        rotation_unit: "radian".to_owned(),
        nominal_level_bps: 9_500,
        predicted_translation_m: [tx, ty, tz],
        predicted_quaternion_xyzw: [0.0, 0.0, 0.0, 1.0],
        covariance_row_major: covariance,
        profile,
    };

    match mutation {
        "none" => {}
        "weak_coverage" => input.profile.heldout_coverage_lower_ci_ppm = 899_999,
        "stale_profile" => input.profile.profile_state = "STALE".to_owned(),
        "insufficient_samples" => {
            input.profile.sample_count = 99;
            input.profile.order_statistic_rank = 95;
        }
        "wrong_rank" => input.profile.order_statistic_rank = 950,
        "missing_auth" => input.upstream_source_bytes_authenticated = false,
        "profile_auth_missing" => input.profile.source_bytes_authenticated = false,
        "not_candidate" => {
            input.disposition = "ABSTAIN_UNOBSERVABLE".to_owned();
            input.candidate_only = false;
        }
        "authority_escalation" => input.deterministic_safety_bound = true,
        "profile_digest_mismatch" => input.profile_sha256 = digest('9'),
        "dataset_scope_mismatch" => input.profile.dataset_sha256 = digest('9'),
        "wrong_score" => input.profile.score_definition = "GAUSSIAN_CHI_SQUARE".to_owned(),
        "wrong_tangent" => input.profile.tangent_convention = "WORLD_TANGENT".to_owned(),
        "wrong_perturbation" => input.profile.perturbation_convention = "LEFT_COMPOSED".to_owned(),
        "wrong_level" => {
            input.nominal_level_bps = 9_000;
            input.profile.nominal_level_bps = 9_000;
        }
        "bad_quantile" => input.profile.quantile_radius_q32_32 = u64::MAX,
        "invalid_quaternion" => input.predicted_quaternion_xyzw = [0.0; 4],
        "asymmetric_covariance" => input.covariance_row_major[3] = 1.0e-6,
        "non_spd_covariance" => input.covariance_row_major[0] = -1.0,
        "singular_covariance" => input.covariance_row_major[0] = 0.0,
        "nonfinite_covariance" => input.covariance_row_major[0] = f64::NAN,
        "overflow_translation" => {
            for axis in 0..3 {
                input.covariance_row_major[axis * 6 + axis] = 1.0e308;
            }
        }
        "negative_quaternion" => input.predicted_quaternion_xyzw = [0.0, 0.0, 0.0, -1.0],
        "wide_orientation" => {
            for axis in 3..6 {
                input.covariance_row_major[axis * 6 + axis] = 9.0;
            }
        }
        "bad_split_identity" => {
            input.profile.heldout_split_sha256 = input.profile.calibration_split_sha256.clone();
        }
        "bad_digest" => input.receipt_sha256 = "bad".to_owned(),
        other => return Err(format!("line {line}: unknown mutation {other}").into()),
    }

    Ok(Case {
        expected: fields[1].to_owned(),
        input,
    })
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => out.push('?'),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn result_json(
    result: &se3_pose_region_construction_bench::RegionResult,
    expected: &str,
) -> String {
    let mut out = String::new();
    let region = match result.region {
        Some(region) => {
            let bounds = region.translation_bounds_um;
            format!(
                "{{\"translation_bounds_um\":[[{},{}],[{},{}],[{},{}]],\"orientation_cap_urad\":{}}}",
                bounds[0][0], bounds[0][1], bounds[1][0], bounds[1][1],
                bounds[2][0], bounds[2][1], region.orientation_cap_urad
            )
        }
        None => "null".to_owned(),
    };
    write!(
        out,
        "{{\"case_id\":{},\"sample_id\":{},\"frame_id\":{},\"expected_disposition\":{},\"disposition\":{},\"reason_code\":{},\"region\":{},\"region_is_statistical\":{},\"outer_approximation_of_joint_tangent_region\":{},\"source_authentication_verified_here\":{},\"source_bytes_authenticated\":{},\"physical_calibration_validated\":{},\"deterministic_safety_bound\":{},\"maneuver_authority_granted\":{},\"runtime_receipt_sha256\":{},\"profile_id\":{},\"profile_sha256\":{},\"calibration_scores_sha256\":{},\"nominal_level_bps\":{},\"calibration_sample_count\":{},\"order_statistic_rank\":{},\"quantile_radius_q32_32\":{}}}",
        json_string(&result.case_id),
        json_string(&result.sample_id),
        json_string(&result.frame_id),
        json_string(expected),
        json_string(result.disposition.as_str()),
        json_string(&result.reason_code),
        region,
        result.region_is_statistical,
        result.outer_approximation_of_joint_tangent_region,
        result.source_authentication_verified_here,
        result.source_bytes_authenticated,
        result.physical_calibration_validated,
        result.deterministic_safety_bound,
        result.maneuver_authority_granted,
        json_string(&result.runtime_receipt_sha256),
        json_string(&result.profile_id),
        json_string(&result.profile_sha256),
        json_string(&result.calibration_scores_sha256),
        result.nominal_level_bps,
        result.calibration_sample_count,
        result.order_statistic_rank,
        result.quantile_radius_q32_32
    )
    .expect("writing to String cannot fail");
    out
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: se3-pose-region-construction-bench <corpus.csv> <source-commit> <corpus-sha256>".into());
    }
    let source_commit = &args[2];
    let corpus_sha256 = &args[3];
    if source_commit.trim().is_empty()
        || corpus_sha256.len() != 64
        || !corpus_sha256.bytes().all(|value| value.is_ascii_hexdigit())
    {
        return Err("invalid source commit or corpus SHA-256 argument".into());
    }

    let contents = fs::read_to_string(&args[1])?;
    let mut lines = contents.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected corpus header".into());
    }
    let mut cases = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        cases.push(make_case(&fields, index + 2)?);
    }
    if cases.is_empty() {
        return Err("empty corpus".into());
    }

    let inputs: Vec<_> = cases.iter().map(|case| case.input.clone()).collect();
    let outputs = evaluate_manifest(&inputs);
    let mut result_rows = Vec::with_capacity(cases.len());
    let mut matched = 0usize;
    for (case, result) in cases.iter().zip(outputs.iter()) {
        if case.expected == result.disposition.as_str() {
            matched += 1;
        }
        result_rows.push(result_json(result, &case.expected));
    }

    let claim_ready = false;
    let mut output = String::new();
    write!(
        output,
        "{{\"schema\":{},\"source_commit\":{},\"corpus_sha256\":{},\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":{},\"claim_ready\":{},\"source_bytes_authenticated\":false,\"physical_calibration_validated\":false,\"deterministic_safety_bound\":false,\"maneuver_authority_granted\":false,\"results\":[{}]}}",
        json_string(RECEIPT_SCHEMA),
        json_string(source_commit),
        json_string(corpus_sha256),
        cases.len(),
        matched,
        cases.len() - matched,
        claim_ready,
        result_rows.join(",")
    )
    .expect("writing to String cannot fail");

    println!("{output}");
    if matched != cases.len() {
        return Err(format!(
            "{} of {} expected dispositions matched",
            matched,
            cases.len()
        )
        .into());
    }
    Ok(())
}
