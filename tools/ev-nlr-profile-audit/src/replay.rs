use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
};

pub const EXPECTED_SOURCE_REVISION: &str = "61530df51f6cd7d64f014ebbf8714cef24c8ca70";
const PYTHON_MASTER_SEED: u64 = 0;
const N_SAMPLES: usize = 50;
const KWH_PER_MILE: f64 = 1.8;
const CHARGER_POWER_KW: f64 = 100.0;
const DAY_SECONDS: usize = 86_399;
const MIDDLE_DAY_START: usize = DAY_SECONDS;
const MIDDLE_DAY_END: usize = DAY_SECONDS * 2;
const QUARTER_HOUR_SECONDS: usize = 900;

const FLEETS: [&str; 3] = [
    "fleet1-beverage-delivery",
    "fleet2-warehouse-delivery",
    "fleet3-food-delivery",
];
const FLEET_SIZES: [usize; 3] = [10, 50, 100];
const STRATEGIES: [&str; 3] = ["immediate", "delayed", "min_power"];
const PROFILE_KINDS: [&str; 3] = ["avg-prof", "min-prof", "peak-prof"];

#[derive(Clone)]
struct OperationDay {
    id: String,
    vmt: f64,
    time_on_shift_s: u64,
    time_off_shift_s: u64,
}

#[derive(Clone)]
struct ScheduleSegment {
    total_time_s: u64,
    on_shift: bool,
    vmt: f64,
}

struct FleetInputs {
    fleet_id: String,
    operation_days: Vec<OperationDay>,
    schedules: HashMap<String, Vec<ScheduleSegment>>,
    operation_days_sha256: String,
    schedules_sha256: String,
    operation_days_bytes: u64,
    schedules_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct OperationDayRow {
    veh_op_day_id: String,
    vmt: f64,
    time_on_shift_s: u64,
    time_off_shift_s: u64,
}

#[derive(Debug, Deserialize)]
struct ScheduleSegmentRow {
    veh_op_day_id: String,
    total_time_s: u64,
    on_shift: u8,
    vmt: f64,
}

#[derive(Debug, Serialize)]
struct SourceInputDigest {
    fleet_id: String,
    operation_days_path: String,
    operation_days_sha256: String,
    operation_days_bytes: u64,
    operation_day_count: usize,
    schedules_path: String,
    schedules_sha256: String,
    schedules_bytes: u64,
    schedule_segment_count: usize,
}

#[derive(Debug, Serialize)]
struct ProfileOutput {
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_kind: String,
    file_name: String,
    sha256: String,
    interval_count: usize,
    mean_power_kw: f64,
    peak_power_kw: f64,
    daily_energy_kwh: f64,
}

#[derive(Debug, Serialize)]
struct ReplayReport {
    schema_version: String,
    tool: String,
    source_repository: String,
    source_revision: String,
    source_script_sha256: String,
    source_script_bytes: u64,
    source_input_files: Vec<SourceInputDigest>,
    python_master_seed: u64,
    python_seed_list_sha256: String,
    python_rng_protocol: String,
    numpy_rng_protocol: String,
    n_samples: usize,
    fleet_sizes: Vec<usize>,
    charging_strategies: Vec<String>,
    profile_kinds: Vec<String>,
    vehicle_energy_kwh_per_mile: f64,
    constant_power_charger_kw: f64,
    daily_source_seconds: usize,
    output_intervals: usize,
    output_interval_minutes: usize,
    generated_profile_count: usize,
    generated_total_interval_count: usize,
    archive_comparison_state: String,
    candidate_evidence_class: String,
    profiles: Vec<ProfileOutput>,
    claim_ceiling: String,
}

#[derive(Debug, Serialize)]
pub struct ReplayEnvelope {
    pub report: ReplayReport,
    pub report_sha256: String,
}

struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new() -> Self {
        Self {
            state: [0; 624],
            index: 624,
        }
    }

    fn seeded_for_python(seed: u32) -> Self {
        let mut rng = Self::new();
        rng.init_by_array(&[seed]);
        rng
    }

    fn seeded_for_numpy(seed: u32) -> Self {
        let mut rng = Self::new();
        rng.init_genrand(seed);
        rng
    }

    fn init_genrand(&mut self, seed: u32) {
        self.state[0] = seed;
        for index in 1..624 {
            let previous = self.state[index - 1];
            self.state[index] = 1_812_433_253_u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(index as u32);
        }
        self.index = 624;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = 624usize.max(key.len());
        while k > 0 {
            let previous = self.state[i - 1];
            self.state[i] = (self.state[i]
                ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                self.state[0] = self.state[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }

        k = 623;
        while k > 0 {
            let previous = self.state[i - 1];
            self.state[i] = (self.state[i]
                ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                self.state[0] = self.state[623];
                i = 1;
            }
            k -= 1;
        }
        self.state[0] = 0x8000_0000;
        self.index = 624;
    }

    fn twist(&mut self) {
        for index in 0..227 {
            let y = (self.state[index] & 0x8000_0000)
                | (self.state[index + 1] & 0x7fff_ffff);
            self.state[index] = self.state[index + 397]
                ^ (y >> 1)
                ^ if y & 1 == 0 { 0 } else { 0x9908_b0df };
        }
        for index in 227..623 {
            let y = (self.state[index] & 0x8000_0000)
                | (self.state[index + 1] & 0x7fff_ffff);
            self.state[index] = self.state[index - 227]
                ^ (y >> 1)
                ^ if y & 1 == 0 { 0 } else { 0x9908_b0df };
        }
        let y = (self.state[623] & 0x8000_0000) | (self.state[0] & 0x7fff_ffff);
        self.state[623] = self.state[396]
            ^ (y >> 1)
            ^ if y & 1 == 0 { 0 } else { 0x9908_b0df };
        self.index = 0;
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c_5680;
        value ^= (value << 15) & 0xefc6_0000;
        value ^= value >> 18;
        value
    }

    fn python_randbelow(&mut self, upper_exclusive: u32) -> u32 {
        let bits = 32 - upper_exclusive.leading_zeros();
        loop {
            let candidate = self.next_u32() >> (32 - bits);
            if candidate < upper_exclusive {
                return candidate;
            }
        }
    }

    fn numpy_interval(&mut self, maximum_inclusive: u32) -> u32 {
        let mut mask = maximum_inclusive;
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask |= mask >> 8;
        mask |= mask >> 16;
        loop {
            let candidate = self.next_u32() & mask;
            if candidate <= maximum_inclusive {
                return candidate;
            }
        }
    }
}

fn python_master_seeds(seed: u64, count: usize) -> Result<Vec<u32>, String> {
    let seed = u32::try_from(seed)
        .map_err(|_| "Python-compatible master seed must fit in u32".to_owned())?;
    let mut rng = Mt19937::seeded_for_python(seed);
    Ok((0..count)
        .map(|_| rng.python_randbelow(1000))
        .collect())
}

fn numpy_choice_indices(
    seed: u32,
    population: usize,
    sample_size: usize,
) -> Result<Vec<usize>, String> {
    if population == 0 {
        return Err("cannot sample an empty operation-day population".to_owned());
    }
    let maximum = u32::try_from(population - 1)
        .map_err(|_| "operation-day population exceeds NumPy RandomState v0.1 range".to_owned())?;
    let mut rng = Mt19937::seeded_for_numpy(seed);
    Ok((0..sample_size)
        .map(|_| rng.numpy_interval(maximum) as usize)
        .collect())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_csv<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<(Vec<T>, Vec<u8>), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read source CSV {}: {error}", path.display()))?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(bytes.as_slice());
    let rows = reader
        .deserialize::<T>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot parse source CSV {}: {error}", path.display()))?;
    Ok((rows, bytes))
}

fn load_fleet_inputs(root: &Path, fleet_id: &str) -> Result<FleetInputs, String> {
    let directory = root.join(fleet_id);
    let op_path = directory.join("veh_op_days.csv");
    let schedule_path = directory.join("veh_schedules.csv");
    let (operation_rows, op_bytes) = read_csv::<OperationDayRow>(&op_path)?;
    let (schedule_rows, schedule_bytes) = read_csv::<ScheduleSegmentRow>(&schedule_path)?;
    if operation_rows.is_empty() || schedule_rows.is_empty() {
        return Err(format!("{fleet_id}: source schedule files must not be empty"));
    }

    let mut seen_days = BTreeSet::new();
    let mut operation_days = Vec::with_capacity(operation_rows.len());
    for row in operation_rows {
        if row.veh_op_day_id.trim().is_empty()
            || !seen_days.insert(row.veh_op_day_id.clone())
            || !row.vmt.is_finite()
            || row.vmt < 0.0
            || row.time_on_shift_s + row.time_off_shift_s != DAY_SECONDS as u64
        {
            return Err(format!(
                "{fleet_id}: invalid or duplicate operation-day row {}",
                row.veh_op_day_id
            ));
        }
        operation_days.push(OperationDay {
            id: row.veh_op_day_id,
            vmt: row.vmt,
            time_on_shift_s: row.time_on_shift_s,
            time_off_shift_s: row.time_off_shift_s,
        });
    }

    let mut schedules: HashMap<String, Vec<ScheduleSegment>> = HashMap::new();
    for row in schedule_rows {
        if row.veh_op_day_id.trim().is_empty()
            || row.total_time_s == 0
            || row.on_shift > 1
            || !row.vmt.is_finite()
            || row.vmt < 0.0
            || (row.on_shift == 0 && row.vmt > 1e-9)
        {
            return Err(format!(
                "{fleet_id}: malformed schedule segment for operation day {}",
                row.veh_op_day_id
            ));
        }
        schedules
            .entry(row.veh_op_day_id)
            .or_default()
            .push(ScheduleSegment {
                total_time_s: row.total_time_s,
                on_shift: row.on_shift == 1,
                vmt: row.vmt,
            });
    }

    for day in &operation_days {
        let segments = schedules
            .get(&day.id)
            .ok_or_else(|| format!("{fleet_id}: missing schedule for operation day {}", day.id))?;
        let total_seconds = segments.iter().map(|segment| segment.total_time_s).sum::<u64>();
        let on_shift_seconds = segments
            .iter()
            .filter(|segment| segment.on_shift)
            .map(|segment| segment.total_time_s)
            .sum::<u64>();
        let off_shift_seconds = total_seconds - on_shift_seconds;
        let schedule_vmt = segments
            .iter()
            .filter(|segment| segment.on_shift)
            .map(|segment| segment.vmt)
            .sum::<f64>();
        if total_seconds != DAY_SECONDS as u64
            || on_shift_seconds != day.time_on_shift_s
            || off_shift_seconds != day.time_off_shift_s
            || (schedule_vmt - day.vmt).abs() > 1e-6_f64.max(day.vmt.abs() * 1e-9)
        {
            return Err(format!(
                "{fleet_id}: operation-day {} does not reconcile with its schedule",
                day.id
            ));
        }
    }
    if schedules.len() != operation_days.len() {
        return Err(format!("{fleet_id}: schedule table contains unknown operation-day IDs"));
    }

    Ok(FleetInputs {
        fleet_id: fleet_id.to_owned(),
        operation_days,
        schedules,
        operation_days_sha256: sha256(&op_bytes),
        schedules_sha256: sha256(&schedule_bytes),
        operation_days_bytes: op_bytes.len() as u64,
        schedules_bytes: schedule_bytes.len() as u64,
    })
}

fn add_interval(delta: &mut [f64], start: usize, end: usize, power_kw: f64) {
    if start >= end || start >= delta.len() - 1 {
        return;
    }
    let clipped_end = end.min(delta.len() - 1);
    delta[start] += power_kw;
    delta[clipped_end] -= power_kw;
}

fn add_middle_day_interval(delta: &mut [f64], start: usize, end: usize, power_kw: f64) {
    let clipped_start = start.max(MIDDLE_DAY_START);
    let clipped_end = end.min(MIDDLE_DAY_END);
    if clipped_start < clipped_end {
        add_interval(
            delta,
            clipped_start - MIDDLE_DAY_START,
            clipped_end - MIDDLE_DAY_START,
            power_kw,
        );
    }
}

fn add_vehicle_immediate_or_delayed(
    delta: &mut [f64],
    day: &OperationDay,
    schedule: &[ScheduleSegment],
    delayed: bool,
) -> Result<(), String> {
    let mut cursor = 0usize;
    let mut remaining_energy_kwh = 0.0_f64;
    let mut row_index = 0usize;
    while row_index < schedule.len() * 3 {
        let shift_state = schedule[row_index % schedule.len()].on_shift;
        let mut duration_s = 0_u64;
        let mut distance_miles = 0.0_f64;
        while row_index < schedule.len() * 3 {
            let current = &schedule[row_index % schedule.len()];
            if current.on_shift != shift_state {
                break;
            }
            duration_s += current.total_time_s;
            distance_miles += current.vmt;
            row_index += 1;
        }

        if shift_state {
            remaining_energy_kwh += distance_miles * KWH_PER_MILE;
        } else {
            let required_seconds =
                (remaining_energy_kwh / CHARGER_POWER_KW * 3600.0).ceil() as u64;
            if required_seconds > duration_s && duration_s > 0 {
                add_middle_day_interval(
                    delta,
                    cursor,
                    cursor + duration_s as usize,
                    CHARGER_POWER_KW,
                );
                remaining_energy_kwh -=
                    CHARGER_POWER_KW * duration_s as f64 / 3600.0;
            } else {
                remaining_energy_kwh = 0.0;
                if required_seconds > duration_s {
                    return Err(format!(
                        "operation day {} has zero dwell with positive charge demand",
                        day.id
                    ));
                }
                let charge_start = if delayed {
                    cursor + duration_s as usize - required_seconds as usize
                } else {
                    cursor
                };
                add_middle_day_interval(
                    delta,
                    charge_start,
                    charge_start + required_seconds as usize,
                    CHARGER_POWER_KW,
                );
            }
        }
        cursor += duration_s as usize;
    }
    if cursor != DAY_SECONDS * 3 {
        return Err(format!(
            "operation day {} repeated schedule has {} seconds, expected {}",
            day.id,
            cursor,
            DAY_SECONDS * 3
        ));
    }
    Ok(())
}

fn add_vehicle_min_power(
    delta: &mut [f64],
    day: &OperationDay,
    schedule: &[ScheduleSegment],
) -> Result<(), String> {
    if day.time_off_shift_s == 0 {
        return Err(format!(
            "operation day {} has no available off-shift time",
            day.id
        ));
    }
    let minimum_power_kw =
        day.vmt * KWH_PER_MILE / (day.time_off_shift_s as f64 / 3600.0);
    let mut cursor = 0usize;
    for segment in schedule {
        let end = cursor + segment.total_time_s as usize;
        if !segment.on_shift {
            add_interval(delta, cursor, end, minimum_power_kw);
        }
        cursor = end;
    }
    if cursor != DAY_SECONDS {
        return Err(format!(
            "operation day {} schedule has {} seconds, expected {}",
            day.id, cursor, DAY_SECONDS
        ));
    }
    Ok(())
}

fn cumulative_profile(delta: &[f64]) -> Vec<f64> {
    let mut load = 0.0_f64;
    let mut output = Vec::with_capacity(delta.len().saturating_sub(1));
    for increment in delta.iter().take(delta.len().saturating_sub(1)) {
        load += increment;
        if load.abs() < 1e-12 {
            load = 0.0;
        }
        output.push(load);
    }
    output
}

fn quarter_hour_means(profile: &[f64]) -> Result<Vec<f64>, String> {
    if profile.len() != DAY_SECONDS {
        return Err(format!(
            "daily profile contains {} seconds, expected {DAY_SECONDS}",
            profile.len()
        ));
    }
    let mut padded = Vec::with_capacity(DAY_SECONDS + 1);
    padded.push(profile[0]);
    padded.extend_from_slice(profile);
    let result = padded
        .chunks_exact(QUARTER_HOUR_SECONDS)
        .map(|interval| interval.iter().sum::<f64>() / QUARTER_HOUR_SECONDS as f64)
        .collect::<Vec<_>>();
    if result.len() != 96 {
        return Err(format!("quarter-hour aggregation produced {} intervals", result.len()));
    }
    Ok(result)
}

struct ProfileKey {
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_kind: String,
}

impl ProfileKey {
    fn file_name(&self) -> String {
        format!(
            "{}_{}vehs_{}_{}_15min.csv",
            self.fleet_id, self.fleet_size, self.profile_kind, self.strategy
        )
    }
}

fn write_profile_csv(
    output_dir: &Path,
    key: &ProfileKey,
    profile: &[f64],
) -> Result<ProfileOutput, String> {
    let values = quarter_hour_means(profile)?;
    let file_name = key.file_name();
    let path = output_dir.join(&file_name);
    let mut content = String::from("time,avg_power_kw
");
    for (index, value) in values.iter().enumerate() {
        let minutes = index * 15;
        let hour = minutes / 60;
        let minute = minutes % 60;
        content.push_str(&format!("{hour:02}:{minute:02}:00,{value:.12}
"));
    }
    let bytes = content.as_bytes();
    fs::write(&path, bytes)
        .map_err(|error| format!("cannot write profile {}: {error}", path.display()))?;
    Ok(ProfileOutput {
        fleet_id: key.fleet_id.clone(),
        fleet_size: key.fleet_size,
        strategy: key.strategy.clone(),
        profile_kind: key.profile_kind.clone(),
        file_name,
        sha256: sha256(bytes),
        interval_count: values.len(),
        mean_power_kw: profile.iter().sum::<f64>() / profile.len() as f64,
        peak_power_kw: profile.iter().copied().fold(0.0_f64, f64::max),
        daily_energy_kwh: profile.iter().sum::<f64>() / 3600.0,
    })
}

#[derive(Debug, Serialize)]
struct SourceInputDigest {
    fleet_id: String,
    operation_days_path: String,
    operation_days_sha256: String,
    operation_days_bytes: u64,
    operation_day_count: usize,
    schedules_path: String,
    schedules_sha256: String,
    schedules_bytes: u64,
    schedule_segment_count: usize,
}

#[derive(Debug, Serialize)]
struct ProfileOutput {
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_kind: String,
    file_name: String,
    sha256: String,
    interval_count: usize,
    mean_power_kw: f64,
    peak_power_kw: f64,
    daily_energy_kwh: f64,
}

#[derive(Debug, Serialize)]
struct ReplayReport {
    schema_version: String,
    tool: String,
    source_repository: String,
    source_revision: String,
    source_script_sha256: String,
    source_script_bytes: u64,
    source_input_files: Vec<SourceInputDigest>,
    python_master_seed: u64,
    python_seed_list_sha256: String,
    python_rng_protocol: String,
    numpy_rng_protocol: String,
    n_samples: usize,
    fleet_sizes: Vec<usize>,
    charging_strategies: Vec<String>,
    profile_kinds: Vec<String>,
    vehicle_energy_kwh_per_mile: f64,
    constant_power_charger_kw: f64,
    daily_source_seconds: usize,
    output_intervals: usize,
    output_interval_minutes: usize,
    generated_profile_count: usize,
    generated_total_interval_count: usize,
    archive_comparison_state: String,
    candidate_evidence_class: String,
    profiles: Vec<ProfileOutput>,
    claim_ceiling: String,
}

#[derive(Debug, Serialize)]
pub struct ReplayEnvelope {
    pub report: ReplayReport,
    pub report_sha256: String,
}

fn generate_candidate_profiles(
    inputs: &FleetInputs,
    fleet_size: usize,
    strategy: &str,
    seeds: &[u32],
) -> Result<BTreeMap<String, Vec<f64>>, String> {
    let mut total_profile = vec![0.0_f64; DAY_SECONDS];
    let mut max_profile: Option<Vec<f64>> = None;
    let mut min_profile: Option<Vec<f64>> = None;
    let mut max_peak = f64::NEG_INFINITY;
    let mut min_peak = f64::INFINITY;

    for seed in seeds {
        let choices = numpy_choice_indices(*seed, inputs.operation_days.len(), fleet_size)?;
        let mut fleet_delta = vec![0.0_f64; DAY_SECONDS + 1];
        for choice in choices {
            let day = &inputs.operation_days[choice];
            let schedule = inputs
                .schedules
                .get(&day.id)
                .ok_or_else(|| format!("{}: missing schedule for {}", inputs.fleet_id, day.id))?;
            match strategy {
                "immediate" => {
                    add_vehicle_immediate_or_delayed(&mut fleet_delta, day, schedule, false)?;
                }
                "delayed" => {
                    add_vehicle_immediate_or_delayed(&mut fleet_delta, day, schedule, true)?;
                }
                "min_power" => add_vehicle_min_power(&mut fleet_delta, day, schedule)?,
                _ => return Err(format!("unknown NLR charging strategy {strategy}")),
            }
        }
        let daily = cumulative_profile(&fleet_delta);
        for (sum, value) in total_profile.iter_mut().zip(daily.iter()) {
            *sum += value;
        }
        let sample_peak = daily.iter().copied().fold(0.0_f64, f64::max);
        if sample_peak > max_peak {
            max_peak = sample_peak;
            max_profile = Some(daily.clone());
        }
        if sample_peak < min_peak {
            min_peak = sample_peak;
            min_profile = Some(daily);
        }
    }

    let average = total_profile
        .into_iter()
        .map(|value| value / seeds.len() as f64)
        .collect::<Vec<_>>();
    let mut profiles = BTreeMap::new();
    profiles.insert("avg-prof".to_owned(), average);
    profiles.insert(
        "min-prof".to_owned(),
        min_profile.ok_or_else(|| "minimum sample profile was not generated".to_owned())?,
    );
    profiles.insert(
        "peak-prof".to_owned(),
        max_profile.ok_or_else(|| "maximum sample profile was not generated".to_owned())?,
    );
    Ok(profiles)
}

fn load_all_inputs(root: &Path) -> Result<Vec<FleetInputs>, String> {
    FLEETS
        .iter()
        .map(|fleet_id| load_fleet_inputs(root, fleet_id))
        .collect()
}

pub fn run_source_replay(
    schedule_root: &Path,
    source_script_path: &Path,
    output_dir: &Path,
    source_revision: &str,
    report_path: &Path,
) -> Result<ReplayEnvelope, String> {
    if source_revision != EXPECTED_SOURCE_REVISION {
        return Err(format!(
            "source revision must be pinned to {EXPECTED_SOURCE_REVISION}"
        ));
    }
    let script_bytes = fs::read(source_script_path)
        .map_err(|error| format!("cannot read pinned upstream generator script: {error}"))?;
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("cannot create output directory: {error}"))?;
    let inputs = load_all_inputs(schedule_root)?;
    let seeds = python_master_seeds(PYTHON_MASTER_SEED, N_SAMPLES)?;
    let seed_bytes = serde_json::to_vec(&seeds)
        .map_err(|error| format!("cannot serialize Python-compatible seed list: {error}"))?;
    let seed_digest = sha256(&seed_bytes);

    let mut source_input_files = Vec::new();
    for input in &inputs {
        source_input_files.push(SourceInputDigest {
            fleet_id: input.fleet_id.clone(),
            operation_days_path: format!(
                "data/fleet-schedules/{}/veh_op_days.csv",
                input.fleet_id
            ),
            operation_days_sha256: input.operation_days_sha256.clone(),
            operation_days_bytes: input.operation_days_bytes,
            operation_day_count: input.operation_days.len(),
            schedules_path: format!(
                "data/fleet-schedules/{}/veh_schedules.csv",
                input.fleet_id
            ),
            schedules_sha256: input.schedules_sha256.clone(),
            schedules_bytes: input.schedules_bytes,
            schedule_segment_count: input.schedules.values().map(Vec::len).sum(),
        });
    }

    let mut profiles = Vec::with_capacity(81);
    for input in &inputs {
        for fleet_size in FLEET_SIZES {
            for strategy in STRATEGIES {
                let generated =
                    generate_candidate_profiles(input, fleet_size, strategy, &seeds)?;
                for profile_kind in PROFILE_KINDS {
                    let values = generated
                        .get(profile_kind)
                        .ok_or_else(|| format!("generated profile kind {profile_kind} missing"))?;
                    let key = ProfileKey {
                        fleet_id: input.fleet_id.clone(),
                        fleet_size,
                        strategy: strategy.to_owned(),
                        profile_kind: profile_kind.to_owned(),
                    };
                    profiles.push(write_profile_csv(output_dir, &key, values)?);
                }
            }
        }
    }
    profiles.sort_by(|left, right| left.file_name.cmp(&right.file_name));

    let report = ReplayReport {
        schema_version: "ev-nlr-source-replay/v0.1".to_owned(),
        tool: "ev-nlr-source-replay".to_owned(),
        source_repository: "https://github.com/NatLabRockies/hdev-depot-charging-2021".to_owned(),
        source_revision: source_revision.to_owned(),
        source_script_sha256: sha256(&script_bytes),
        source_script_bytes: script_bytes.len() as u64,
        source_input_files,
        python_master_seed: PYTHON_MASTER_SEED,
        python_seed_list_sha256: seed_digest,
        python_rng_protocol:
            "CPython random.Random integer seed via MT19937 init_by_array; randint(0,999) via getrandbits rejection sampling"
                .to_owned(),
        numpy_rng_protocol:
            "NumPy legacy RandomState MT19937; choice(population, replace=true) via rk_interval mask/rejection"
                .to_owned(),
        n_samples: N_SAMPLES,
        fleet_sizes: FLEET_SIZES.to_vec(),
        charging_strategies: STRATEGIES.iter().map(|value| (*value).to_owned()).collect(),
        profile_kinds: PROFILE_KINDS.iter().map(|value| (*value).to_owned()).collect(),
        vehicle_energy_kwh_per_mile: KWH_PER_MILE,
        constant_power_charger_kw: CHARGER_POWER_KW,
        daily_source_seconds: DAY_SECONDS,
        output_intervals: 96,
        output_interval_minutes: 15,
        generated_profile_count: profiles.len(),
        generated_total_interval_count: profiles.len() * 96,
        archive_comparison_state: "not_performed_official_catalog_zip_not_supplied".to_owned(),
        candidate_evidence_class: "source_reconstructed_model_output".to_owned(),
        profiles,
        claim_ceiling: "Source-reconstructed candidate profiles from pinned NLR schedule inputs and the documented model algorithm. This is not an official archive audit or an independent NLR replication until the generated outputs are compared with the exact catalog ZIP under matched source/runtime semantics. The source data are modeled operating scenarios, not metered depot loads.".to_owned(),
    };
    let report_bytes =
        serde_json::to_vec(&report).map_err(|error| format!("replay report serialization: {error}"))?;
    let envelope = ReplayEnvelope {
        report,
        report_sha256: sha256(&report_bytes),
    };
    let output = serde_json::to_vec_pretty(&envelope)
        .map_err(|error| format!("replay envelope serialization: {error}"))?;
    fs::write(report_path, output)
        .map_err(|error| format!("cannot write replay report {}: {error}", report_path.display()))?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_random_seed_zero_matches_cpython_randint_known_answer() {
        assert_eq!(
            python_master_seeds(0, 10).expect("Python seed vector"),
            vec![864, 394, 776, 911, 430, 41, 265, 988, 523, 497]
        );
    }

    #[test]
    fn numpy_randomstate_seed_zero_matches_legacy_randint_known_answer() {
        assert_eq!(
            numpy_choice_indices(0, 5, 10).expect("NumPy choice"),
            vec![4, 0, 3, 3, 3, 1, 3, 2, 4, 0]
        );
    }

    #[test]
    fn interval_difference_accumulation_preserves_overlapping_power() {
        let mut delta = vec![0.0; 11];
        add_interval(&mut delta, 1, 5, 10.0);
        add_interval(&mut delta, 3, 8, 7.0);
        let load = cumulative_profile(&delta);
        assert_eq!(load, vec![0.0, 10.0, 10.0, 17.0, 17.0, 7.0, 7.0, 7.0, 0.0, 0.0]);
    }

    #[test]
    fn immediate_delayed_and_minimum_power_obey_pinned_day_contract() {
        let day = OperationDay {
            id: "fixture".to_owned(),
            vmt: 10.0,
            time_on_shift_s: 3600,
            time_off_shift_s: DAY_SECONDS as u64 - 3600,
        };
        let schedule = vec![
            ScheduleSegment {
                total_time_s: 1000,
                on_shift: false,
                vmt: 0.0,
            },
            ScheduleSegment {
                total_time_s: 3600,
                on_shift: true,
                vmt: 10.0,
            },
            ScheduleSegment {
                total_time_s: DAY_SECONDS as u64 - 4600,
                on_shift: false,
                vmt: 0.0,
            },
        ];
        let mut immediate_delta = vec![0.0; DAY_SECONDS + 1];
        add_vehicle_immediate_or_delayed(&mut immediate_delta, &day, &schedule, false)
            .expect("immediate strategy");
        let immediate = cumulative_profile(&immediate_delta);
        assert_eq!(immediate[4600], 100.0);
        assert_eq!(immediate[5248], 0.0);

        let mut delayed_delta = vec![0.0; DAY_SECONDS + 1];
        add_vehicle_immediate_or_delayed(&mut delayed_delta, &day, &schedule, true)
            .expect("delayed strategy");
        let delayed = cumulative_profile(&delayed_delta);
        assert_eq!(delayed[352], 100.0);
        assert_eq!(delayed[1000], 0.0);

        let mut minimum_delta = vec![0.0; DAY_SECONDS + 1];
        add_vehicle_min_power(&mut minimum_delta, &day, &schedule)
            .expect("minimum-power strategy");
        let minimum = cumulative_profile(&minimum_delta);
        let expected_minimum =
            day.vmt * KWH_PER_MILE / (day.time_off_shift_s as f64 / 3600.0);
        assert!((minimum[0] - expected_minimum).abs() < 1e-12);
        assert_eq!(minimum[1000], 0.0);
        assert_eq!(minimum[4600], expected_minimum);
    }

    #[test]
    fn quarter_hour_aggregation_preserves_ninety_six_bins_and_first_sample_padding() {
        let profile = vec![1.0; DAY_SECONDS];
        let output = quarter_hour_means(&profile).expect("quarter-hour profile");
        assert_eq!(output.len(), 96);
        assert!(output.iter().all(|value| (*value - 1.0).abs() < 1e-12));
    }
}
