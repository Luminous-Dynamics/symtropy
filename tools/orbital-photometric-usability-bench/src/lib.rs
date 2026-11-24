use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, f64::consts::PI};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-photometric-usability.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-photometric-usability.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-PHOTO-USABILITY-001";
pub const CLAIM_CEILING: &str = "synthetic_declared_brdf_signal_and_sensor_dynamic_range_only";
pub const CASE_COUNT: usize = 8;
const SCALE: f64 = 1_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeometryDisposition {
    Pass,
    Fail,
    Indeterminate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignalStatus {
    Usable,
    Underexposed,
    Saturated,
    Indeterminate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SummaryStatus {
    FullyUsable,
    Partial,
    NotUsable,
    Indeterminate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AcquisitionStatus {
    Usable,
    Partial,
    Unusable,
    Indeterminate,
    GeometryRejected,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "model",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum Brdf {
    Lambertian {
        albedo: f64,
    },
    BlinnPhong {
        albedo: f64,
        specular: f64,
        shininess: f64,
    },
    CookTorranceGgx {
        albedo: f64,
        f0: f64,
        roughness: f64,
        metallic: f64,
    },
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    pub direction: [f64; 3],
    pub incident_radiance: f64,
    pub illumination_factor: f64,
    pub asserted: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    pub id: String,
    pub exposure_gain: f64,
    pub noise_floor: f64,
    pub saturation: f64,
    pub asserted: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub id: String,
    pub geometry: GeometryDisposition,
    pub normal: [f64; 3],
    pub view_direction: [f64; 3],
    pub material_asserted: bool,
    pub brdf: Brdf,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub mode_id: String,
    pub summary: SummaryStatus,
    pub acquisition: AcquisitionStatus,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub frame_id: String,
    pub light: Light,
    pub modes: Vec<Mode>,
    pub samples: Vec<Sample>,
    pub expected: Vec<Expected>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SampleResult {
    pub sample_id: String,
    pub geometry: GeometryDisposition,
    pub signal_micro_units: Option<u64>,
    pub status: SignalStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeResult {
    pub mode_id: String,
    pub sample_count: usize,
    pub usable_count: usize,
    pub underexposed_count: usize,
    pub saturated_count: usize,
    pub indeterminate_count: usize,
    pub geometry_pass_count: usize,
    pub geometry_fail_count: usize,
    pub geometry_indeterminate_count: usize,
    pub summary: SummaryStatus,
    pub acquisition: AcquisitionStatus,
    pub samples: Vec<SampleResult>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub expected: Vec<Expected>,
    pub expected_matches: usize,
    pub expected_mismatches: usize,
    pub modes: Vec<ModeResult>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub fixture_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_matches: usize,
    pub expected_mismatches: usize,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> bool {
    a.iter().all(|x| x.is_finite()) && (norm(a) - 1.0).abs() <= 1.0e-6
}
fn normalize(a: [f64; 3]) -> [f64; 3] {
    let n = norm(a);
    [a[0] / n, a[1] / n, a[2] / n]
}

fn valid_brdf(b: &Brdf) -> bool {
    match b {
        Brdf::Lambertian { albedo } => albedo.is_finite() && (0.0..=1.0).contains(albedo),
        Brdf::BlinnPhong {
            albedo,
            specular,
            shininess,
        } => {
            albedo.is_finite()
                && (0.0..=1.0).contains(albedo)
                && specular.is_finite()
                && (0.0..=1.0).contains(specular)
                && shininess.is_finite()
                && (1.0..=2048.0).contains(shininess)
        }
        Brdf::CookTorranceGgx {
            albedo,
            f0,
            roughness,
            metallic,
        } => {
            albedo.is_finite()
                && (0.0..=1.0).contains(albedo)
                && f0.is_finite()
                && (0.0..=1.0).contains(f0)
                && roughness.is_finite()
                && (0.05..=1.0).contains(roughness)
                && metallic.is_finite()
                && (0.0..=1.0).contains(metallic)
        }
    }
}
fn validate(c: &Case) -> Result<(), String> {
    if c.case_id.trim().is_empty()
        || c.frame_id.trim().is_empty()
        || c.modes.is_empty()
        || c.samples.is_empty()
    {
        return Err("CASE_BOUNDS_INVALID".into());
    }
    if !unit(c.light.direction)
        || !c.light.incident_radiance.is_finite()
        || !(0.0..=100.0).contains(&c.light.incident_radiance)
        || !c.light.illumination_factor.is_finite()
        || !(0.0..=1.0).contains(&c.light.illumination_factor)
    {
        return Err("LIGHT_INPUT_INVALID".into());
    }
    let mut sample_ids = BTreeSet::new();
    for s in &c.samples {
        if s.id.trim().is_empty()
            || !sample_ids.insert(s.id.as_str())
            || !unit(s.normal)
            || !unit(s.view_direction)
            || !valid_brdf(&s.brdf)
        {
            return Err("SURFACE_SAMPLE_INVALID".into());
        }
    }
    let mut mode_ids = BTreeSet::new();
    for m in &c.modes {
        if m.id.trim().is_empty()
            || !mode_ids.insert(m.id.as_str())
            || !m.exposure_gain.is_finite()
            || !(0.0..=100.0).contains(&m.exposure_gain)
            || !m.noise_floor.is_finite()
            || !(0.0..=1.0e6).contains(&m.noise_floor)
            || !m.saturation.is_finite()
            || !(0.0..=1.0e6).contains(&m.saturation)
            || m.noise_floor >= m.saturation
        {
            return Err("SENSOR_MODE_INVALID".into());
        }
    }
    let expected: BTreeSet<&str> = c.expected.iter().map(|x| x.mode_id.as_str()).collect();
    if expected.len() != c.expected.len() || expected != mode_ids {
        return Err("EXPECTED_MODE_SET_INVALID".into());
    }
    Ok(())
}
fn fresnel(f0: f64, c: f64) -> f64 {
    f0 + (1.0 - f0) * (1.0 - c.clamp(0.0, 1.0)).powi(5)
}
fn smith(n: f64, a2: f64) -> f64 {
    if n <= 0.0 {
        0.0
    } else {
        2.0 * n / (n + (a2 + (1.0 - a2) * n * n).sqrt())
    }
}
fn response(b: &Brdf, n: [f64; 3], l: [f64; 3], v: [f64; 3]) -> f64 {
    let nl = dot(n, l).clamp(0.0, 1.0);
    let nv = dot(n, v).clamp(0.0, 1.0);
    if nl <= 0.0 || nv <= 0.0 {
        return 0.0;
    }
    let h = normalize([l[0] + v[0], l[1] + v[1], l[2] + v[2]]);
    let nh = dot(n, h).clamp(0.0, 1.0);
    let vh = dot(v, h).clamp(0.0, 1.0);
    match b {
        Brdf::Lambertian { albedo } => *albedo / PI,
        Brdf::BlinnPhong {
            albedo,
            specular,
            shininess,
        } => *albedo / PI + *specular * (*shininess + 2.0) / (2.0 * PI) * nh.powf(*shininess),
        Brdf::CookTorranceGgx {
            albedo,
            f0,
            roughness,
            metallic,
        } => {
            let alpha = roughness * roughness;
            let a2 = alpha * alpha;
            let den = nh * nh * (a2 - 1.0) + 1.0;
            let d = a2 / (PI * den * den);
            let g = smith(nl, a2) * smith(nv, a2);
            let f = fresnel(*f0, vh);
            d * g * f / (4.0 * nl * nv).max(1.0e-12)
                + (1.0 - *metallic) * (1.0 - f) * (*albedo) / PI
        }
    }
}
fn quantize(x: f64) -> Result<u64, String> {
    if !x.is_finite() || x < 0.0 {
        return Err("SIGNAL_INVALID".into());
    }
    let y = x * SCALE;
    if !y.is_finite() || y > u64::MAX as f64 {
        return Err("SIGNAL_OVERFLOW".into());
    }
    Ok(y.round() as u64)
}
fn eval_sample(c: &Case, s: &Sample, m: &Mode) -> Result<SampleResult, String> {
    if !c.light.asserted || !s.material_asserted || !m.asserted {
        return Ok(SampleResult {
            sample_id: s.id.clone(),
            geometry: s.geometry,
            signal_micro_units: None,
            status: SignalStatus::Indeterminate,
        });
    }
    let n = normalize(s.normal);
    let l = normalize(c.light.direction);
    let v = normalize(s.view_direction);
    let raw = response(&s.brdf, n, l, v)
        * c.light.incident_radiance
        * c.light.illumination_factor
        * dot(n, l).clamp(0.0, 1.0)
        * m.exposure_gain;
    let q = quantize(raw)?;
    let lo = quantize(m.noise_floor)?;
    let hi = quantize(m.saturation)?;
    let status = if q < lo {
        SignalStatus::Underexposed
    } else if q > hi {
        SignalStatus::Saturated
    } else {
        SignalStatus::Usable
    };
    Ok(SampleResult {
        sample_id: s.id.clone(),
        geometry: s.geometry,
        signal_micro_units: Some(q),
        status,
    })
}
fn eval_mode(c: &Case, m: &Mode) -> Result<ModeResult, String> {
    let samples = c
        .samples
        .iter()
        .map(|s| eval_sample(c, s, m))
        .collect::<Result<Vec<_>, _>>()?;
    let count = samples.len();
    let usable = samples
        .iter()
        .filter(|s| s.status == SignalStatus::Usable)
        .count();
    let under = samples
        .iter()
        .filter(|s| s.status == SignalStatus::Underexposed)
        .count();
    let sat = samples
        .iter()
        .filter(|s| s.status == SignalStatus::Saturated)
        .count();
    let ind = samples
        .iter()
        .filter(|s| s.status == SignalStatus::Indeterminate)
        .count();
    let gp = samples
        .iter()
        .filter(|s| s.geometry == GeometryDisposition::Pass)
        .count();
    let gf = samples
        .iter()
        .filter(|s| s.geometry == GeometryDisposition::Fail)
        .count();
    let gi = samples
        .iter()
        .filter(|s| s.geometry == GeometryDisposition::Indeterminate)
        .count();
    let summary = if ind > 0 {
        SummaryStatus::Indeterminate
    } else if usable == count {
        SummaryStatus::FullyUsable
    } else if usable > 0 {
        SummaryStatus::Partial
    } else {
        SummaryStatus::NotUsable
    };
    let acquisition = if gf > 0 {
        AcquisitionStatus::GeometryRejected
    } else if gi > 0 || summary == SummaryStatus::Indeterminate {
        AcquisitionStatus::Indeterminate
    } else {
        match summary {
            SummaryStatus::FullyUsable => AcquisitionStatus::Usable,
            SummaryStatus::Partial => AcquisitionStatus::Partial,
            SummaryStatus::NotUsable => AcquisitionStatus::Unusable,
            SummaryStatus::Indeterminate => AcquisitionStatus::Indeterminate,
        }
    };
    Ok(ModeResult {
        mode_id: m.id.clone(),
        sample_count: count,
        usable_count: usable,
        underexposed_count: under,
        saturated_count: sat,
        indeterminate_count: ind,
        geometry_pass_count: gp,
        geometry_fail_count: gf,
        geometry_indeterminate_count: gi,
        summary,
        acquisition,
        samples,
    })
}
pub fn evaluate_case(c: &Case) -> Result<CaseResult, String> {
    validate(c)?;
    let modes = c
        .modes
        .iter()
        .map(|m| eval_mode(c, m))
        .collect::<Result<Vec<_>, _>>()?;
    let mut matches = 0;
    for e in &c.expected {
        let r = modes
            .iter()
            .find(|r| r.mode_id == e.mode_id)
            .ok_or("EXPECTED_MODE_MISSING")?;
        if r.summary == e.summary && r.acquisition == e.acquisition {
            matches += 1
        }
    }
    Ok(CaseResult {
        case_id: c.case_id.clone(),
        expected: c.expected.clone(),
        expected_matches: matches,
        expected_mismatches: c.expected.len() - matches,
        modes,
    })
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn audit(bytes: &[u8], sha: &str) -> Result<Receipt, String> {
    if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_INVALID".into());
    }
    let corpus: Corpus = serde_json::from_slice(bytes).map_err(|e| format!("CORPUS_PARSE:{e}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.len() != CASE_COUNT
    {
        return Err("CORPUS_IDENTITY_INVALID".into());
    }
    let mut ids = BTreeSet::new();
    let mut cases = Vec::new();
    for c in &corpus.cases {
        if !ids.insert(c.case_id.as_str()) {
            return Err("DUPLICATE_CASE_ID".into());
        }
        cases.push(evaluate_case(c)?)
    }
    let matches = cases.iter().map(|c| c.expected_matches).sum::<usize>();
    let total = cases.iter().map(|c| c.expected.len()).sum::<usize>();
    Ok(Receipt {
        schema: RECEIPT_SCHEMA.into(),
        benchmark_id: BENCHMARK_ID.into(),
        source_commit: sha.to_ascii_lowercase(),
        fixture_sha256: digest(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_matches: matches,
        expected_mismatches: total - matches,
        authority_granted: false,
        structural_acceptance_authorized: false,
        physical_validation: "NOT_ASSESSED".into(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(brdf: Brdf) -> Sample {
        Sample {
            id: "s".into(),
            geometry: GeometryDisposition::Pass,
            normal: [0., 0., 1.],
            view_direction: [0., 0., 1.],
            material_asserted: true,
            brdf,
        }
    }
    fn case(brdf: Brdf) -> Case {
        Case {
            case_id: "t".into(),
            frame_id: "f".into(),
            light: Light {
                direction: [0., 0., 1.],
                incident_radiance: 1.,
                illumination_factor: 1.,
                asserted: true,
            },
            modes: vec![Mode {
                id: "m".into(),
                exposure_gain: 1.,
                noise_floor: 0.01,
                saturation: 1.,
                asserted: true,
            }],
            samples: vec![sample(brdf)],
            expected: vec![Expected {
                mode_id: "m".into(),
                summary: SummaryStatus::FullyUsable,
                acquisition: AcquisitionStatus::Usable,
            }],
        }
    }
    #[test]
    fn lambertian_known_answer() {
        let r = evaluate_case(&case(Brdf::Lambertian { albedo: 0.8 })).unwrap();
        assert_eq!(r.modes[0].samples[0].signal_micro_units, Some(254648));
    }
    #[test]
    fn blinn_can_saturate() {
        let c = case(Brdf::BlinnPhong {
            albedo: 0.1,
            specular: 0.5,
            shininess: 16.,
        });
        assert_eq!(
            evaluate_case(&c).unwrap().modes[0].samples[0].status,
            SignalStatus::Saturated
        );
    }
    #[test]
    fn missing_material_abstains() {
        let mut c = case(Brdf::Lambertian { albedo: 0.8 });
        c.samples[0].material_asserted = false;
        assert_eq!(
            evaluate_case(&c).unwrap().modes[0].acquisition,
            AcquisitionStatus::Indeterminate
        );
    }
    #[test]
    fn geometry_failure_not_hidden() {
        let mut c = case(Brdf::Lambertian { albedo: 0.8 });
        c.samples[0].geometry = GeometryDisposition::Fail;
        assert_eq!(
            evaluate_case(&c).unwrap().modes[0].summary,
            SummaryStatus::FullyUsable
        );
        assert_eq!(
            evaluate_case(&c).unwrap().modes[0].acquisition,
            AcquisitionStatus::GeometryRejected
        );
    }
    #[test]
    fn rotation_invariant() {
        let mut c = case(Brdf::CookTorranceGgx {
            albedo: 0.5,
            f0: 0.04,
            roughness: 0.6,
            metallic: 0.,
        });
        let a = evaluate_case(&c).unwrap().modes[0].samples[0].signal_micro_units;
        c.samples[0].normal = [1., 0., 0.];
        c.samples[0].view_direction = [1., 0., 0.];
        c.light.direction = [1., 0., 0.];
        let b = evaluate_case(&c).unwrap().modes[0].samples[0].signal_micro_units;
        assert_eq!(a, b);
    }
    #[test]
    fn nonunit_rejected() {
        let mut c = case(Brdf::Lambertian { albedo: 0.8 });
        c.samples[0].normal = [0., 0., 2.];
        assert_eq!(evaluate_case(&c), Err("SURFACE_SAMPLE_INVALID".into()));
    }
}
