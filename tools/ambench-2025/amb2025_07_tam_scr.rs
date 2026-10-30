//! AMB2025-07 TAM/SCR reference scorer.
//!
//! This program scores *summary observables* for NIST CHAL-AMB2025-07-PCRTAM.
//! It does not reconstruct temperature fields or validate the thermal solver.
//! Raw rows are admitted from a strict CSV schema, filtered per the NIST report,
//! pooled across three repeats, then compared with published aggregate summaries.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/ambench-2025/amb2025_07_tam_scr.rs -o /tmp/amb07-tests
//!   /tmp/amb07-tests
//!   rustc --edition=2024 -D warnings tools/ambench-2025/amb2025_07_tam_scr.rs -o /tmp/amb07
//!   /tmp/amb07 --verify
//!   /tmp/amb07 --reference
//!   /tmp/amb07 --score data.csv
//!
//! CSV columns:
//! sample_id,repeat_id,pad_width_mm,turnaround_ms,tam_s,scr_c_per_s
//! Empty TAM/SCR cells are missing observations; they are never converted to zero.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const TAM_MIN_SECONDS: f64 = 5.0e-4;
const SCR_MAX_C_PER_S: f64 = 1.0e6;
const NIST_TAM_COUNT_SCALE: f64 = 1.0e5;
const NIST_SCR_COUNT_SCALE: f64 = 1.0e4;
const NIST_SCR_VALUE_SCALE: f64 = 1.0e5;
const ROUNDING_TOLERANCE: f64 = 0.005 + 1.0e-10;
const FILTER_VERSION: &str =
    "amb2025-07-tam-scr-filter-v0.1|tam_s>=0.0005|scr_c_per_s<=1e6|pool-three-repeats|population-std|linear-quantile";

const HEADER: &str = "sample_id,repeat_id,pad_width_mm,turnaround_ms,tam_s,scr_c_per_s";
const REPEAT_IDS_075: [&str; 3] = [
    "20241010_AMB_T72",
    "20241015_AMB_T82",
    "20241015_AMB_T92",
];
const REPEAT_IDS_500: [&str; 3] = [
    "20241010_AMB_T102",
    "20241015_AMB_T112",
    "20241015_AMB_T122",
];

#[derive(Clone, Copy, Debug)]
struct Reference {
    pad_width_mm: u8,
    turnaround_ms_hundredths: u16,
    tam_n_x1e5: f64,
    tam_mean_ms: f64,
    tam_median_ms: f64,
    tam_std_ms: f64,
    scr_n_x1e4: f64,
    scr_mean_x1e5: f64,
    scr_median_x1e5: f64,
    scr_std_x1e5: f64,
}

const REFERENCES: [Reference; 4] = [
    Reference {
        pad_width_mm: 1, turnaround_ms_hundredths: 75,
        tam_n_x1e5: 6.59, tam_mean_ms: 2.00, tam_median_ms: 1.80, tam_std_ms: 1.00,
        scr_n_x1e4: 10.11, scr_mean_x1e5: 3.86, scr_median_x1e5: 3.61, scr_std_x1e5: 2.79,
    },
    Reference {
        pad_width_mm: 1, turnaround_ms_hundredths: 500,
        tam_n_x1e5: 5.95, tam_mean_ms: 0.87, tam_median_ms: 0.87, tam_std_ms: 0.20,
        scr_n_x1e4: 10.46, scr_mean_x1e5: 4.75, scr_median_x1e5: 4.32, scr_std_x1e5: 2.39,
    },
    Reference {
        pad_width_mm: 5, turnaround_ms_hundredths: 7,
        tam_n_x1e5: 36.20, tam_mean_ms: 1.50, tam_median_ms: 1.50, tam_std_ms: 0.26,
        scr_n_x1e4: 39.77, scr_mean_x1e5: 1.95, scr_median_x1e5: 1.21, scr_std_x1e5: 1.84,
    },
    Reference {
        pad_width_mm: 5, turnaround_ms_hundredths: 50,
        tam_n_x1e5: 36.27, tam_mean_ms: 1.20, tam_median_ms: 1.30, tam_std_ms: 0.21,
        scr_n_x1e4: 40.38, scr_mean_x1e5: 2.67, scr_median_x1e5: 1.94, scr_std_x1e5: 1.80,
    },
];

#[derive(Clone, Copy, Debug, Default)]
struct Stats {
    n: usize,
    mean: f64,
    median: f64,
    population_std: f64,
    q05: f64,
    q25: f64,
    q75: f64,
    q95: f64,
}

impl Stats {
    fn from_values(values: &[f64]) -> Result<Self, String> {
        if values.is_empty() {
            return Err("NO_ADMITTED_OBSERVATIONS".into());
        }
        if values.iter().any(|x| !x.is_finite()) {
            return Err("NONFINITE_ADMITTED_VALUE".into());
        }
        let n = values.len();
        let mean = values.iter().sum::<f64>() / n as f64;
        let variance = values.iter().map(|x| {
            let d = x - mean;
            d * d
        }).sum::<f64>() / n as f64;
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let median = quantile_sorted(&sorted, 0.5);
        let population_std = variance.sqrt();
        let result = Self {
            n, mean, median, population_std,
            q05: quantile_sorted(&sorted, 0.05),
            q25: quantile_sorted(&sorted, 0.25),
            q75: quantile_sorted(&sorted, 0.75),
            q95: quantile_sorted(&sorted, 0.95),
        };
        if [
            result.mean, result.median, result.population_std,
            result.q05, result.q25, result.q75, result.q95,
        ].iter().any(|x| !x.is_finite()) {
            return Err("NONFINITE_STATISTIC".into());
        }
        Ok(result)
    }
}

// Linear interpolation on the (n-1)*p index; matches the declared Type-7
// quantile convention. For an even median this averages the two central values.
fn quantile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.len() == 1 { return sorted[0]; }
    let position = (sorted.len() - 1) as f64 * p;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper { sorted[lower] }
    else {
        let fraction = position - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

#[derive(Debug)]
struct GroupAccumulator {
    reference: Reference,
    seen_repeats: HashSet<String>,
    tam_values: Vec<f64>,
    scr_values: Vec<f64>,
    tam_missing: usize,
    tam_below_threshold: usize,
    scr_missing: usize,
    scr_above_threshold: usize,
    tam_pre_filter: usize,
    scr_pre_filter: usize,
}

impl GroupAccumulator {
    fn new(reference: Reference) -> Self {
        Self {
            reference,
            seen_repeats: HashSet::new(),
            tam_values: Vec::new(),
            scr_values: Vec::new(),
            tam_missing: 0,
            tam_below_threshold: 0,
            scr_missing: 0,
            scr_above_threshold: 0,
            tam_pre_filter: 0,
            scr_pre_filter: 0,
        }
    }

    fn ingest(&mut self, tam_s: Option<f64>, scr_c_per_s: Option<f64>) -> Result<(), String> {
        match tam_s {
            None => self.tam_missing += 1,
            Some(value) => {
                if !value.is_finite() || value < 0.0 {
                    return Err("TAM_VALUE_INVALID: expected finite seconds >= 0".into());
                }
                self.tam_pre_filter += 1;
                if value < TAM_MIN_SECONDS {
                    self.tam_below_threshold += 1;
                } else {
                    self.tam_values.push(value * 1.0e3); // report in ms
                }
            }
        }
        match scr_c_per_s {
            None => self.scr_missing += 1,
            Some(value) => {
                if !value.is_finite() || value < 0.0 {
                    return Err("SCR_VALUE_INVALID: expected finite degrees C per second >= 0".into());
                }
                self.scr_pre_filter += 1;
                if value > SCR_MAX_C_PER_S {
                    self.scr_above_threshold += 1;
                } else {
                    self.scr_values.push(value / NIST_SCR_VALUE_SCALE); // report in 1e5 C/s
                }
            }
        }
        if tam_s.is_none() && scr_c_per_s.is_none() {
            return Err("ROW_HAS_NO_METRIC_OBSERVATION".into());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ScoredGroup {
    reference: Reference,
    tam: Stats,
    scr: Stats,
    tam_missing: usize,
    tam_below_threshold: usize,
    tam_pre_filter: usize,
    scr_missing: usize,
    scr_above_threshold: usize,
    scr_pre_filter: usize,
    tam_summary_match: bool,
    scr_summary_match: bool,
}

#[derive(Debug)]
struct Campaign {
    groups: Vec<ScoredGroup>,
    source_sha256: String,
    filter_sha256: String,
    total_rows: usize,
    status: &'static str,
}

fn group_index(pad_width: f64, turnaround_ms: f64) -> Result<usize, String> {
    if !pad_width.is_finite() || !turnaround_ms.is_finite() {
        return Err("GROUP_VALUE_NONFINITE".into());
    }
    let pad = if (pad_width - 1.0).abs() < 1.0e-9 { 1_u8 }
        else if (pad_width - 5.0).abs() < 1.0e-9 { 5_u8 }
        else { return Err("PAD_WIDTH_UNSUPPORTED".into()); };
    let turnaround = if (turnaround_ms - 0.75).abs() < 1.0e-9 { 75_u16 }
        else if (turnaround_ms - 5.0).abs() < 1.0e-9 { 500_u16 }
        else { return Err("TURNAROUND_UNSUPPORTED".into()); };
    REFERENCES.iter().position(|r| {
        r.pad_width_mm == pad && r.turnaround_ms_hundredths == turnaround
    }).ok_or_else(|| "GROUP_UNSUPPORTED".into())
}

fn allowed_repeats(turnaround_ms_hundredths: u8) -> &'static [&'static str; 3] {
    if turnaround_ms_hundredths == 75 { &REPEAT_IDS_075 } else { &REPEAT_IDS_500 }
}

fn within_published_rounding(actual: f64, reference: f64) -> bool {
    (actual - reference).abs() <= ROUNDING_TOLERANCE
}

fn parse_optional_number(field: &str, name: &str) -> Result<Option<f64>, String> {
    if field.is_empty() { return Ok(None); }
    let value = field.parse::<f64>()
        .map_err(|_| format!("{name}_PARSE_ERROR"))?;
    if !value.is_finite() { return Err(format!("{name}_NONFINITE")); }
    Ok(Some(value))
}

fn parse_csv<R: BufRead>(reader: R, source_sha256: String) -> Result<Campaign, String> {
    if !is_sha256(&source_sha256) {
        return Err("SOURCE_SHA256_INVALID".into());
    }
    let filter_sha256 = sha256_hex(FILTER_VERSION.as_bytes());
    let mut lines = reader.lines();
    let first = lines.next().ok_or("CSV_EMPTY")?
        .map_err(|_| "CSV_HEADER_READ_ERROR")?;
    if first.trim_end_matches('\r') != HEADER {
        return Err("CSV_HEADER_MISMATCH".into());
    }
    let mut samples = HashSet::new();
    let mut groups: Vec<GroupAccumulator> = REFERENCES.iter().copied()
        .map(GroupAccumulator::new).collect();
    let mut total_rows = 0usize;

    for (line_index, line) in lines.enumerate() {
        let line = line.map_err(|_| format!("CSV_READ_ERROR_LINE_{}", line_index + 2))?;
        let line = line.trim_end_matches('\r');
        if line.is_empty() { continue; }
        if line.contains('"') {
            return Err(format!("CSV_QUOTES_UNSUPPORTED_LINE_{}", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            return Err(format!("CSV_FIELD_COUNT_LINE_{}", line_index + 2));
        }
        let sample_id = fields[0].trim();
        let repeat_id = fields[1].trim();
        if sample_id.is_empty() || repeat_id.is_empty() {
            return Err(format!("SAMPLE_OR_REPEAT_ID_MISSING_LINE_{}", line_index + 2));
        }
        if !samples.insert(sample_id.to_string()) {
            return Err(format!("DUPLICATE_SAMPLE_ID_LINE_{}", line_index + 2));
        }
        let pad_width = fields[2].parse::<f64>()
            .map_err(|_| format!("PAD_WIDTH_PARSE_LINE_{}", line_index + 2))?;
        let turnaround = fields[3].parse::<f64>()
            .map_err(|_| format!("TURNAROUND_PARSE_LINE_{}", line_index + 2))?;
        let index = group_index(pad_width, turnaround)?;
        let expected_repeats = allowed_repeats(REFERENCES[index].turnaround_ms_hundredths);
        if !expected_repeats.contains(&repeat_id) {
            return Err(format!("REPEAT_ID_INVALID_FOR_GROUP_LINE_{}", line_index + 2));
        }
        groups[index].seen_repeats.insert(repeat_id.to_string());
        let tam = parse_optional_number(fields[4], "TAM")?;
        let scr = parse_optional_number(fields[5], "SCR")?;
        groups[index].ingest(tam, scr)
            .map_err(|e| format!("{e}_LINE_{}", line_index + 2))?;
        total_rows += 1;
    }

    if total_rows == 0 { return Err("CSV_HAS_NO_SAMPLE_ROWS".into()); }

    let mut scored = Vec::with_capacity(4);
    
    let mut all_summaries_match = true;

    for group in groups {
        if group.seen_repeats.len() != 3 {
            return Err(format!(
                "GROUP_MISSING_REPEAT: pad={} turnaround_hundredths={}",
                group.reference.pad_width_mm, group.reference.turnaround_ms_hundredths
            ));
        }
        let tam = Stats::from_values(&group.tam_values)?;
        let scr = Stats::from_values(&group.scr_values)?;
        let ref_row = group.reference;
        let tam_n_ok = within_published_rounding(
            tam.n as f64 / NIST_TAM_COUNT_SCALE, ref_row.tam_n_x1e5
        );
        let scr_n_ok = within_published_rounding(
            scr.n as f64 / NIST_SCR_COUNT_SCALE, ref_row.scr_n_x1e4
        );
        let tam_ok = tam_n_ok
            && within_published_rounding(tam.mean, ref_row.tam_mean_ms)
            && within_published_rounding(tam.median, ref_row.tam_median_ms)
            && within_published_rounding(tam.population_std, ref_row.tam_std_ms);
        let scr_ok = scr_n_ok
            && within_published_rounding(scr.mean, ref_row.scr_mean_x1e5)
            && within_published_rounding(scr.median, ref_row.scr_median_x1e5)
            && within_published_rounding(scr.population_std, ref_row.scr_std_x1e5);
        all_summaries_match &= tam_ok && scr_ok;

        scored.push(ScoredGroup {
            reference: ref_row,
            tam, scr,
            tam_missing: group.tam_missing,
            tam_below_threshold: group.tam_below_threshold,
            tam_pre_filter: group.tam_pre_filter,
            scr_missing: group.scr_missing,
            scr_above_threshold: group.scr_above_threshold,
            scr_pre_filter: group.scr_pre_filter,
            tam_summary_match: tam_ok,
            scr_summary_match: scr_ok,
        });
    }

    let status = if all_summaries_match { "MATCHES_PUBLISHED_SUMMARY" }
        else { "DOES_NOT_MATCH_PUBLISHED_SUMMARY" };
    Ok(Campaign {
        groups: scored,
        source_sha256,
        filter_sha256,
        total_rows,
        status,
    })
}

fn emit_reference_csv() {
    println!("pad_width_mm,turnaround_ms,tam_n_x1e5,tam_mean_ms,tam_median_ms,tam_std_ms,scr_n_x1e4,scr_mean_x1e5,scr_median_x1e5,scr_std_x1e5");
    for r in REFERENCES {
        println!("{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
            r.pad_width_mm, r.turnaround_ms_hundredths as f64 / 100.0,
            r.tam_n_x1e5, r.tam_mean_ms, r.tam_median_ms, r.tam_std_ms,
            r.scr_n_x1e4, r.scr_mean_x1e5, r.scr_median_x1e5, r.scr_std_x1e5);
    }
}

fn report_csv(campaign: &Campaign) {
    println!("pad_width_mm,turnaround_ms,total_rows,tam_pre_filter,tam_missing,tam_below_0_5ms,tam_n,tam_mean_ms,tam_median_ms,tam_population_std_ms,tam_q05_ms,tam_q25_ms,tam_q75_ms,tam_q95_ms,tam_ref_mean_ms,tam_abs_mean_error_ms,tam_ref_median_ms,tam_abs_median_error_ms,tam_ref_std_ms,tam_abs_std_error_ms,tam_summary_match,scr_pre_filter,scr_missing,scr_above_1e6,scr_n,scr_mean_x1e5_c_per_s,scr_median_x1e5_c_per_s,scr_population_std_x1e5_c_per_s,scr_q05_x1e5,scr_q25_x1e5,scr_q75_x1e5,scr_q95_x1e5,scr_ref_mean_x1e5,scr_abs_mean_error_x1e5,scr_ref_median_x1e5,scr_abs_median_error_x1e5,scr_ref_std_x1e5,scr_abs_std_error_x1e5,scr_summary_match,source_sha256,filter_sha256,campaign_status");
    for g in &campaign.groups {
        let r = g.reference;
        let turnaround = r.turnaround_ms_hundredths as f64 / 100.0;
        println!(
            "{},{:.2},{},{},{},{},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{},{},{},{},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{},{},{},{}",
            r.pad_width_mm, turnaround, campaign.total_rows,
            g.tam_pre_filter, g.tam_missing, g.tam_below_threshold, g.tam.n,
            g.tam.mean, g.tam.median, g.tam.population_std,
            g.tam.q05, g.tam.q25, g.tam.q75, g.tam.q95,
            r.tam_mean_ms, (g.tam.mean-r.tam_mean_ms).abs(),
            r.tam_median_ms, (g.tam.median-r.tam_median_ms).abs(),
            r.tam_std_ms, (g.tam.population_std-r.tam_std_ms).abs(),
            g.tam_summary_match, g.scr_pre_filter, g.scr_missing,
            g.scr_above_threshold, g.scr.n,
            g.scr.mean, g.scr.median, g.scr.population_std,
            g.scr.q05, g.scr.q25, g.scr.q75, g.scr.q95,
            r.scr_mean_x1e5, (g.scr.mean-r.scr_mean_x1e5).abs(),
            r.scr_median_x1e5, (g.scr.median-r.scr_median_x1e5).abs(),
            r.scr_std_x1e5, (g.scr.population_std-r.scr_std_x1e5).abs(),
            g.scr_summary_match, campaign.source_sha256,
            campaign.filter_sha256, campaign.status
        );
    }
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
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
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i*4..i*4+4].try_into().unwrap());
        }
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
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b);
        h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f);
        h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }
    let mut output = String::with_capacity(64);
    for value in h { output.push_str(&format!("{value:08x}")); }
    output
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [arg] if arg == "--verify" => verify().map(|()| {
            println!("AMB2025_07_METRICS_OK: frozen NIST summary, independent filters and hash known-answer");
        }),
        [arg] if arg == "--reference" => { emit_reference_csv(); Ok(()) },
        [arg, path] if arg == "--score" => {
            score_file(Path::new(path))
        },
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: amb07 [--verify|--reference|--score input.csv]");
            Ok(())
        }
        _ => Err("expected --verify, --reference, or --score input.csv".into()),
    };
    if let Err(e) = result {
        eprintln!("AMB2025_07_METRICS_ERROR: {e}");
        std::process::exit(1);
    }
}

fn score_file(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("INPUT_READ_ERROR: {e}"))?;
    let source_hash = sha256_hex(&bytes);
    let reader = BufReader::new(bytes.as_slice());
    let campaign = parse_csv(reader, source_hash)?;
    report_csv(&campaign);
    Ok(())
}

fn verify() -> Result<(), String> {
    if sha256_hex(b"abc") != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err("SHA256_KNOWN_ANSWER_FAILED".into());
    }
    if quantile_sorted(&[1.0, 3.0, 5.0, 7.0], 0.5) != 4.0
        || quantile_sorted(&[1.0, 3.0, 5.0, 7.0], 0.25) != 2.5 {
        return Err("LINEAR_QUANTILE_KNOWN_ANSWER_FAILED".into());
    }
    let stats = Stats::from_values(&[1.0, 2.0, 3.0, 4.0]).map_err(|e| e)?;
    if (stats.mean - 2.5).abs() > 1.0e-14
        || (stats.population_std - 1.118033988749895).abs() > 1.0e-14
        || stats.median != 2.5 {
        return Err("POPULATION_STATISTICS_KNOWN_ANSWER_FAILED".into());
    }

    let reference = REFERENCES;
    if reference.len() != 4 {
        return Err("REFERENCE_GROUP_COUNT_INVALID".into());
    }
    for r in reference {
        if !(r.pad_width_mm == 1 || r.pad_width_mm == 5)
            || !(r.turnaround_ms_hundredths == 7 || r.turnaround_ms_hundredths == 50)
            || r.tam_n_x1e5 <= 0.0 || r.scr_n_x1e4 <= 0.0
            || r.tam_mean_ms < TAM_MIN_SECONDS * 1.0e3
            || r.scr_mean_x1e5 <= 0.0 {
            return Err("REFERENCE_ROW_INVALID".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answer() {
        assert_eq!(sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn nist_filters_are_independent_and_have_explicit_boundary_semantics() {
        let mut g = GroupAccumulator::new(REFERENCES[0]);
        g.ingest(Some(0.000_499), Some(500_000.0)).unwrap();
        g.ingest(Some(0.000_500), Some(1_000_000.0)).unwrap();
        g.ingest(Some(0.001_000), Some(1_000_001.0)).unwrap();
        g.ingest(None, Some(200_000.0)).unwrap();
        g.ingest(Some(0.002_000), None).unwrap();

        assert_eq!(g.tam_pre_filter, 4);
        assert_eq!(g.tam_missing, 1);
        assert_eq!(g.tam_below_threshold, 1);
        assert_eq!(g.tam_values.len(), 3);
        assert_eq!(g.scr_pre_filter, 4);
        assert_eq!(g.scr_missing, 1);
        assert_eq!(g.scr_above_threshold, 1);
        assert_eq!(g.scr_values.len(), 3);
    }

    #[test]
    fn population_std_and_type7_quantile_are_deterministic() {
        let s = Stats::from_values(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!(s.mean, 2.5);
        assert_eq!(s.median, 2.5);
        assert!((s.population_std - 1.118033988749895).abs() < 1.0e-14);
        assert_eq!(s.q25, 1.75);
        assert_eq!(s.q75, 3.25);
    }

    #[test]
    fn non_normal_scr_distribution_retains_tail_quantiles() {
        let values = [1.0, 1.0, 1.0, 1.0, 2.0, 3.0, 8.0, 25.0, 80.0];
        let s = Stats::from_values(&values).unwrap();
        assert!(s.q95 > s.q75);
        assert!(s.population_std > 1.0);
        assert!(s.median < s.mean);
    }

    #[test]
    fn pooled_three_repeat_samples_are_not_mean_of_repeat_means() {
        let pooled = Stats::from_values(&[0.0, 0.0, 0.0, 10.0]).unwrap().mean;
        let repeat_mean_average = (0.0 + 10.0) / 2.0;
        assert_eq!(pooled, 2.5);
        assert_eq!(repeat_mean_average, 5.0);
        assert_ne!(pooled, repeat_mean_average);
    }

    #[test]
    fn reference_values_encode_four_official_groups() {
        assert_eq!(REFERENCES.len(), 4);
        assert_eq!(REFERENCES[0].tam_mean_ms, 2.00);
        assert_eq!(REFERENCES[1].tam_mean_ms, 0.87);
        assert_eq!(REFERENCES[2].scr_median_x1e5, 1.21);
        assert_eq!(REFERENCES[3].scr_std_x1e5, 1.80);
    }

    #[test]
    fn strict_csv_rejects_duplicate_ids_and_bad_headers() {
        let bad_header = "sample_id,repeat_id,pad_width_mm,turnaround_ms,tam_s,scr_c_per_s\n";
        let result = parse_csv(BufReader::new(bad_header.as_bytes()), "a".repeat(64));
        assert!(result.is_err());
        let duplicate = format!("{HEADER}\ns1,{},{},{},{},{}\ns1,{},{},{},{},{}\n",
            REPEAT_IDS_075[0],1,0.75,0.001,500000.0,
            REPEAT_IDS_075[0],1,0.75,0.001,500000.0);
        let result = parse_csv(BufReader::new(duplicate.as_bytes()), "a".repeat(64));
        assert!(result.unwrap_err().contains("DUPLICATE_SAMPLE_ID"));
    }

    #[test]
    fn missing_metric_is_not_imputed_or_shared_across_channels() {
        let mut g = GroupAccumulator::new(REFERENCES[0]);
        g.ingest(Some(0.001), None).unwrap();
        g.ingest(None, Some(500_000.0)).unwrap();
        assert_eq!(g.tam_values.len(), 1);
        assert_eq!(g.tam_missing, 1);
        assert_eq!(g.scr_values.len(), 1);
        assert_eq!(g.scr_missing, 1);
    }

    #[test]
    fn campaign_requires_all_three_repeat_ids_in_all_four_groups() {
        let mut source = format!("{HEADER}\n");
        for r in REFERENCES {
            let repeats = allowed_repeats(r.turnaround_ms_hundredths);
            for repeat in repeats.iter().take(2) {
                source.push_str(&format!("{}-{repeat},{repeat},{},{:.2},0.001,500000\n",
                    r.pad_width_mm, r.pad_width_mm, r.pad_width_mm,
                    r.turnaround_ms_hundredths as f64 / 10.0));
            }
        }
        let result = parse_csv(BufReader::new(source.as_bytes()), "a".repeat(64));
        assert!(result.unwrap_err().contains("GROUP_MISSING_REPEAT"));
    }

    #[test]
    fn malformed_numeric_and_empty_rows_reject() {
        let malformed = format!("{HEADER}\ns1,{},1,0.75,NaN,500000\n", REPEAT_IDS_075[0]);
        assert!(parse_csv(BufReader::new(malformed.as_bytes()), "a".repeat(64)).unwrap_err().contains("TAM_NONFINITE"));
        let empty_row = format!("{HEADER}\ns1,{},1,0.75,,\n", REPEAT_IDS_075[0]);
        assert!(parse_csv(BufReader::new(empty_row.as_bytes()), "a".repeat(64)).unwrap_err().contains("ROW_HAS_NO_METRIC_OBSERVATION"));
    }
}
