//! NIST AMB2025-07 per-pixel TAM/SCR measurement-operator reference.
//!
//! Input is an already calibrated temperature-vs-time trace. This module does
//! not perform radiometric conversion, emissivity correction, spatter removal,
//! or timebase/calibration authentication.
//!
//! NIST v3 description: TAM is the longest continuous interval with T>1298 C.
//! SCR uses 1260 C and 1150 C crossings from the same selected peak.
//! The source prints dt=t1260-t1150, but that is negative for a cooling trace;
//! this implementation requires the physically positive elapsed interval
//! t1150-t1260 and reports the source sign ambiguity explicitly.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/ambench-2025/tam_scr_pixel_operator.rs -o /tmp/amb07-pixel-tests
//!   /tmp/amb07-pixel-tests
//!   rustc --edition=2024 -D warnings tools/ambench-2025/tam_scr_pixel_operator.rs -o /tmp/amb07-pixel
//!   /tmp/amb07-pixel --verify
//!   /tmp/amb07-pixel --reference

const TAM_THRESHOLD_C: f64 = 1298.0;
const SOLIDUS_C: f64 = 1260.0;
const SCR_LOWER_C: f64 = 1150.0;
const SCR_DELTA_C: f64 = 110.0;
const FRAME_GAP_REASON: &str = "FRAME_GAP_NO_INTERPOLATION";
const OPERATOR_VERSION: &str =
    "amb2025-07-pixel-operator-v0.1|tam=max-contiguous-T>1298|scr=same-peak-1260-to-1150|linear-crossings";

#[derive(Clone, Debug)]
struct TraceMeta {
    dataset_id: String,
    run_id: String,
    sample_id: String,
    pixel_id: String,
    camera_clock_id: String,
    temperature_transform_id: String,
    temperature_transform_sha256: String,
    source_sha256: String,
}

#[derive(Clone, Copy, Debug)]
struct TraceSample {
    frame_index: u64,
    time_s: f64,
    temperature_c: f64,
}

#[derive(Clone, Debug)]
struct PixelTrace {
    meta: TraceMeta,
    samples: Vec<TraceSample>,
}

#[derive(Clone, Copy, Debug)]
struct HotInterval {
    first_hot_index: usize,
    last_hot_index: usize,
    start_time_s: f64,
    end_time_s: f64,
    duration_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetricStatus {
    Derived,
    Missing,
    Inconclusive,
}

impl MetricStatus {
    fn code(self) -> &'static str {
        match self {
            Self::Derived => "DERIVED",
            Self::Missing => "MISSING",
            Self::Inconclusive => "INCONCLUSIVE",
        }
    }
}

#[derive(Clone, Debug)]
struct PixelReport {
    dataset_id: String,
    run_id: String,
    sample_id: String,
    pixel_id: String,
    camera_clock_id: String,
    temperature_transform_id: String,
    temperature_transform_sha256: String,
    source_sha256: String,
    trace_sha256: String,
    operator_sha256: String,
    frame_count: usize,
    candidate_hot_intervals: usize,
    tam_status: MetricStatus,
    tam_reason: &'static str,
    tam_s: Option<f64>,
    selected_interval_start_s: Option<f64>,
    selected_interval_end_s: Option<f64>,
    selected_peak_temperature_c: Option<f64>,
    scr_status: MetricStatus,
    scr_reason: &'static str,
    t1260_s: Option<f64>,
    t1150_s: Option<f64>,
    scr_delta_t_s: Option<f64>,
    scr_c_per_s: Option<f64>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_trace(trace: &PixelTrace) -> Result<(), String> {
    let m = &trace.meta;
    if !nonempty(&m.dataset_id) || !nonempty(&m.run_id) || !nonempty(&m.sample_id)
        || !nonempty(&m.pixel_id) || !nonempty(&m.camera_clock_id)
        || !nonempty(&m.temperature_transform_id) {
        return Err("TRACE_IDENTITY_MISSING".into());
    }
    if !valid_sha256(&m.temperature_transform_sha256) || !valid_sha256(&m.source_sha256) {
        return Err("TRACE_SOURCE_DIGEST_INVALID".into());
    }
    if trace.samples.len() < 3 {
        return Err("TRACE_TOO_SHORT: require at least three frames to bracket a thermal event".into());
    }
    for (index, sample) in trace.samples.iter().enumerate() {
        if !sample.time_s.is_finite() || !sample.temperature_c.is_finite() {
            return Err("TRACE_NONFINITE_SAMPLE".into());
        }
        if index > 0 {
            let previous = trace.samples[index - 1];
            let expected_frame = previous.frame_index.checked_add(1)
                .ok_or("FRAME_INDEX_OVERFLOW")?;
            if sample.frame_index != expected_frame {
                return Err(FRAME_GAP_REASON.into());
            }
            if sample.time_s <= previous.time_s {
                return Err("TRACE_TIME_NOT_STRICTLY_INCREASING".into());
            }
        }
    }
    Ok(())
}

fn interpolate_crossing(
    left: TraceSample,
    right: TraceSample,
    threshold_c: f64,
) -> Result<f64, String> {
    let delta_temperature = right.temperature_c - left.temperature_c;
    if !delta_temperature.is_finite() || delta_temperature == 0.0 {
        return Err("CROSSING_FLAT_OR_INVALID".into());
    }
    let fraction = (threshold_c - left.temperature_c) / delta_temperature;
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err("CROSSING_NOT_BRACKETED".into());
    }
    let time = left.time_s + fraction * (right.time_s - left.time_s);
    if !time.is_finite() || time < left.time_s || time > right.time_s {
        return Err("CROSSING_TIME_INVALID".into());
    }
    Ok(time)
}

fn find_hot_intervals(samples: &[TraceSample]) -> Result<(Vec<HotInterval>, bool), String> {
    let mut intervals = Vec::new();
    let mut edge_censored = false;
    let mut index = 0usize;

    while index < samples.len() {
        if samples[index].temperature_c <= TAM_THRESHOLD_C {
            index += 1;
            continue;
        }
        let first_hot = index;
        while index + 1 < samples.len()
            && samples[index + 1].temperature_c > TAM_THRESHOLD_C {
            index += 1;
        }
        let last_hot = index;

        // The event must be bracketed by observed temperatures at/below
        // threshold on both sides. Never clip an edge-censored event to file bounds.
        if first_hot == 0 || last_hot + 1 >= samples.len() {
            edge_censored = true;
            index += 1;
            continue;
        }

        let start = interpolate_crossing(
            samples[first_hot - 1], samples[first_hot], TAM_THRESHOLD_C
        )?;
        let end = interpolate_crossing(
            samples[last_hot], samples[last_hot + 1], TAM_THRESHOLD_C
        )?;
        let duration = end - start;
        if !duration.is_finite() || duration <= 0.0 {
            return Err("TAM_INTERVAL_DURATION_INVALID".into());
        }
        intervals.push(HotInterval {
            first_hot_index: first_hot,
            last_hot_index: last_hot,
            start_time_s: start,
            end_time_s: end,
            duration_s: duration,
        });
        index += 1;
    }
    Ok((intervals, edge_censored))
}

fn choose_longest_interval(intervals: &[HotInterval]) -> Option<(usize, HotInterval)> {
    let mut best: Option<(usize, HotInterval)> = None;
    for (index, interval) in intervals.iter().copied().enumerate() {
        match best {
            None => best = Some((index, interval)),
            Some((_, current)) if interval.duration_s > current.duration_s => {
                best = Some((index, interval));
            }
            _ => {} // exact ties keep the earliest candidate.
        }
    }
    best
}

fn find_downward_crossing(
    samples: &[TraceSample],
    start_index: usize,
    threshold_c: f64,
) -> Option<(usize, f64)> {
    if samples.len() < 2 { return None; }
    for index in start_index..(samples.len() - 1) {
        let left = samples[index];
        let right = samples[index + 1];
        if left.temperature_c > threshold_c && right.temperature_c <= threshold_c {
            if let Ok(time) = interpolate_crossing(left, right, threshold_c) {
                return Some((index, time));
            }
        } else if left.temperature_c == threshold_c && right.temperature_c < threshold_c {
            return Some((index, left.time_s));
        }
    }
    None
}

fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 { data.push(0); }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x6a09e667_u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes(chunk[i*4..i*4+4].try_into().unwrap()); }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh] = h;
        for i in 0..64 {
            let big1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(big1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let big0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = big0.wrapping_add(maj);
            hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
        }
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b); h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f); h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }
    let mut output = String::with_capacity(64);
    for value in h { output.push_str(&format!("{value:08x}")); }
    output
}

fn trace_digest(trace: &PixelTrace) -> String {
    let mut bytes = Vec::with_capacity(trace.samples.len() * 40);
    for sample in &trace.samples {
        bytes.extend_from_slice(&sample.frame_index.to_le_bytes());
        bytes.extend_from_slice(&sample.time_s.to_bits().to_le_bytes());
        bytes.extend_from_slice(&sample.temperature_c.to_bits().to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn empty_report(trace: &PixelTrace, trace_sha: String, tam_status: MetricStatus,
    tam_reason: &'static str, scr_status: MetricStatus, scr_reason: &'static str,
    candidates: usize) -> PixelReport {
    PixelReport {
        dataset_id: trace.meta.dataset_id.clone(),
        run_id: trace.meta.run_id.clone(),
        sample_id: trace.meta.sample_id.clone(),
        pixel_id: trace.meta.pixel_id.clone(),
        camera_clock_id: trace.meta.camera_clock_id.clone(),
        temperature_transform_id: trace.meta.temperature_transform_id.clone(),
        temperature_transform_sha256: trace.meta.temperature_transform_sha256.clone(),
        source_sha256: trace.meta.source_sha256.clone(),
        trace_sha256: trace_sha,
        operator_sha256: sha256_hex(OPERATOR_VERSION.as_bytes()),
        frame_count: trace.samples.len(),
        candidate_hot_intervals: candidates,
        tam_status,
        tam_reason,
        tam_s: None,
        selected_interval_start_s: None,
        selected_interval_end_s: None,
        selected_peak_temperature_c: None,
        scr_status,
        scr_reason,
        t1260_s: None,
        t1150_s: None,
        scr_delta_t_s: None,
        scr_c_per_s: None,
    }
}

fn derive_pixel(trace: &PixelTrace) -> Result<PixelReport, String> {
    validate_trace(trace)?;
    let trace_sha = trace_digest(trace);
    let (intervals, edge_censored) = find_hot_intervals(&trace.samples)?;

    if edge_censored {
        return Ok(empty_report(
            trace, trace_sha, MetricStatus::Inconclusive,
            "TAM_EDGE_CENSORED_HOT_INTERVAL", MetricStatus::Inconclusive,
            "SCR_REQUIRES_COMPLETE_TAM_INTERVAL", intervals.len(),
        ));
    }
    let Some((_, interval)) = choose_longest_interval(&intervals) else {
        return Ok(empty_report(
            trace, trace_sha, MetricStatus::Missing, "NO_INTERVAL_ABOVE_1298C",
            MetricStatus::Missing, "NO_TAM_PEAK_FOR_SCR", 0,
        ));
    };

    let mut peak_index = interval.first_hot_index;
    for index in interval.first_hot_index..=interval.last_hot_index {
        if trace.samples[index].temperature_c > trace.samples[peak_index].temperature_c {
            peak_index = index;
        }
    }
    let peak_temperature = trace.samples[peak_index].temperature_c;

    let mut report = PixelReport {
        dataset_id: trace.meta.dataset_id.clone(),
        run_id: trace.meta.run_id.clone(),
        sample_id: trace.meta.sample_id.clone(),
        pixel_id: trace.meta.pixel_id.clone(),
        camera_clock_id: trace.meta.camera_clock_id.clone(),
        temperature_transform_id: trace.meta.temperature_transform_id.clone(),
        temperature_transform_sha256: trace.meta.temperature_transform_sha256.clone(),
        source_sha256: trace.meta.source_sha256.clone(),
        trace_sha256: trace_sha,
        operator_sha256: sha256_hex(OPERATOR_VERSION.as_bytes()),
        frame_count: trace.samples.len(),
        candidate_hot_intervals: intervals.len(),
        tam_status: MetricStatus::Derived,
        tam_reason: "LONGEST_COMPLETE_INTERVAL_ABOVE_1298C",
        tam_s: Some(interval.duration_s),
        selected_interval_start_s: Some(interval.start_time_s),
        selected_interval_end_s: Some(interval.end_time_s),
        selected_peak_temperature_c: Some(peak_temperature),
        scr_status: MetricStatus::Missing,
        scr_reason: "SCR_CROSSING_NOT_FOUND",
        t1260_s: None,
        t1150_s: None,
        scr_delta_t_s: None,
        scr_c_per_s: None,
    };

    let Some((index_1260, time_1260)) =
        find_downward_crossing(&trace.samples, peak_index, SOLIDUS_C) else {
        report.scr_status = MetricStatus::Missing;
        report.scr_reason = "SOLIDUS_DOWNWARD_CROSSING_NOT_FOUND";
        return Ok(report);
    };

    // Start at the same segment that brackets 1260 C. A single pair of
    // sparse frames may bracket both 1260 C and 1150 C; skipping that pair
    // would incorrectly mark a valid fast-cooling event as missing.
    let Some((index_1150, time_1150)) =
        find_downward_crossing(&trace.samples, index_1260, SCR_LOWER_C) else {
        report.scr_status = MetricStatus::Missing;
        report.scr_reason = "1150C_DOWNWARD_CROSSING_NOT_FOUND";
        report.t1260_s = Some(time_1260);
        return Ok(report);
    };

    // Reject a reheat above solidus between the two threshold crossings. If
    // both crossings lie in the same linear segment, there is no intermediate
    // observed sample at which a reheat can occur.
    if index_1150 > index_1260
        && trace.samples[index_1260 + 1..=index_1150]
            .iter().any(|sample| sample.temperature_c > SOLIDUS_C)
    {
        report.scr_status = MetricStatus::Inconclusive;
        report.scr_reason = "REHEATED_ABOVE_SOLIDUS_BEFORE_1150C";
        report.t1260_s = Some(time_1260);
        report.t1150_s = Some(time_1150);
        return Ok(report);
    }

    let delta_t = time_1150 - time_1260;
    if !delta_t.is_finite() || delta_t <= 0.0 {
        report.scr_status = MetricStatus::Inconclusive;
        report.scr_reason = "NONPOSITIVE_PHYSICAL_COOLING_INTERVAL";
        return Ok(report);
    }
    let rate = SCR_DELTA_C / delta_t;
    if !rate.is_finite() || rate <= 0.0 {
        report.scr_status = MetricStatus::Inconclusive;
        report.scr_reason = "SCR_RATE_INVALID";
        return Ok(report);
    }

    report.scr_status = MetricStatus::Derived;
    report.scr_reason = "SAME_TAM_PEAK_1260_TO_1150_LINEAR_COOLING";
    report.t1260_s = Some(time_1260);
    report.t1150_s = Some(time_1150);
    report.scr_delta_t_s = Some(delta_t);
    report.scr_c_per_s = Some(rate);
    Ok(report)
}

fn csv_row(report: &PixelReport) -> String {
    let f = |v: Option<f64>| v.map(|x| format!("{x:.12e}")).unwrap_or_default();
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        report.dataset_id, report.run_id, report.sample_id, report.pixel_id,
        report.camera_clock_id, report.temperature_transform_id,
        report.temperature_transform_sha256, report.source_sha256,
        report.trace_sha256, report.operator_sha256, report.frame_count,
        report.candidate_hot_intervals, report.tam_status.code(), report.tam_reason,
        f(report.tam_s), f(report.selected_interval_start_s), f(report.selected_interval_end_s),
        f(report.selected_peak_temperature_c), report.scr_status.code(), report.scr_reason,
        f(report.t1260_s), f(report.t1150_s), f(report.scr_delta_t_s), f(report.scr_c_per_s)
    )
}

fn fixture_meta(pixel_id: &str) -> TraceMeta {
    TraceMeta {
        dataset_id: "SYNTHETIC-OPERATOR-CHECK".into(),
        run_id: "RUN-001".into(),
        sample_id: "PAD-001".into(),
        pixel_id: pixel_id.into(),
        camera_clock_id: "CLOCK-CALIBRATED-TEST".into(),
        temperature_transform_id: "SYNTHETIC-TEMP-TRANSFORM".into(),
        temperature_transform_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
    }
}

fn sample(frame_index: u64, time_s: f64, temperature_c: f64) -> TraceSample {
    TraceSample { frame_index, time_s, temperature_c }
}

fn primary_fixture() -> PixelTrace {
    PixelTrace {
        meta: fixture_meta("PIXEL-1"),
        samples: vec![
            sample(0, 0.0, 1200.0),
            sample(1, 1.0, 1300.0),
            sample(2, 2.0, 1400.0),
            sample(3, 3.0, 1350.0),
            sample(4, 4.0, 1250.0),
            sample(5, 5.0, 1100.0),
            sample(6, 6.0, 900.0),
        ],
    }
}

fn verify() -> Result<(), String> {
    if sha256_hex(b"abc") != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err("SHA256_KNOWN_ANSWER_FAILED".into());
    }
    let primary = derive_pixel(&primary_fixture())?;
    let expected_tam = 3.0 + (1350.0 - 1298.0) / 100.0 - (1.0 + (1298.0 - 1300.0) / 100.0);
    let expected_t1260 = 3.0 + (1350.0 - 1260.0) / 100.0;
    let expected_t1150 = 4.0 + (1250.0 - 1150.0) / 150.0;
    let expected_scr = 110.0 / (expected_t1150 - expected_t1260);
    if primary.tam_status != MetricStatus::Derived
        || primary.scr_status != MetricStatus::Derived
        || (primary.tam_s.unwrap() - expected_tam).abs() > 1.0e-12
        || (primary.t1260_s.unwrap() - expected_t1260).abs() > 1.0e-12
        || (primary.t1150_s.unwrap() - expected_t1150).abs() > 1.0e-12
        || (primary.scr_c_per_s.unwrap() - expected_scr).abs() > 1.0e-10 {
        return Err("PRIMARY_PIXEL_KNOWN_ANSWER_FAILED".into());
    }

    // Longest of two hot intervals wins; exact ties keep the earliest.
    let two_peaks = PixelTrace {
        meta: fixture_meta("PIXEL-2"),
        samples: vec![
            sample(0,0.0,1200.0), sample(1,1.0,1320.0), sample(2,2.0,1340.0),
            sample(3,3.0,1200.0), sample(4,4.0,1330.0), sample(5,5.0,1400.0),
            sample(6,6.0,1300.0), sample(7,7.0,1200.0), sample(8,8.0,1100.0),
        ],
    };
    let multiple = derive_pixel(&two_peaks)?;
    if multiple.candidate_hot_intervals != 2
        || multiple.selected_interval_start_s.unwrap() <= 3.0 {
        return Err("LONGEST_TAM_INTERVAL_SELECTION_FAILED".into());
    }

    let incomplete_edge = PixelTrace {
        meta: fixture_meta("PIXEL-3"),
        samples: vec![
            sample(0,0.0,1300.0), sample(1,1.0,1400.0),
            sample(2,2.0,1200.0), sample(3,3.0,1000.0),
        ],
    };
    if derive_pixel(&incomplete_edge)?.tam_status != MetricStatus::Inconclusive {
        return Err("EDGE_CENSORING_CONTROL_FAILED".into());
    }

    Ok(())
}

fn emit_reference_csv() -> Result<(), String> {
    println!("dataset_id,run_id,sample_id,pixel_id,camera_clock_id,temperature_transform_id,temperature_transform_sha256,source_sha256,trace_sha256,operator_sha256,frame_count,candidate_hot_intervals,tam_status,tam_reason,tam_s,selected_interval_start_s,selected_interval_end_s,selected_peak_temperature_c,scr_status,scr_reason,t1260_s,t1150_s,scr_delta_t_s,scr_c_per_s");
    let report = derive_pixel(&primary_fixture())?;
    println!("{}", csv_row(&report));
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("AMB2025_07_PIXEL_OPERATOR_OK: TAM/SCR same-peak crossings and provenance")),
        [arg] if arg == "--verify" => verify().map(|()| println!("AMB2025_07_PIXEL_OPERATOR_OK: TAM/SCR same-peak crossings and provenance")),
        [arg] if arg == "--reference" => emit_reference_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: amb07-pixel [--verify|--reference|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --reference or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("AMB2025_07_PIXEL_OPERATOR_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_answer_uses_linear_crossings_and_same_peak() {
        verify().unwrap();
    }

    #[test]
    fn nonuniform_timestamps_are_used_not_nominal_frame_rate() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-NONUNIFORM"),
            samples: vec![
                sample(0,0.00,1200.0), sample(1,0.80,1300.0),
                sample(2,1.70,1400.0), sample(3,2.90,1350.0),
                sample(4,4.20,1250.0), sample(5,5.90,1100.0),
            ],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.tam_status, MetricStatus::Derived);
        let t1260 = 2.90 + (1350.0 - 1260.0) / 100.0 * (4.20 - 2.90);
        let t1150 = 4.20 + (1250.0 - 1150.0) / 150.0 * (5.90 - 4.20);
        assert!((report.t1260_s.unwrap() - t1260).abs() < 1.0e-12);
        assert!((report.t1150_s.unwrap() - t1150).abs() < 1.0e-12);
    }

    #[test]
    fn one_sparse_frame_pair_can_bracket_both_scr_temperatures() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-FAST-COOL"),
            samples: vec![
                sample(0,0.0,1200.0),
                sample(1,1.0,1320.0),
                sample(2,2.0,1400.0),
                sample(3,3.0,1100.0),
                sample(4,4.0,1000.0),
            ],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.tam_status, MetricStatus::Derived);
        assert_eq!(report.scr_status, MetricStatus::Derived);
        let expected_t1260 = 2.0 + (1400.0 - 1260.0) / 300.0;
        let expected_t1150 = 2.0 + (1400.0 - 1150.0) / 300.0;
        assert!((report.t1260_s.unwrap() - expected_t1260).abs() < 1.0e-12);
        assert!((report.t1150_s.unwrap() - expected_t1150).abs() < 1.0e-12);
        assert!(report.scr_delta_t_s.unwrap() > 0.0);
    }

    #[test]
    fn exact_threshold_is_not_above_melt() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-EQUAL-THRESHOLD"),
            samples: vec![
                sample(0,0.0,1200.0), sample(1,1.0,1298.0),
                sample(2,2.0,1290.0), sample(3,3.0,1200.0),
            ],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.tam_status, MetricStatus::Missing);
    }

    #[test]
    fn frame_gap_rejects_instead_of_interpolating_across_dropout() {
        let mut trace = primary_fixture();
        trace.samples[4].frame_index = 8;
        assert!(derive_pixel(&trace).unwrap_err().contains(FRAME_GAP_REASON));
    }

    #[test]
    fn nonmonotone_timestamps_and_nonfinite_values_reject() {
        let mut trace = primary_fixture();
        trace.samples[3].time_s = trace.samples[2].time_s;
        assert!(derive_pixel(&trace).unwrap_err().contains("TRACE_TIME_NOT_STRICTLY_INCREASING"));
        let mut trace = primary_fixture();
        trace.samples[2].temperature_c = f64::NAN;
        assert!(derive_pixel(&trace).unwrap_err().contains("TRACE_NONFINITE_SAMPLE"));
    }

    #[test]
    fn no_above_threshold_region_produces_missing_not_zero() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-NO-TAM"),
            samples: vec![sample(0,0.0,900.0), sample(1,1.0,1200.0), sample(2,2.0,1000.0)],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.tam_status, MetricStatus::Missing);
        assert_eq!(report.scr_status, MetricStatus::Missing);
        assert!(report.tam_s.is_none());
        assert!(report.scr_c_per_s.is_none());
    }

    #[test]
    fn incomplete_unbounded_hot_event_is_not_clipped_to_trace_boundary() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-EDGE"),
            samples: vec![sample(0,0.0,1300.0), sample(1,1.0,1400.0),
                sample(2,2.0,1350.0), sample(3,3.0,1200.0)],
        };
        assert_eq!(derive_pixel(&trace).unwrap().tam_status, MetricStatus::Inconclusive);
    }

    #[test]
    fn report_replays_with_identical_trace_and_transform_digests() {
        let trace = primary_fixture();
        let a = derive_pixel(&trace).unwrap();
        let b = derive_pixel(&trace).unwrap();
        assert_eq!(a.trace_sha256, b.trace_sha256);
        assert_eq!(a.operator_sha256, b.operator_sha256);
        assert_eq!(csv_row(&a), csv_row(&b));
    }

    #[test]
    fn invalid_provenance_fails_closed() {
        let mut trace = primary_fixture();
        trace.meta.temperature_transform_sha256 = "bad".into();
        assert!(derive_pixel(&trace).unwrap_err().contains("TRACE_SOURCE_DIGEST_INVALID"));
    }

    #[test]
    fn reheat_above_solidus_before_1150_makes_scr_inconclusive() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-REHEAT"),
            samples: vec![
                sample(0,0.0,1200.0), sample(1,1.0,1300.0),
                sample(2,2.0,1400.0), sample(3,3.0,1250.0),
                sample(4,4.0,1300.0), sample(5,5.0,1200.0),
                sample(6,6.0,1100.0),
            ],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.tam_status, MetricStatus::Derived);
        assert_eq!(report.scr_status, MetricStatus::Inconclusive);
        assert_eq!(report.scr_reason, "REHEATED_ABOVE_SOLIDUS_BEFORE_1150C");
    }

    #[test]
    fn selected_interval_ties_keep_earliest() {
        let trace = PixelTrace {
            meta: fixture_meta("PIXEL-TIE"),
            samples: vec![
                sample(0,0.0,1200.0), sample(1,1.0,1300.0), sample(2,2.0,1400.0),
                sample(3,3.0,1290.0), sample(4,4.0,1200.0),
                sample(5,5.0,1300.0), sample(6,6.0,1400.0),
                sample(7,7.0,1290.0), sample(8,8.0,1200.0),
                sample(9,9.0,1100.0),
            ],
        };
        let report = derive_pixel(&trace).unwrap();
        assert_eq!(report.candidate_hot_intervals, 2);
        assert!(report.selected_interval_end_s.unwrap() < 4.0);
    }
}
