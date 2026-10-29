//! Deterministic, offline-only SIM-REL-001 reference-parity smoke harness.
//!
//! This harness compares two implementations of the same declared linear
//! spring-mass equations. It is a simulator/fixture integrity check only; it
//! does not implement a relational-grammar learner or establish scientific
//! superiority. The full 30+ scenario qualification corpus remains future work.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::process::Command;

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[derive(Clone, Debug)]
struct Spring {
    a: usize,
    b: Option<usize>,
    stiffness: f64,
}

#[derive(Clone, Debug)]
struct ForcePulse {
    node: usize,
    start_step: usize,
    end_step: usize,
    amplitude: f64,
}

#[derive(Clone, Debug)]
struct Scenario {
    id: String,
    split: String,
    difficulty: String,
    masses: Vec<f64>,
    x0: Vec<f64>,
    v0: Vec<f64>,
    damping: Vec<f64>,
    dt: f64,
    steps: usize,
    springs: Vec<Spring>,
    forces: Vec<ForcePulse>,
    observation_stride: usize,
}

#[derive(Clone, Copy)]
enum Solver {
    Edgewise,
    StiffnessMatrix,
}

struct CaseReceipt {
    id: String,
    split: String,
    steps: usize,
    observation_stride: usize,
    observations: usize,
    max_abs_delta: f64,
    rmse: f64,
    status: &'static str,
}

fn parse_float_list(raw: &str, field: &str) -> Result<Vec<f64>, String> {
    let values: Result<Vec<_>, _> = raw
        .split(',')
        .map(|v| {
            v.parse::<f64>()
                .map_err(|_| format!("invalid {field} value: {v}"))
        })
        .collect();
    let values = values?;
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err(format!("{field} must contain finite values"));
    }
    Ok(values)
}

fn parse_springs(raw: &str) -> Result<Vec<Spring>, String> {
    if raw == "none" {
        return Ok(Vec::new());
    }
    raw.split(';')
        .map(|part| {
            let fields: Vec<_> = part.split(',').collect();
            if fields.len() != 3 {
                return Err(format!("spring must have a,b,k fields: {part}"));
            }
            let a_i = fields[0]
                .parse::<i64>()
                .map_err(|_| format!("invalid spring node a: {}", fields[0]))?;
            let b_i = fields[1]
                .parse::<i64>()
                .map_err(|_| format!("invalid spring node b: {}", fields[1]))?;
            let stiffness = fields[2]
                .parse::<f64>()
                .map_err(|_| format!("invalid stiffness: {}", fields[2]))?;
            if a_i < 0 || b_i < -1 || !stiffness.is_finite() || stiffness <= 0.0 {
                return Err(format!("spring endpoints or stiffness out of range: {part}"));
            }
            Ok(Spring {
                a: a_i as usize,
                b: (b_i >= 0).then_some(b_i as usize),
                stiffness,
            })
        })
        .collect()
}

fn parse_forces(raw: &str) -> Result<Vec<ForcePulse>, String> {
    if raw == "none" {
        return Ok(Vec::new());
    }
    raw.split(';')
        .map(|part| {
            let fields: Vec<_> = part.split(',').collect();
            if fields.len() != 4 {
                return Err(format!("force pulse must have node,start,end,amplitude: {part}"));
            }
            let node = fields[0]
                .parse::<usize>()
                .map_err(|_| format!("invalid force node: {}", fields[0]))?;
            let start_step = fields[1]
                .parse::<usize>()
                .map_err(|_| format!("invalid force start: {}", fields[1]))?;
            let end_step = fields[2]
                .parse::<usize>()
                .map_err(|_| format!("invalid force end: {}", fields[2]))?;
            let amplitude = fields[3]
                .parse::<f64>()
                .map_err(|_| format!("invalid force amplitude: {}", fields[3]))?;
            if !amplitude.is_finite() || end_step <= start_step {
                return Err(format!("invalid force pulse interval/amplitude: {part}"));
            }
            Ok(ForcePulse {
                node,
                start_step,
                end_step,
                amplitude,
            })
        })
        .collect()
}

impl Scenario {
    fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!("invalid scenario id: {}", self.id));
        }
        if !["train", "validation", "heldout"].contains(&self.split.as_str()) {
            return Err(format!("unknown split '{}' in {}", self.split, self.id));
        }
        if !["easy", "medium", "hard"].contains(&self.difficulty.as_str()) {
            return Err(format!("unknown difficulty '{}' in {}", self.difficulty, self.id));
        }
        let n = self.masses.len();
        if n == 0 || self.x0.len() != n || self.v0.len() != n || self.damping.len() != n {
            return Err(format!("state/mass/damping vector length mismatch in {}", self.id));
        }
        if self.masses.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(format!("masses must be finite and positive in {}", self.id));
        }
        if self.damping.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(format!("damping must be finite and non-negative in {}", self.id));
        }
        if self.x0.iter().chain(self.v0.iter()).any(|v| !v.is_finite()) {
            return Err(format!("initial state contains non-finite value in {}", self.id));
        }
        if !self.dt.is_finite() || self.dt <= 0.0 || self.steps == 0 || self.steps > 100_000 {
            return Err(format!("invalid dt/steps in {}", self.id));
        }
        if self.observation_stride == 0 {
            return Err(format!("observation_stride must be positive in {}", self.id));
        }
        for spring in &self.springs {
            if spring.a >= n || spring.b.is_some_and(|b| b >= n || b == spring.a) {
                return Err(format!("invalid spring endpoint in {}", self.id));
            }
        }
        for force in &self.forces {
            if force.node >= n || force.end_step > self.steps {
                return Err(format!("invalid force pulse bounds in {}", self.id));
            }
        }
        if self.springs.is_empty() {
            return Err(format!("scenario {} has no springs", self.id));
        }
        Ok(())
    }
}

fn parse_corpus(raw: &str) -> Result<Vec<Scenario>, String> {
    let mut scenarios = Vec::new();
    let mut ids = BTreeSet::new();
    for (line_idx, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        if fields.len() != 12 {
            return Err(format!("line {}: expected 12 fields, got {}", line_idx + 1, fields.len()));
        }
        let scenario = Scenario {
            id: fields[0].to_owned(),
            split: fields[1].to_owned(),
            difficulty: fields[2].to_owned(),
            masses: parse_float_list(fields[3], "masses")?,
            x0: parse_float_list(fields[4], "x0")?,
            v0: parse_float_list(fields[5], "v0")?,
            damping: parse_float_list(fields[6], "damping")?,
            dt: fields[7]
                .parse::<f64>()
                .map_err(|_| format!("line {}: invalid dt", line_idx + 1))?,
            steps: fields[8]
                .parse::<usize>()
                .map_err(|_| format!("line {}: invalid steps", line_idx + 1))?,
            springs: parse_springs(fields[9])?,
            forces: parse_forces(fields[10])?,
            observation_stride: fields[11]
                .parse::<usize>()
                .map_err(|_| format!("line {}: invalid observation_stride", line_idx + 1))?,
        };
        scenario.validate().map_err(|e| format!("line {}: {e}", line_idx + 1))?;
        if !ids.insert(scenario.id.clone()) {
            return Err(format!("duplicate scenario id: {}", scenario.id));
        }
        scenarios.push(scenario);
    }
    if scenarios.is_empty() {
        return Err("corpus contains no scenarios".to_owned());
    }
    if !scenarios.iter().any(|s| s.split == "train")
        || !scenarios.iter().any(|s| s.split == "validation")
        || !scenarios.iter().any(|s| s.split == "heldout")
    {
        return Err("corpus must include train, validation, and heldout splits".to_owned());
    }
    Ok(scenarios)
}

fn stiffness_matrix(scenario: &Scenario) -> Vec<Vec<f64>> {
    let n = scenario.masses.len();
    let mut k = vec![vec![0.0; n]; n];
    for spring in &scenario.springs {
        k[spring.a][spring.a] += spring.stiffness;
        if let Some(b) = spring.b {
            k[b][b] += spring.stiffness;
            k[spring.a][b] -= spring.stiffness;
            k[b][spring.a] -= spring.stiffness;
        }
    }
    k
}

fn simulate(scenario: &Scenario, solver: Solver) -> Vec<f64> {
    let n = scenario.masses.len();
    let k_matrix = match solver {
        Solver::Edgewise => None,
        Solver::StiffnessMatrix => Some(stiffness_matrix(scenario)),
    };
    let mut x = scenario.x0.clone();
    let mut v = scenario.v0.clone();
    let mut trace = Vec::with_capacity((scenario.steps + 1) * n * 2);
    trace.extend_from_slice(&x);
    trace.extend_from_slice(&v);

    for step in 0..scenario.steps {
        let mut forces = vec![0.0; n];
        for pulse in &scenario.forces {
            if step >= pulse.start_step && step < pulse.end_step {
                forces[pulse.node] += pulse.amplitude;
            }
        }
        match &k_matrix {
            None => {
                for spring in &scenario.springs {
                    let xb = spring.b.map_or(0.0, |b| x[b]);
                    let force_a = -spring.stiffness * (x[spring.a] - xb);
                    forces[spring.a] += force_a;
                    if let Some(b) = spring.b {
                        forces[b] -= force_a;
                    }
                }
            }
            Some(matrix) => {
                for i in 0..n {
                    for j in 0..n {
                        forces[i] -= matrix[i][j] * x[j];
                    }
                }
            }
        }
        for i in 0..n {
            let acceleration = (forces[i] - scenario.damping[i] * v[i]) / scenario.masses[i];
            v[i] += acceleration * scenario.dt;
            x[i] += v[i] * scenario.dt;
        }
        trace.extend_from_slice(&x);
        trace.extend_from_slice(&v);
    }
    trace
}

fn compare_solvers(scenario: &Scenario) -> CaseReceipt {
    let edgewise = simulate(scenario, Solver::Edgewise);
    let matrix = simulate(scenario, Solver::StiffnessMatrix);
    let width = scenario.masses.len() * 2;
    let mut max_abs_delta: f64 = 0.0;
    for (a, b) in edgewise.iter().zip(matrix.iter()) {
        max_abs_delta = max_abs_delta.max((a - b).abs());
    }
    let mut squared_sum = 0.0;
    let mut sampled_values = 0usize;
    for state_index in 0..=scenario.steps {
        let sampled = state_index == 0
            || state_index % scenario.observation_stride == 0
            || state_index == scenario.steps;
        if !sampled {
            continue;
        }
        let start = state_index * width;
        for index in start..(start + width) {
            let delta = edgewise[index] - matrix[index];
            squared_sum += delta * delta;
            sampled_values += 1;
        }
    }
    let rmse = (squared_sum / sampled_values as f64).sqrt();
    let observations = scenario.steps / scenario.observation_stride
        + 1
        + usize::from(scenario.steps % scenario.observation_stride != 0);
    let status = if max_abs_delta <= 1.0e-10 { "PASS" } else { "FAIL" };
    CaseReceipt {
        id: scenario.id.clone(),
        split: scenario.split.clone(),
        steps: scenario.steps,
        observation_stride: scenario.observation_stride,
        observations,
        max_abs_delta,
        rmse,
        status,
    }
}

fn sha256_hex(input: &[u8]) -> String {
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = [
        0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, four) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([four[0], four[1], four[2], four[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        use std::fmt::Write as _;
        let _ = write!(out, "{word:08x}");
    }
    out
}

fn current_git_head() -> Result<String, String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("could not execute git rev-parse HEAD: {e}"))?;
    if !out.status.success() {
        return Err("git rev-parse HEAD failed; run from a Git checkout".to_owned());
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|e| format!("git returned a non-UTF-8 HEAD: {e}"))
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_report(head: &str, corpus_sha: &str, cases: &[CaseReceipt]) -> String {
    let all_pass = cases.iter().all(|c| c.status == "PASS");
    let mut out = format!(
        "{{\n  \"schema_version\": 1,\n  \"synthetic_only\": true,\n  \"exact_head\": \"{}\",\n  \"corpus_sha256\": \"{}\",\n  \"case_count\": {},\n  \"all_reference_parity_cases_pass\": {},\n  \"claim_ceiling\": \"reference parity only; no discovery or superiority claim\",\n  \"cases\": [\n",
        json_escape(head), corpus_sha, cases.len(), all_pass
    );
    for (i, case) in cases.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"id\":\"{}\",\"split\":\"{}\",\"steps\":{},\"observation_stride\":{},\"recorded_states\":{},\"max_abs_delta\":{:.17e},\"rmse\":{:.17e},\"status\":\"{}\"}}{}\n",
            json_escape(&case.id),
            json_escape(&case.split),
            case.steps,
            case.observation_stride,
            case.observations,
            case.max_abs_delta,
            case.rmse,
            case.status,
            if i + 1 == cases.len() { "" } else { "," }
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut corpus = None;
    let mut expected_head = None;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => corpus = args.next(),
            "--expected-head" => expected_head = args.next(),
            "--out" => output = args.next(),
            "--help" | "-h" => {
                println!("sim-rel-reference --corpus PATH --expected-head SHA --out PATH");
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let corpus = corpus.ok_or("--corpus is required")?;
    let expected_head = expected_head.ok_or("--expected-head is required")?;
    let output = output.ok_or("--out is required")?;
    let head = current_git_head().map_err(|e| format!("identity gate: {e}"))?;
    if head != expected_head {
        return Err(format!("exact-head mismatch: expected {expected_head}, checkout is {head}").into());
    }

    let bytes = fs::read(&corpus)?;
    let corpus_sha = sha256_hex(&bytes);
    let raw = std::str::from_utf8(&bytes)?;
    let scenarios = parse_corpus(raw).map_err(|e| format!("corpus contract: {e}"))?;
    let cases: Vec<_> = scenarios.iter().map(compare_solvers).collect();
    let report = render_report(&head, &corpus_sha, &cases);
    if cases.iter().any(|c| c.status != "PASS") {
        if output == "-" {
            print!("{report}");
        } else {
            fs::write(&output, &report)?;
        }
        return Err("one or more reference-parity cases failed".into());
    }
    if output == "-" {
        print!("{report}");
    } else {
        fs::write(&output, &report)?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../fixtures/spring-mass-v1.tsv");

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn smoke_corpus_parses_with_all_splits() {
        let scenarios = parse_corpus(CORPUS).expect("corpus must parse");
        assert_eq!(scenarios.len(), 5);
        assert!(scenarios.iter().any(|s| s.split == "train"));
        assert!(scenarios.iter().any(|s| s.split == "validation"));
        assert!(scenarios.iter().any(|s| s.split == "heldout"));
    }

    #[test]
    fn duplicates_are_rejected() {
        let line = CORPUS.lines().find(|s| s.starts_with("train_chain2|")).unwrap();
        let duplicate = format!("{line}\n{line}");
        let err = parse_corpus(&duplicate).unwrap_err();
        assert!(err.contains("duplicate scenario id"));
    }

    #[test]
    fn unknown_or_missing_fields_fail_closed() {
        let err = parse_corpus("scenario|train|easy|1.0|0.1|0.0|0.0|0.01|10|0,-1,1.0|none")
            .unwrap_err();
        assert!(err.contains("expected 12 fields"));
    }

    #[test]
    fn reference_implementations_agree_on_smoke_corpus() {
        for scenario in parse_corpus(CORPUS).expect("valid test corpus") {
            let receipt = compare_solvers(&scenario);
            assert_eq!(receipt.status, "PASS", "scenario {}", scenario.id);
            assert!(receipt.max_abs_delta <= 1.0e-10);
        }
    }

    #[test]
    fn spring_edge_order_does_not_change_solution() {
        let mut scenario = parse_corpus(CORPUS).unwrap().remove(4);
        let reference = simulate(&scenario, Solver::Edgewise);
        scenario.springs.reverse();
        let reordered = simulate(&scenario, Solver::Edgewise);
        assert_eq!(reference.len(), reordered.len());
        for (a, b) in reference.iter().zip(reordered.iter()) {
            assert!((a - b).abs() <= 1.0e-10);
        }
    }

    #[test]
    fn invalid_endpoint_is_rejected() {
        let mut scenario = parse_corpus(CORPUS).unwrap().remove(0);
        scenario.springs[0].a = scenario.masses.len();
        assert!(scenario.validate().unwrap_err().contains("spring endpoint"));
    }
}
