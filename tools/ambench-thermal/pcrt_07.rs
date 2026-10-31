//! NIST AM-Bench 2025-07 PCRTAM thermal-observable reference.
//!
//! Implements the official per-pixel TAM and SCR extraction definitions from
//! AMB2025-06/07 PDF §3.1. It accepts temperature-vs-time traces already
//! converted from thermography; it does not convert raw IR video.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/ambench-thermal/pcrt_07.rs -o /tmp/pcrt-tests
//!   /tmp/pcrt-tests
//!   rustc --edition=2024 -D warnings tools/ambench-thermal/pcrt_07.rs -o /tmp/pcrt
//!   /tmp/pcrt --verify
//!   /tmp/pcrt --csv

use std::collections::BTreeMap;

const TAM_THRESHOLD_C: f64 = 1298.0;
const SOLIDUS_C: f64 = 1260.0;
const SCR_LOWER_C: f64 = 1150.0;
const SCR_DELTA_C: f64 = SOLIDUS_C - SCR_LOWER_C;
const REQUIRED_EMISSIVITY: f64 = 0.5;
const MAX_OUTPUT_POINTS: usize = 1_000_000;
const GRID_REL_TOL: f64 = 1.0e-9;
const TIME_JOIN_TOL: f64 = 128.0 * f64::EPSILON;

#[derive(Clone, Debug)]
struct Grid {
    rows: usize,
    columns: usize,
    origin_x_um: f64,
    origin_y_um: f64,
    spacing_x_um: f64,
    spacing_y_um: f64,
}

#[derive(Clone, Debug)]
struct PixelProfile {
    pixel_id: String,
    row: usize,
    column: usize,
    x_um: f64,
    y_um: f64,
    temperatures_c: Vec<Option<f64>>,
    source_profile_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CollectionKind {
    ThermographyReference,
    ModelPrediction,
}

impl CollectionKind {
    fn code(&self) -> &'static str {
        match self {
            Self::ThermographyReference => "THERMOGRAPHY_REFERENCE",
            Self::ModelPrediction => "MODEL_PREDICTION",
        }
    }
}

#[derive(Clone, Debug)]
struct Collection {
    collection_id: String,
    kind: CollectionKind,
    dataset_doi: String,
    dataset_version: String,
    source_sha256: String,
    grid_sha256: String,
    time_axis_sha256: String,
    conversion_receipt_sha256: Option<String>,
    emissivity: Option<f64>,
    temperature_unit: String,
    time_unit: String,
    algorithm_version: String,
    grid: Grid,
    times_s: Vec<f64>,
    profiles: Vec<PixelProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Interval {
    start_s: f64,
    end_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct TamEpisode {
    start_s: f64,
    end_s: f64,
    peak_s: f64,
    peak_index: usize,
    duration_s: f64,
}

#[derive(Clone, Debug)]
struct PixelResult {
    pixel_id: String,
    row: usize,
    column: usize,
    x_um: f64,
    y_um: f64,
    tam_s: Option<f64>,
    scr_c_per_s: Option<f64>,
    selected_tam_start_s: Option<f64>,
    selected_tam_end_s: Option<f64>,
    selected_peak_s: Option<f64>,
    tam_reason: &'static str,
    scr_reason: &'static str,
    source_profile_sha256: String,
}

#[derive(Clone, Copy, Debug)]
struct MetricStats {
    total_count: usize,
    valid_count: usize,
    missing_count: usize,
    mean: Option<f64>,
    median: Option<f64>,
    sample_sd: Option<f64>,
    population_sd: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct Comparison {
    tam_ks_d: Option<f64>,
    scr_ks_d: Option<f64>,
    paired_tam_count: usize,
    paired_tam_mae_s: Option<f64>,
    paired_tam_rmse_s: Option<f64>,
    paired_scr_count: usize,
    paired_scr_mae_c_per_s: Option<f64>,
    paired_scr_rmse_c_per_s: Option<f64>,
}

#[derive(Clone, Debug)]
struct CollectionReport {
    collection_id: String,
    kind: CollectionKind,
    source_sha256: String,
    grid_sha256: String,
    time_axis_sha256: String,
    dataset_doi: String,
    dataset_version: String,
    conversion_receipt_sha256: Option<String>,
    emissivity: Option<f64>,
    temperature_unit: String,
    time_unit: String,
    algorithm_version: String,
    grid: Grid,
    frame_count: usize,
    start_time_s: f64,
    end_time_s: f64,
    tam: MetricStats,
    scr: MetricStats,
    pixels: Vec<PixelResult>,
}

#[derive(Clone, Debug)]
struct ComparisonReport {
    predicted: CollectionReport,
    measured: CollectionReport,
    comparison: Comparison,
    paired_grid: bool,
}

fn safe_token(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.contains(',')
        && !value.contains('\n')
        && !value.contains('\r')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn near(a: f64, b: f64, relative_tolerance: f64) -> bool {
    (a - b).abs() <= relative_tolerance * a.abs().max(b.abs()).max(1.0)
}

fn validate_collection(collection: &Collection) -> Result<(), &'static str> {
    let g = &collection.grid;
    for identity in [
        collection.collection_id.as_str(),
        collection.dataset_doi.as_str(),
        collection.dataset_version.as_str(),
        collection.temperature_unit.as_str(),
        collection.time_unit.as_str(),
        collection.algorithm_version.as_str(),
    ] {
        if !safe_token(identity) {
            return Err("COLLECTION_IDENTITY_MISSING_OR_UNSAFE");
        }
    }
    if !valid_sha256(&collection.source_sha256)
        || !valid_sha256(&collection.grid_sha256)
        || !valid_sha256(&collection.time_axis_sha256) {
        return Err("COLLECTION_SOURCE_OR_AXIS_DIGEST_INVALID");
    }
    if collection.kind == CollectionKind::ThermographyReference {
        let receipt = collection.conversion_receipt_sha256.as_deref()
            .ok_or("THERMOGRAPHY_CONVERSION_RECEIPT_MISSING")?;
        if !valid_sha256(receipt) {
            return Err("THERMOGRAPHY_CONVERSION_RECEIPT_INVALID");
        }
        let emissivity = collection.emissivity.ok_or("THERMOGRAPHY_EMISSIVITY_MISSING")?;
        if !emissivity.is_finite() || !near(emissivity, REQUIRED_EMISSIVITY, 1.0e-12) {
            return Err("THERMOGRAPHY_EMISSIVITY_NOT_NIST_050");
        }
    }
    if collection.temperature_unit != "degC" || collection.time_unit != "s" {
        return Err("NONCANONICAL_TEMPERATURE_OR_TIME_UNIT");
    }
    if g.rows == 0 || g.columns == 0 {
        return Err("GRID_DIMENSIONS_INVALID");
    }
    if !g.origin_x_um.is_finite() || !g.origin_y_um.is_finite()
        || !g.spacing_x_um.is_finite() || !g.spacing_y_um.is_finite()
        || g.spacing_x_um <= 0.0 || g.spacing_y_um <= 0.0 {
        return Err("GRID_GEOMETRY_INVALID");
    }
    if collection.profiles.is_empty() || collection.profiles.len() > MAX_OUTPUT_POINTS {
        return Err("GRID_SAMPLE_COUNT_INVALID");
    }
    if collection.profiles.len() > g.rows.saturating_mul(g.columns) {
        return Err("GRID_SAMPLE_COUNT_EXCEEDS_DECLARED_GRID");
    }
    if collection.times_s.len() < 2 {
        return Err("TIME_AXIS_TOO_SHORT");
    }
    for (index, time) in collection.times_s.iter().enumerate() {
        if !time.is_finite() {
            return Err("TIME_AXIS_NONFINITE");
        }
        if index > 0 && *time <= collection.times_s[index - 1] {
            return Err("TIME_AXIS_NOT_STRICTLY_INCREASING");
        }
    }

    let mut pixel_ids = Vec::with_capacity(collection.profiles.len());
    let mut grid_indices = Vec::with_capacity(collection.profiles.len());
    for profile in &collection.profiles {
        if !safe_token(&profile.pixel_id) {
            return Err("PIXEL_ID_MISSING_OR_UNSAFE");
        }
        if profile.row >= g.rows || profile.column >= g.columns {
            return Err("PIXEL_GRID_INDEX_OUT_OF_RANGE");
        }
        if !profile.x_um.is_finite() || !profile.y_um.is_finite() {
            return Err("PIXEL_COORDINATE_NONFINITE");
        }
        let expected_x = g.origin_x_um + profile.column as f64 * g.spacing_x_um;
        let expected_y = g.origin_y_um + profile.row as f64 * g.spacing_y_um;
        if !near(profile.x_um, expected_x, GRID_REL_TOL)
            || !near(profile.y_um, expected_y, GRID_REL_TOL) {
            return Err("PIXEL_COORDINATE_NOT_ON_DECLARED_UNIFORM_GRID");
        }
        if profile.temperatures_c.len() != collection.times_s.len() {
            return Err("PROFILE_FRAME_COUNT_MISMATCH");
        }
        if !valid_sha256(&profile.source_profile_sha256) {
            return Err("PIXEL_SOURCE_DIGEST_INVALID");
        }
        pixel_ids.push(profile.pixel_id.as_str());
        grid_indices.push((profile.row, profile.column));
    }
    pixel_ids.sort_unstable();
    if pixel_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DUPLICATE_PIXEL_ID");
    }
    grid_indices.sort_unstable();
    if grid_indices.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DUPLICATE_PIXEL_GRID_INDEX");
    }
    Ok(())
}

fn interpolate_crossing(
    t0: f64, y0: f64, t1: f64, y1: f64, threshold: f64,
) -> Result<f64, &'static str> {
    if !t0.is_finite() || !t1.is_finite() || !y0.is_finite() || !y1.is_finite()
        || !threshold.is_finite() || t1 <= t0 || y1 == y0 {
        return Err("CROSSING_INTERPOLATION_INVALID");
    }
    let fraction = (threshold - y0) / (y1 - y0);
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err("CROSSING_NOT_BRACKETED");
    }
    let crossing = t0 + fraction * (t1 - t0);
    if crossing.is_finite() {
        Ok(crossing)
    } else {
        Err("CROSSING_TIME_NONFINITE")
    }
}

fn above_threshold_intervals(
    times: &[f64], temperatures: &[f64], threshold: f64,
) -> Result<Vec<Interval>, &'static str> {
    if times.len() != temperatures.len() || times.len() < 2 {
        return Err("PROFILE_LENGTH_INVALID");
    }
    if temperatures[0] > threshold || *temperatures.last().unwrap() > threshold {
        return Err("TAM_INTERVAL_OPEN_AT_PROFILE_BOUNDARY");
    }
    let mut intervals: Vec<Interval> = Vec::new();
    for i in 0..(times.len() - 1) {
        let t0 = times[i];
        let t1 = times[i + 1];
        let y0 = temperatures[i];
        let y1 = temperatures[i + 1];
        let segment = if y0 > threshold && y1 > threshold {
            Some(Interval { start_s: t0, end_s: t1 })
        } else if y0 > threshold && y1 <= threshold {
            Some(Interval {
                start_s: t0,
                end_s: interpolate_crossing(t0, y0, t1, y1, threshold)?,
            })
        } else if y0 <= threshold && y1 > threshold {
            Some(Interval {
                start_s: interpolate_crossing(t0, y0, t1, y1, threshold)?,
                end_s: t1,
            })
        } else {
            None
        };

        if let Some(interval) = segment {
            if interval.end_s > interval.start_s {
                if let Some(previous) = intervals.last_mut() {
                    let joins_at_above_sample = near(previous.end_s, interval.start_s, TIME_JOIN_TOL)
                        && y0 > threshold;
                    if joins_at_above_sample {
                        previous.end_s = interval.end_s;
                        continue;
                    }
                }
                intervals.push(interval);
            }
        }
    }
    Ok(intervals)
}

fn select_tam_episode(
    times: &[f64], temperatures: &[f64],
) -> Result<TamEpisode, &'static str> {
    let intervals = above_threshold_intervals(times, temperatures, TAM_THRESHOLD_C)?;
    if intervals.is_empty() {
        return Err("NO_TAM_INTERVAL_ABOVE_1298C");
    }
    let mut episodes = Vec::with_capacity(intervals.len());
    for interval in intervals {
        let mut peak_index: Option<usize> = None;
        for i in 0..times.len() {
            if times[i] + TIME_JOIN_TOL < interval.start_s
                || times[i] - TIME_JOIN_TOL > interval.end_s
                || temperatures[i] <= TAM_THRESHOLD_C {
                continue;
            }
            match peak_index {
                None => peak_index = Some(i),
                Some(current) if temperatures[i] > temperatures[current] => peak_index = Some(i),
                _ => {}
            }
        }
        let peak_index = peak_index.ok_or("TAM_INTERVAL_HAS_NO_ABOVE_THRESHOLD_SAMPLE")?;
        episodes.push(TamEpisode {
            start_s: interval.start_s,
            end_s: interval.end_s,
            peak_s: times[peak_index],
            peak_index,
            duration_s: interval.end_s - interval.start_s,
        });
    }

    let mut selected = episodes[0];
    for episode in episodes.iter().copied().skip(1) {
        if episode.duration_s > selected.duration_s
            || (episode.duration_s == selected.duration_s && episode.start_s < selected.start_s) {
            selected = episode;
        }
    }
    Ok(selected)
}

fn first_descending_crossing(
    times: &[f64], temperatures: &[f64], threshold: f64, not_before_s: f64,
) -> Result<(f64, usize), &'static str> {
    for i in 0..(times.len() - 1) {
        if times[i + 1] < not_before_s {
            continue;
        }
        if temperatures[i] > threshold && temperatures[i + 1] <= threshold {
            let crossing = interpolate_crossing(
                times[i], temperatures[i], times[i + 1], temperatures[i + 1], threshold,
            )?;
            if crossing >= not_before_s {
                return Ok((crossing, i));
            }
        }
    }
    Err("NO_DESCENDING_CROSSING")
}

fn has_upward_crossing(
    times: &[f64], temperatures: &[f64], threshold: f64,
    after_s: f64, before_s: f64,
) -> Result<bool, &'static str> {
    for i in 0..(times.len() - 1) {
        if times[i + 1] < after_s || times[i] > before_s {
            continue;
        }
        if temperatures[i] <= threshold && temperatures[i + 1] > threshold {
            let crossing = interpolate_crossing(
                times[i], temperatures[i], times[i + 1], temperatures[i + 1], threshold,
            )?;
            if crossing > after_s && crossing < before_s {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn invalid_pixel(profile: &PixelProfile, reason: &'static str) -> PixelResult {
    PixelResult {
        pixel_id: profile.pixel_id.clone(),
        row: profile.row,
        column: profile.column,
        x_um: profile.x_um,
        y_um: profile.y_um,
        tam_s: None,
        scr_c_per_s: None,
        selected_tam_start_s: None,
        selected_tam_end_s: None,
        selected_peak_s: None,
        tam_reason: reason,
        scr_reason: reason,
        source_profile_sha256: profile.source_profile_sha256.clone(),
    }
}

fn extract_pixel(times: &[f64], profile: &PixelProfile, all_profiles: &[PixelProfile]) -> PixelResult {
    if profile.temperatures_c.iter().any(|sample| match sample {
        None => true,
        Some(value) => !value.is_finite(),
    }) {
        return invalid_pixel(profile, "PROFILE_HAS_MISSING_OR_NONFINITE_TEMPERATURE");
    }
    let temperatures: Vec<f64> = profile.temperatures_c.iter().map(|v| v.unwrap()).collect();
    let episode = match select_tam_episode(times, &temperatures) {
        Ok(value) => value,
        Err(reason) => return invalid_pixel(profile, reason),
    };

    let t1260 = match first_descending_crossing(times, &temperatures, SOLIDUS_C, episode.end_s) {
        Ok((time, _segment)) => time,
        Err(_) => {
            return PixelResult {
                pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
                x_um: profile.x_um, y_um: profile.y_um,
                tam_s: Some(episode.duration_s), scr_c_per_s: None,
                selected_tam_start_s: Some(episode.start_s),
                selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
                tam_reason: "VALID", scr_reason: "NO_DESCENDING_1260C_CROSSING",
                source_profile_sha256: profile.source_profile_sha256.clone(),
            };
        }
    };

    // A later above-melt episode before reaching the solidus makes the
    // cooling-rate attribution to the selected TAM spike ambiguous.
    let other_tam_before_solidus = all_profiles.iter()
        .find(|candidate| candidate.pixel_id == profile.pixel_id)
        .and_then(|_| above_threshold_intervals(times, &temperatures, TAM_THRESHOLD_C).ok())
        .map(|intervals| intervals.iter().any(|interval| {
            interval.start_s > episode.end_s && interval.start_s < t1260
        }))
        .unwrap_or(false);
    if other_tam_before_solidus {
        return PixelResult {
            pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
            x_um: profile.x_um, y_um: profile.y_um,
            tam_s: Some(episode.duration_s), scr_c_per_s: None,
            selected_tam_start_s: Some(episode.start_s),
            selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
            tam_reason: "VALID", scr_reason: "SAME_SPIKE_AMBIGUOUS_SUBSEQUENT_TAM_EPISODE",
            source_profile_sha256: profile.source_profile_sha256.clone(),
        };
    }

    let t1150 = match first_descending_crossing(times, &temperatures, SCR_LOWER_C, t1260) {
        Ok((time, _segment)) => time,
        Err(_) => {
            return PixelResult {
                pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
                x_um: profile.x_um, y_um: profile.y_um,
                tam_s: Some(episode.duration_s), scr_c_per_s: None,
                selected_tam_start_s: Some(episode.start_s),
                selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
                tam_reason: "VALID", scr_reason: "NO_DESCENDING_1150C_CROSSING",
                source_profile_sha256: profile.source_profile_sha256.clone(),
            };
        }
    };
    if t1150 <= t1260 {
        return PixelResult {
            pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
            x_um: profile.x_um, y_um: profile.y_um,
            tam_s: Some(episode.duration_s), scr_c_per_s: None,
            selected_tam_start_s: Some(episode.start_s),
            selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
            tam_reason: "VALID", scr_reason: "SCR_CROSSING_ORDER_INVALID",
            source_profile_sha256: profile.source_profile_sha256.clone(),
        };
    }
    match has_upward_crossing(times, &temperatures, SOLIDUS_C, t1260, t1150) {
        Ok(true) => {
            return PixelResult {
                pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
                x_um: profile.x_um, y_um: profile.y_um,
                tam_s: Some(episode.duration_s), scr_c_per_s: None,
                selected_tam_start_s: Some(episode.start_s),
                selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
                tam_reason: "VALID", scr_reason: "SCR_REHEATING_CROSSES_SOLIDUS",
                source_profile_sha256: profile.source_profile_sha256.clone(),
            };
        }
        Err(reason) => return invalid_pixel(profile, reason),
        Ok(false) => {}
    }

    let scr = SCR_DELTA_C / (t1150 - t1260);
    if !scr.is_finite() || scr <= 0.0 {
        return PixelResult {
            pixel_id: profile.pixel_id.clone(), row: profile.row, column: profile.column,
            x_um: profile.x_um, y_um: profile.y_um,
            tam_s: Some(episode.duration_s), scr_c_per_s: None,
            selected_tam_start_s: Some(episode.start_s),
            selected_tam_end_s: Some(episode.end_s), selected_peak_s: Some(episode.peak_s),
            tam_reason: "VALID", scr_reason: "SCR_NONPOSITIVE_OR_NONFINITE",
            source_profile_sha256: profile.source_profile_sha256.clone(),
        };
    }

    PixelResult {
        pixel_id: profile.pixel_id.clone(),
        row: profile.row,
        column: profile.column,
        x_um: profile.x_um,
        y_um: profile.y_um,
        tam_s: Some(episode.duration_s),
        scr_c_per_s: Some(scr),
        selected_tam_start_s: Some(episode.start_s),
        selected_tam_end_s: Some(episode.end_s),
        selected_peak_s: Some(episode.peak_s),
        tam_reason: "VALID",
        scr_reason: "VALID",
        source_profile_sha256: profile.source_profile_sha256.clone(),
    }
}

fn stats(values: &[f64], total_count: usize) -> MetricStats {
    if values.is_empty() {
        return MetricStats {
            total_count, valid_count: 0, missing_count: total_count,
            mean: None, median: None, sample_sd: None, population_sd: None,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let median = if count % 2 == 1 {
        sorted[count / 2]
    } else {
        (sorted[count / 2 - 1] + sorted[count / 2]) * 0.5
    };
    let sum_sq = sorted.iter().map(|value| (value - mean).powi(2)).sum::<f64>();
    let population_variance = sum_sq / count as f64;
    let sample_sd = (count > 1).then(|| (sum_sq / (count - 1) as f64).sqrt());
    MetricStats {
        total_count,
        valid_count: count,
        missing_count: total_count - count,
        mean: Some(mean),
        median: Some(median),
        sample_sd,
        population_sd: Some(population_variance.sqrt()),
    }
}

fn ks_d(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.is_empty() || right.is_empty()
        || left.iter().chain(right.iter()).any(|v| !v.is_finite()) {
        return None;
    }
    let mut a = left.to_vec();
    let mut b = right.to_vec();
    a.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);
    let mut i = 0;
    let mut j = 0;
    let mut maximum = 0.0_f64;
    while i < a.len() || j < b.len() {
        let next = match (a.get(i), b.get(j)) {
            (Some(x), Some(y)) => (*x).min(*y),
            (Some(x), None) => *x,
            (None, Some(y)) => *y,
            (None, None) => break,
        };
        while i < a.len() && a[i] <= next { i += 1; }
        while j < b.len() && b[j] <= next { j += 1; }
        let difference = (i as f64 / a.len() as f64 - j as f64 / b.len() as f64).abs();
        maximum = maximum.max(difference);
    }
    Some(maximum)
}

fn collect_values<F>(results: &[PixelResult], selector: F) -> Vec<f64>
where
    F: Fn(&PixelResult) -> Option<f64>,
{
    results.iter().filter_map(selector).collect()
}

fn process_collection(collection: &Collection) -> Result<CollectionReport, &'static str> {
    validate_collection(collection)?;
    let mut pixels: Vec<PixelResult> = collection.profiles.iter()
        .map(|profile| extract_pixel(&collection.times_s, profile, &collection.profiles))
        .collect();
    pixels.sort_by_key(|pixel| (pixel.row, pixel.column, pixel.pixel_id.clone()));
    let tam_values = collect_values(&pixels, |p| p.tam_s);
    let scr_values = collect_values(&pixels, |p| p.scr_c_per_s);
    Ok(CollectionReport {
        collection_id: collection.collection_id.clone(),
        kind: collection.kind.clone(),
        source_sha256: collection.source_sha256.clone(),
        grid_sha256: collection.grid_sha256.clone(),
        time_axis_sha256: collection.time_axis_sha256.clone(),
        dataset_doi: collection.dataset_doi.clone(),
        dataset_version: collection.dataset_version.clone(),
        conversion_receipt_sha256: collection.conversion_receipt_sha256.clone(),
        emissivity: collection.emissivity,
        temperature_unit: collection.temperature_unit.clone(),
        time_unit: collection.time_unit.clone(),
        algorithm_version: collection.algorithm_version.clone(),
        grid: collection.grid.clone(),
        frame_count: collection.times_s.len(),
        start_time_s: collection.times_s[0],
        end_time_s: *collection.times_s.last().unwrap(),
        tam: stats(&tam_values, pixels.len()),
        scr: stats(&scr_values, pixels.len()),
        pixels,
    })
}

fn same_grid(a: &Grid, b: &Grid) -> bool {
    a.rows == b.rows && a.columns == b.columns
        && near(a.origin_x_um, b.origin_x_um, GRID_REL_TOL)
        && near(a.origin_y_um, b.origin_y_um, GRID_REL_TOL)
        && near(a.spacing_x_um, b.spacing_x_um, GRID_REL_TOL)
        && near(a.spacing_y_um, b.spacing_y_um, GRID_REL_TOL)
}

fn paired_errors<F>(predicted: &[PixelResult], measured: &[PixelResult], selector: F) -> (usize, Option<f64>, Option<f64>)
where
    F: Fn(&PixelResult) -> Option<f64>,
{
    let reference: BTreeMap<(usize, usize), &PixelResult> =
        measured.iter().map(|pixel| ((pixel.row, pixel.column), pixel)).collect();
    let mut errors = Vec::new();
    for pred in predicted {
        let Some(meas) = reference.get(&(pred.row, pred.column)) else { continue; };
        if let (Some(p), Some(m)) = (selector(pred), selector(meas)) {
            errors.push(p - m);
        }
    }
    if errors.is_empty() {
        return (0, None, None);
    }
    let count = errors.len();
    let mae = errors.iter().map(|x| x.abs()).sum::<f64>() / count as f64;
    let rmse = (errors.iter().map(|x| x * x).sum::<f64>() / count as f64).sqrt();
    (count, Some(mae), Some(rmse))
}

fn compare(predicted: &Collection, measured: &Collection) -> Result<ComparisonReport, &'static str> {
    if predicted.kind != CollectionKind::ModelPrediction
        || measured.kind != CollectionKind::ThermographyReference {
        return Err("COLLECTION_KIND_MISMATCH");
    }
    let predicted_report = process_collection(predicted)?;
    let measured_report = process_collection(measured)?;
    let predicted_tam = collect_values(&predicted_report.pixels, |p| p.tam_s);
    let measured_tam = collect_values(&measured_report.pixels, |p| p.tam_s);
    let predicted_scr = collect_values(&predicted_report.pixels, |p| p.scr_c_per_s);
    let measured_scr = collect_values(&measured_report.pixels, |p| p.scr_c_per_s);
    let grid_matches = same_grid(&predicted.grid, &measured.grid);
    let (paired_tam_count, paired_tam_mae_s, paired_tam_rmse_s) = if grid_matches {
        paired_errors(&predicted_report.pixels, &measured_report.pixels, |p| p.tam_s)
    } else { (0, None, None) };
    let (paired_scr_count, paired_scr_mae_c_per_s, paired_scr_rmse_c_per_s) = if grid_matches {
        paired_errors(&predicted_report.pixels, &measured_report.pixels, |p| p.scr_c_per_s)
    } else { (0, None, None) };
    Ok(ComparisonReport {
        comparison: Comparison {
            tam_ks_d: ks_d(&predicted_tam, &measured_tam),
            scr_ks_d: ks_d(&predicted_scr, &measured_scr),
            paired_tam_count, paired_tam_mae_s, paired_tam_rmse_s,
            paired_scr_count, paired_scr_mae_c_per_s, paired_scr_rmse_c_per_s,
        },
        predicted: predicted_report,
        measured: measured_report,
        paired_grid: grid_matches,
    })
}

fn synthetic_collection(kind: CollectionKind, collection_id: &str, offset: f64, source_char: char) -> Collection {
    let profiles = vec![
        PixelProfile {
            pixel_id: "p00".into(), row: 0, column: 0, x_um: 0.0, y_um: 0.0,
            temperatures_c: vec![1000.0, 1300.0 + offset, 1400.0 + offset,
                1300.0 + offset, 1200.0 + offset, 1100.0 + offset, 1000.0]
                .into_iter().map(Some).collect(),
            source_profile_sha256: source_char.to_string().repeat(64),
        },
        PixelProfile {
            pixel_id: "p01".into(), row: 0, column: 1, x_um: 100.0, y_um: 0.0,
            temperatures_c: vec![1000.0, 1300.0 + offset, 1400.0 + offset,
                1300.0 + offset, 1190.0 + offset, 1080.0 + offset, 1000.0]
                .into_iter().map(Some).collect(),
            source_profile_sha256: source_char.to_string().repeat(64),
        },
        PixelProfile {
            pixel_id: "p10".into(), row: 1, column: 0, x_um: 0.0, y_um: 100.0,
            temperatures_c: vec![1000.0, 1400.0 + offset, 1200.0 + offset, 1000.0,
                1300.0 + offset, 1400.0 + offset, 1350.0 + offset, 1200.0 + offset,
                1100.0 + offset, 1000.0].into_iter().map(Some).collect(),
            source_profile_sha256: source_char.to_string().repeat(64),
        },
        PixelProfile {
            pixel_id: "p11".into(), row: 1, column: 1, x_um: 100.0, y_um: 100.0,
            temperatures_c: vec![1000.0, 1100.0, 1150.0, 1200.0, 1250.0, 1270.0, 1200.0]
                .into_iter().map(Some).collect(),
            source_profile_sha256: source_char.to_string().repeat(64),
        },
    ];
    let times = if profiles[2].temperatures_c.len() > profiles[0].temperatures_c.len() {
        (0..10).map(|x| x as f64).collect()
    } else {
        (0..7).map(|x| x as f64).collect()
    };
    Collection {
        collection_id: collection_id.into(),
        kind,
        dataset_doi: "10.18434/mds2-3707".into(),
        dataset_version: "synthetic-fixture-v1".into(),
        source_sha256: source_char.to_string().repeat(64),
        grid_sha256: "2".repeat(64),
        time_axis_sha256: "3".repeat(64),
        conversion_receipt_sha256: (kind == CollectionKind::ThermographyReference)
            .then(|| "d".repeat(64)),
        emissivity: (kind == CollectionKind::ThermographyReference).then_some(REQUIRED_EMISSIVITY),
        temperature_unit: "degC".into(),
        time_unit: "s".into(),
        algorithm_version: "NIST-AMB2025-07-PCRTAM-v0.1".into(),
        grid: Grid {
            rows: 2, columns: 2, origin_x_um: 0.0, origin_y_um: 0.0,
            spacing_x_um: 100.0, spacing_y_um: 100.0,
        },
        times_s: times,
        profiles,
    }
}

fn synthetic_pair() -> (Collection, Collection) {
    // Use profiles with common frame counts for the normal golden run.
    let times: Vec<f64> = (0..10).map(|x| x as f64).collect();
    let build = |kind: CollectionKind, id: &str, offset: f64, ch: char| {
        let mut collection = synthetic_collection(kind, id, offset, ch);
        collection.times_s = times.clone();
        collection.profiles.truncate(4);
        collection.grid.rows = 2;
        collection.grid.columns = 2;
        collection
    };
    let mut predicted = build(CollectionKind::ModelPrediction, "predicted-fixture", 0.0, 'a');
    let mut measured = build(CollectionKind::ThermographyReference, "measured-fixture", 2.0, 'b');
    predicted.profiles[0].temperatures_c.resize(10, Some(1000.0));
    predicted.profiles[1].temperatures_c.resize(10, Some(1000.0));
    predicted.profiles[3].temperatures_c.resize(10, Some(1000.0));
    measured.profiles[0].temperatures_c.resize(10, Some(1000.0));
    measured.profiles[1].temperatures_c.resize(10, Some(1000.0));
    measured.profiles[3].temperatures_c.resize(10, Some(1000.0));
    (predicted, measured)
}

fn verify() -> Result<(), String> {
    let times = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let temps = vec![1000.0, 1300.0, 1400.0, 1300.0, 1200.0, 1100.0, 1000.0];
    let intervals = above_threshold_intervals(&times, &temps, TAM_THRESHOLD_C)
        .map_err(str::to_string)?;
    if intervals.len() != 1 { return Err("SINGLE_SPIKE_INTERVAL_COUNT_FAILED".into()); }
    let episode = select_tam_episode(&times, &temps).map_err(str::to_string)?;
    let expected_start = 1.0 - (1300.0 - TAM_THRESHOLD_C) / 300.0;
    let expected_end = 3.0 + (1300.0 - TAM_THRESHOLD_C) / 100.0;
    if (episode.start_s - expected_start).abs() > 1.0e-12
        || (episode.end_s - expected_end).abs() > 1.0e-12 {
        return Err("TAM_LINEAR_CROSSING_KNOWN_ANSWER_FAILED".into());
    }
    let profile = PixelProfile {
        pixel_id: "single".into(), row: 0, column: 0, x_um: 0.0, y_um: 0.0,
        temperatures_c: temps.iter().copied().map(Some).collect(),
        source_profile_sha256: "a".repeat(64),
    };
    let pixel = extract_pixel(&times, &profile, std::slice::from_ref(&profile));
    if (pixel.tam_s.unwrap() - (expected_end - expected_start)).abs() > 1.0e-12
        || (pixel.scr_c_per_s.unwrap() - 100.0).abs() > 1.0e-12 {
        return Err("SAME_SPIKE_SCR_KNOWN_ANSWER_FAILED".into());
    }

    let multi_times: Vec<f64> = (0..10).map(|x| x as f64).collect();
    let multi_temps = vec![1000.0, 1400.0, 1200.0, 1000.0, 1300.0, 1400.0,
        1350.0, 1200.0, 1100.0, 1000.0];
    let multi = select_tam_episode(&multi_times, &multi_temps).map_err(str::to_string)?;
    if multi.peak_index != 5 || multi.start_s < 3.0 {
        return Err("LONGEST_TAM_EPISODE_SELECTION_FAILED".into());
    }

    let equal_times = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let equal_temps = vec![1000.0, 1400.0, 1000.0, 1000.0, 1400.0, 1000.0, 1000.0];
    let equal = select_tam_episode(&equal_times, &equal_temps).map_err(str::to_string)?;
    if equal.peak_index != 1 {
        return Err("EQUAL_DURATION_EARLIEST_TIE_BREAK_FAILED".into());
    }

    let no_solidus = vec![1000.0, 1400.0, 1300.0, 1290.0, 1280.0, 1270.0, 1265.0];
    let invalid_scr = PixelProfile {
        pixel_id: "no-scr".into(), row: 0, column: 0, x_um: 0.0, y_um: 0.0,
        temperatures_c: no_solidus.iter().copied().map(Some).collect(),
        source_profile_sha256: "b".repeat(64),
    };
    let invalid_result = extract_pixel(&times, &invalid_scr, std::slice::from_ref(&invalid_scr));
    if invalid_result.tam_s.is_none() || invalid_result.scr_c_per_s.is_some() {
        return Err("MISSING_SCR_CROSSING_MUST_NOT_ZERO_FILL".into());
    }

    let reheating = vec![1000.0, 1400.0, 1200.0, 1270.0, 1200.0, 1100.0, 1000.0];
    let reheating_profile = PixelProfile {
        pixel_id: "reheat".into(), row: 0, column: 0, x_um: 0.0, y_um: 0.0,
        temperatures_c: reheating.iter().copied().map(Some).collect(),
        source_profile_sha256: "c".repeat(64),
    };
    let reheating_result = extract_pixel(&times, &reheating_profile, std::slice::from_ref(&reheating_profile));
    if reheating_result.scr_c_per_s.is_some() {
        return Err("REHEATING_MUST_INVALIDATE_SAME_SPIKE_SCR".into());
    }

    let (predicted, measured) = synthetic_pair();
    let synthetic_comparison = compare(&predicted, &measured).map_err(str::to_string)?;
    if !synthetic_comparison.paired_grid
        || synthetic_comparison.comparison.paired_tam_count == 0
        || synthetic_comparison.comparison.paired_scr_count == 0 {
        return Err("SYNTHETIC_COLLECTION_COMPARISON_FAILED".into());
    }
    let output_rows = csv_rows(&synthetic_comparison);
    for row in &output_rows {
        if row.split(',').count() != 49 {
            return Err("CSV_COLUMN_COUNT_MISMATCH".into());
        }
    }
    if !output_rows.iter().any(|row| row.contains("TAM_INVALID_REASON:NO_TAM_INTERVAL_ABOVE_1298C")) {
        return Err("MISSING_TAM_REASON_DISTRIBUTION".into());
    }

    let ks = ks_d(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]).ok_or("KS_EMPTY")?;
    if (ks - (1.0 / 3.0)).abs() > 1.0e-12 {
        return Err("KS_D_KNOWN_ANSWER_FAILED".into());
    }

    Ok(())
}

fn reason_counts(pixels: &[PixelResult], tam: bool) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for pixel in pixels {
        let (value, reason) = if tam {
            (pixel.tam_s, pixel.tam_reason)
        } else {
            (pixel.scr_c_per_s, pixel.scr_reason)
        };
        if value.is_none() {
            *counts.entry(reason).or_insert(0) += 1;
        }
    }
    counts
}

fn csv_header() -> &'static str {
    "row_type,collection_kind,collection_id,dataset_doi,dataset_version,source_sha256,grid_sha256,time_axis_sha256,conversion_receipt_sha256,emissivity,temperature_unit,time_unit,algorithm_version,grid_rows,grid_columns,grid_origin_x_um,grid_origin_y_um,grid_spacing_x_um,grid_spacing_y_um,frame_count,start_time_s,end_time_s,pixel_id,row,column,x_um,y_um,tam_s,scr_c_per_s,tam_start_s,tam_end_s,selected_peak_s,tam_reason,scr_reason,profile_sha256,metric,total_count,valid_count,missing_count,mean,median,sample_sd,population_sd,tam_ks_d,scr_ks_d,paired_count,paired_mae,paired_rmse,paired_grid_exact"
}

fn collection_prefix(collection: &CollectionReport) -> Vec<String> {
    vec![
        collection.kind.code().to_string(),
        collection.collection_id.clone(),
        collection.dataset_doi.clone(),
        collection.dataset_version.clone(),
        collection.source_sha256.clone(),
        collection.grid_sha256.clone(),
        collection.time_axis_sha256.clone(),
        collection.conversion_receipt_sha256.clone().unwrap_or_default(),
        collection.emissivity.map(|x| format!("{x:.12e}")).unwrap_or_default(),
        collection.temperature_unit.clone(),
        collection.time_unit.clone(),
        collection.algorithm_version.clone(),
        collection.grid.rows.to_string(),
        collection.grid.columns.to_string(),
        format!("{:.12e}", collection.grid.origin_x_um),
        format!("{:.12e}", collection.grid.origin_y_um),
        format!("{:.12e}", collection.grid.spacing_x_um),
        format!("{:.12e}", collection.grid.spacing_y_um),
        collection.frame_count.to_string(),
        format!("{:.12e}", collection.start_time_s),
        format!("{:.12e}", collection.end_time_s),
    ]
}

fn csv_rows(report: &ComparisonReport) -> Vec<String> {
    let mut rows = Vec::new();
    for collection in [&report.predicted, &report.measured] {
        let prefix = collection_prefix(collection);
        for pixel in &collection.pixels {
            let mut cells = vec!["PIXEL".to_string()];
            cells.extend(prefix.clone());
            cells.extend([
                pixel.pixel_id.clone(), pixel.row.to_string(), pixel.column.to_string(),
                format!("{:.12e}", pixel.x_um), format!("{:.12e}", pixel.y_um),
                pixel.tam_s.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                pixel.scr_c_per_s.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                pixel.selected_tam_start_s.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                pixel.selected_tam_end_s.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                pixel.selected_peak_s.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                pixel.tam_reason.to_string(), pixel.scr_reason.to_string(),
                pixel.source_profile_sha256.clone(),
            ]);
            cells.resize(49, String::new());
            rows.push(cells.join(","));
        }
        for (metric, stat) in [("TAM", collection.tam), ("SCR", collection.scr)] {
            let mut cells = vec!["SUMMARY".to_string()];
            cells.extend(prefix.clone());
            cells.resize(35, String::new());
            cells.extend([
                metric.to_string(), stat.total_count.to_string(), stat.valid_count.to_string(),
                stat.missing_count.to_string(),
                stat.mean.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                stat.median.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                stat.sample_sd.map(|x| format!("{x:.12e}")).unwrap_or_default(),
                stat.population_sd.map(|x| format!("{x:.12e}")).unwrap_or_default(),
            ]);
            cells.resize(49, String::new());
            rows.push(cells.join(","));
        }
        for (metric, reason_counts) in [
            ("TAM", reason_counts(&collection.pixels, true)),
            ("SCR", reason_counts(&collection.pixels, false)),
        ] {
            for (reason, count) in reason_counts {
                let mut cells = vec!["REASON_COUNT".to_string()];
                cells.extend(prefix.clone());
                cells.resize(35, String::new());
                cells.extend([
                    format!("{metric}_INVALID_REASON:{reason}"),
                    collection.pixels.len().to_string(),
                    String::new(),
                    count.to_string(),
                ]);
                cells.resize(49, String::new());
                rows.push(cells.join(","));
            }
        }
    }

    let comparison_rows = [
        ("TAM", report.comparison.tam_ks_d, report.comparison.paired_tam_count,
            report.comparison.paired_tam_mae_s, report.comparison.paired_tam_rmse_s),
        ("SCR", report.comparison.scr_ks_d, report.comparison.paired_scr_count,
            report.comparison.paired_scr_mae_c_per_s, report.comparison.paired_scr_rmse_c_per_s),
    ];
    for (metric, ks, paired_count, mae, rmse) in comparison_rows {
        let mut cells = vec!["COMPARISON".to_string()];
        cells.resize(35, String::new());
        cells.push(metric.to_string());
        cells.resize(45, String::new());
        if metric == "TAM" {
            cells[43] = ks.map(|x| format!("{x:.12e}")).unwrap_or_default();
        } else {
            cells[44] = ks.map(|x| format!("{x:.12e}")).unwrap_or_default();
        }
        cells.push(paired_count.to_string());
        cells.push(mae.map(|x| format!("{x:.12e}")).unwrap_or_default());
        cells.push(rmse.map(|x| format!("{x:.12e}")).unwrap_or_default());
        cells.push(report.paired_grid.to_string());
        cells.resize(49, String::new());
        rows.push(cells.join(","));
    }
    rows
}

fn emit_csv() -> Result<(), String> {
    let (predicted, measured) = synthetic_pair();
    let report = compare(&predicted, &measured).map_err(str::to_string)?;
    println!("{}", csv_header());
    for row in csv_rows(&report) { println!("{row}"); }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("AMBENCH_PCRTAM_OK: NIST TAM/SCR extraction and comparison reference")),
        [arg] if arg == "--verify" => verify().map(|()| println!("AMBENCH_PCRTAM_OK: NIST TAM/SCR extraction and comparison reference")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: pcrt-07 [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("AMBENCH_PCRTAM_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, temps: &[f64]) -> PixelProfile {
        PixelProfile {
            pixel_id: id.into(), row: 0, column: 0, x_um: 0.0, y_um: 0.0,
            temperatures_c: temps.iter().copied().map(Some).collect(),
            source_profile_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn exact_single_spike_tam_and_scr_known_answer() {
        verify().unwrap();
    }

    #[test]
    fn exact_threshold_samples_use_strict_above_semantics() {
        let times = vec![0.0, 1.0, 2.0, 3.0];
        let temps = vec![1298.0, 1400.0, 1298.0, 1200.0];
        let intervals = above_threshold_intervals(&times, &temps, TAM_THRESHOLD_C).unwrap();
        assert_eq!(intervals.len(), 1);
        assert!((intervals[0].start_s - 0.0).abs() < 1.0e-12);
        assert!((intervals[0].end_s - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn profile_open_above_threshold_at_first_or_last_frame_abstains() {
        let times = vec![0.0, 1.0, 2.0];
        assert_eq!(
            above_threshold_intervals(&times, &[1300.0, 1400.0, 1000.0], TAM_THRESHOLD_C),
            Err("TAM_INTERVAL_OPEN_AT_PROFILE_BOUNDARY")
        );
        assert_eq!(
            above_threshold_intervals(&times, &[1000.0, 1400.0, 1300.0], TAM_THRESHOLD_C),
            Err("TAM_INTERVAL_OPEN_AT_PROFILE_BOUNDARY")
        );
    }

    #[test]
    fn missing_frame_abstains_instead_of_interpolating() {
        let times = vec![0.0, 1.0, 2.0, 3.0];
        let mut p = profile("missing", &[1000.0, 1400.0, 1200.0, 1100.0]);
        p.temperatures_c[2] = None;
        let result = extract_pixel(&times, &p, std::slice::from_ref(&p));
        assert!(result.tam_s.is_none());
        assert_eq!(result.tam_reason, "PROFILE_HAS_MISSING_OR_NONFINITE_TEMPERATURE");
    }

    #[test]
    fn nonfinite_temperature_abstains() {
        let times = vec![0.0, 1.0, 2.0];
        let mut p = profile("bad", &[1000.0, 1400.0, 1200.0]);
        p.temperatures_c[1] = Some(f64::NAN);
        let result = extract_pixel(&times, &p, std::slice::from_ref(&p));
        assert!(result.scr_c_per_s.is_none());
    }

    #[test]
    fn duplicate_and_nonmonotonic_times_reject_collection() {
        let (mut predicted, measured) = synthetic_pair();
        predicted.times_s[2] = predicted.times_s[1];
        assert_eq!(validate_collection(&predicted), Err("TIME_AXIS_NOT_STRICTLY_INCREASING"));
        assert!(validate_collection(&measured).is_ok());
    }

    #[test]
    fn nonuniform_grid_and_duplicate_pixel_indices_reject() {
        let (mut predicted, _) = synthetic_pair();
        predicted.profiles[1].x_um = 101.0;
        assert_eq!(validate_collection(&predicted), Err("PIXEL_COORDINATE_NOT_ON_DECLARED_UNIFORM_GRID"));

        let (mut predicted, _) = synthetic_pair();
        predicted.profiles[1].column = 0;
        predicted.profiles[1].x_um = 0.0;
        assert_eq!(validate_collection(&predicted), Err("DUPLICATE_PIXEL_GRID_INDEX"));
    }

    #[test]
    fn thermography_manifest_requires_nist_emissivity_and_conversion_receipt() {
        let (_, mut measured) = synthetic_pair();
        measured.emissivity = Some(0.6);
        assert_eq!(validate_collection(&measured), Err("THERMOGRAPHY_EMISSIVITY_NOT_NIST_050"));
        measured.emissivity = Some(0.5);
        measured.conversion_receipt_sha256 = None;
        assert_eq!(validate_collection(&measured), Err("THERMOGRAPHY_CONVERSION_RECEIPT_MISSING"));
    }

    #[test]
    fn ks_d_matches_two_sample_empirical_cdf_known_answer() {
        assert!((ks_d(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]).unwrap() - 1.0 / 3.0).abs() < 1.0e-12);
        assert_eq!(ks_d(&[], &[1.0]), None);
    }

    #[test]
    fn missing_spatial_alignment_disables_paired_metrics_but_keeps_ks() {
        let (predicted, mut measured) = synthetic_pair();
        measured.grid.spacing_x_um = 120.0;
        let result = compare(&predicted, &measured).unwrap();
        assert!(!result.paired_grid);
        assert_eq!(result.comparison.paired_tam_count, 0);
        assert!(result.comparison.tam_ks_d.is_some());
    }

    #[test]
    fn paired_spatial_metrics_only_use_same_grid_cells() {
        let (predicted, measured) = synthetic_pair();
        let result = compare(&predicted, &measured).unwrap();
        assert!(result.paired_grid);
        assert!(result.comparison.paired_tam_count > 0);
    }

    #[test]
    fn deterministic_repeat() {
        let (predicted, measured) = synthetic_pair();
        let a = compare(&predicted, &measured).unwrap();
        let b = compare(&predicted, &measured).unwrap();
        assert_eq!(a.comparison.tam_ks_d, b.comparison.tam_ks_d);
        assert_eq!(csv_rows(&a), csv_rows(&b));
    }

    #[test]
    fn strict_sample_limit_rejects_over_one_million_pixels() {
        let mut collection = synthetic_collection(CollectionKind::ModelPrediction, "too-large", 0.0, 'a');
        collection.grid.rows = 1001;
        collection.grid.columns = 1000;
        collection.profiles = (0..=MAX_OUTPUT_POINTS).map(|i| {
            PixelProfile {
                pixel_id: format!("p{i}"), row: 0, column: i, x_um: i as f64 * 100.0,
                y_um: 0.0, temperatures_c: vec![Some(1000.0); collection.times_s.len()],
                source_profile_sha256: "a".repeat(64),
            }
        }).collect();
        assert_eq!(validate_collection(&collection), Err("GRID_SAMPLE_COUNT_INVALID"));
    }
}
