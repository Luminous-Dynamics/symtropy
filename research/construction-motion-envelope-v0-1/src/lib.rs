#![forbid(unsafe_code)]
//! Conservative trajectory-to-occupancy projection. Producer-supplied bounds are
//! checked for structure and arithmetic, not authenticated or proven physically true.

use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point3Mm {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AxisMm {
    pub x: u64,
    pub y: u64,
    pub z: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarginsMm {
    pub path_deviation: AxisMm,
    pub localization: AxisMm,
    pub tracking: AxisMm,
    pub model_uncertainty: AxisMm,
    pub numerical_enclosure: AxisMm,
    pub stopping_reachability: AxisMm,
    pub policy_clearance: AxisMm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRefs {
    pub path_deviation: String,
    pub localization: String,
    pub tracking: String,
    pub model_uncertainty: String,
    pub numerical_enclosure: String,
    pub stopping_reachability: String,
    pub policy_clearance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub project_id: String,
    pub work_package_id: String,
    pub task_id: String,
    pub trajectory_id: String,
    pub robot_id: String,
    pub platform_config_sha256: String,
    pub tool_config_sha256: String,
    pub payload_config_sha256: String,
    pub collision_model_sha256: String,
    /// External reference to the authoritative body-primitive manifest bound by the collision-model digest.
    pub primitive_set_ref: String,
    /// Exact expected link/tool/payload primitive IDs from that manifest.
    pub expected_primitive_ids: Vec<String>,
    pub calibration_ref: String,
    pub controller_profile_ref: String,
    pub frame_id: String,
    pub transform_sha256: String,
    pub design_revision_sha256: String,
    pub site_epoch: String,
    pub clock_domain_id: String,
    pub trajectory_profile_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub slice_index: u32,
    pub primitive_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub from_mm: Point3Mm,
    pub to_mm: Point3Mm,
    pub radius_mm: u64,
    pub margins: MarginsMm,
    pub evidence: EvidenceRefs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AabbMm {
    pub min: Point3Mm,
    pub max: Point3Mm,
}

impl AabbMm {
    pub fn contains_sphere(&self, p: Point3Mm, r_mm: u64) -> bool {
        let Ok(r) = i64::try_from(r_mm) else {
            return false;
        };
        let (Some(x0), Some(x1), Some(y0), Some(y1), Some(z0), Some(z1)) = (
            p.x.checked_sub(r),
            p.x.checked_add(r),
            p.y.checked_sub(r),
            p.y.checked_add(r),
            p.z.checked_sub(r),
            p.z.checked_add(r),
        ) else {
            return false;
        };
        x0 >= self.min.x
            && x1 <= self.max.x
            && y0 >= self.min.y
            && y1 <= self.max.y
            && z0 >= self.min.z
            && z1 <= self.max.z
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvelopeSlice {
    pub slice_index: u32,
    pub primitive_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub from_mm: Point3Mm,
    pub to_mm: Point3Mm,
    pub radius_mm: u64,
    pub physical_swept_outer: AabbMm,
    pub stopping_outer: AabbMm,
    pub coordination_outer: AabbMm,
    pub margins: MarginsMm,
    pub evidence: EvidenceRefs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub context: Context,
    pub slices: Vec<EnvelopeSlice>,
}

impl Report {
    /// Versioned deterministic serialization; it does not hash or authenticate bytes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"CTMENV01".to_vec();
        for s in [
            &self.context.project_id,
            &self.context.work_package_id,
            &self.context.task_id,
            &self.context.trajectory_id,
            &self.context.robot_id,
            &self.context.platform_config_sha256,
            &self.context.tool_config_sha256,
            &self.context.payload_config_sha256,
            &self.context.collision_model_sha256,
            &self.context.primitive_set_ref,
            &self.context.calibration_ref,
            &self.context.controller_profile_ref,
            &self.context.frame_id,
            &self.context.transform_sha256,
            &self.context.design_revision_sha256,
            &self.context.site_epoch,
            &self.context.clock_domain_id,
            &self.context.trajectory_profile_sha256,
        ] {
            put_str(&mut out, s);
        }
        let expected_ids: BTreeSet<&str> = self
            .context
            .expected_primitive_ids
            .iter()
            .map(String::as_str)
            .collect();
        put_u64(&mut out, expected_ids.len() as u64);
        for primitive_id in expected_ids {
            put_str(&mut out, primitive_id);
        }
        put_u64(&mut out, self.slices.len() as u64);
        for s in &self.slices {
            put_u64(&mut out, u64::from(s.slice_index));
            put_str(&mut out, &s.primitive_id);
            put_u64(&mut out, s.start_ms);
            put_u64(&mut out, s.end_ms);
            put_point(&mut out, s.from_mm);
            put_point(&mut out, s.to_mm);
            put_u64(&mut out, s.radius_mm);
            for b in [
                s.physical_swept_outer,
                s.stopping_outer,
                s.coordination_outer,
            ] {
                put_point(&mut out, b.min);
                put_point(&mut out, b.max);
            }
            for a in [
                s.margins.path_deviation,
                s.margins.localization,
                s.margins.tracking,
                s.margins.model_uncertainty,
                s.margins.numerical_enclosure,
                s.margins.stopping_reachability,
                s.margins.policy_clearance,
            ] {
                put_axis(&mut out, a);
            }
            for r in [
                &s.evidence.path_deviation,
                &s.evidence.localization,
                &s.evidence.tracking,
                &s.evidence.model_uncertainty,
                &s.evidence.numerical_enclosure,
                &s.evidence.stopping_reachability,
                &s.evidence.policy_clearance,
            ] {
                put_str(&mut out, r);
            }
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Missing(&'static str),
    BadDigest(&'static str),
    Empty,
    BadInterval(u32),
    SliceIndex { expected: u32, got: u32 },
    TimeDiscontinuity,
    IntervalMismatch(u32),
    DuplicatePrimitive(String),
    DuplicateExpectedPrimitive(String),
    PrimitiveSetChanged(u32),
    CenterDiscontinuity(String),
    RadiusChanged(String),
    Overflow,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ErrorKind {}

pub fn build(context: &Context, input: &[Segment]) -> Result<Report, ErrorKind> {
    validate_context(context)?;
    if input.is_empty() {
        return Err(ErrorKind::Empty);
    }
    let mut groups: BTreeMap<u32, Vec<&Segment>> = BTreeMap::new();
    for s in input {
        if s.primitive_id.trim().is_empty() {
            return Err(ErrorKind::Missing("primitive_id"));
        }
        if s.start_ms >= s.end_ms {
            return Err(ErrorKind::BadInterval(s.slice_index));
        }
        if s.radius_mm == 0 {
            return Err(ErrorKind::Missing("positive radius_mm"));
        }
        validate_evidence(&s.evidence)?;
        groups.entry(s.slice_index).or_default().push(s);
    }

    let mut out = Vec::with_capacity(input.len());
    let expected_primitives: BTreeSet<String> =
        context.expected_primitive_ids.iter().cloned().collect();
    let mut previous: BTreeMap<String, (Point3Mm, u64)> = BTreeMap::new();
    let mut previous_end = None;

    for (position, (index, group_ref)) in groups.iter().enumerate() {
        let expected = u32::try_from(position).map_err(|_| ErrorKind::Overflow)?;
        if *index != expected {
            return Err(ErrorKind::SliceIndex {
                expected,
                got: *index,
            });
        }
        let mut group = group_ref.clone();
        group.sort_by(|a, b| a.primitive_id.cmp(&b.primitive_id));
        let first = group.first().ok_or(ErrorKind::Empty)?;
        let (start, end) = (first.start_ms, first.end_ms);
        if group.iter().any(|s| s.start_ms != start || s.end_ms != end) {
            return Err(ErrorKind::IntervalMismatch(*index));
        }
        if previous_end.is_some_and(|v| v != start) {
            return Err(ErrorKind::TimeDiscontinuity);
        }

        let mut current = BTreeSet::new();
        for s in &group {
            if !current.insert(s.primitive_id.clone()) {
                return Err(ErrorKind::DuplicatePrimitive(s.primitive_id.clone()));
            }
        }
        if current != expected_primitives {
            return Err(ErrorKind::PrimitiveSetChanged(*index));
        }

        for s in group {
            if let Some((end_point, radius)) = previous.get(&s.primitive_id) {
                if *end_point != s.from_mm {
                    return Err(ErrorKind::CenterDiscontinuity(s.primitive_id.clone()));
                }
                if *radius != s.radius_mm {
                    return Err(ErrorKind::RadiusChanged(s.primitive_id.clone()));
                }
            } else if *index != 0 {
                return Err(ErrorKind::PrimitiveSetChanged(*index));
            }
            let m = s.margins;
            let x = sum([
                s.radius_mm,
                m.path_deviation.x,
                m.localization.x,
                m.tracking.x,
                m.model_uncertainty.x,
                m.numerical_enclosure.x,
            ])?;
            let y = sum([
                s.radius_mm,
                m.path_deviation.y,
                m.localization.y,
                m.tracking.y,
                m.model_uncertainty.y,
                m.numerical_enclosure.y,
            ])?;
            let z = sum([
                s.radius_mm,
                m.path_deviation.z,
                m.localization.z,
                m.tracking.z,
                m.model_uncertainty.z,
                m.numerical_enclosure.z,
            ])?;
            let physical = chord_box(s.from_mm, s.to_mm, AxisMm { x, y, z })?;
            let stopping = expand(physical, m.stopping_reachability)?;
            let coordination = expand(stopping, m.policy_clearance)?;
            out.push(EnvelopeSlice {
                slice_index: *index,
                primitive_id: s.primitive_id.clone(),
                start_ms: start,
                end_ms: end,
                from_mm: s.from_mm,
                to_mm: s.to_mm,
                radius_mm: s.radius_mm,
                physical_swept_outer: physical,
                stopping_outer: stopping,
                coordination_outer: coordination,
                margins: m,
                evidence: s.evidence.clone(),
            });
            previous.insert(s.primitive_id.clone(), (s.to_mm, s.radius_mm));
        }
        previous_end = Some(end);
    }
    Ok(Report {
        context: context.clone(),
        slices: out,
    })
}

fn validate_context(c: &Context) -> Result<(), ErrorKind> {
    for (n, v) in [
        ("project_id", &c.project_id),
        ("work_package_id", &c.work_package_id),
        ("task_id", &c.task_id),
        ("trajectory_id", &c.trajectory_id),
        ("robot_id", &c.robot_id),
        ("primitive_set_ref", &c.primitive_set_ref),
        ("calibration_ref", &c.calibration_ref),
        ("controller_profile_ref", &c.controller_profile_ref),
        ("frame_id", &c.frame_id),
        ("site_epoch", &c.site_epoch),
        ("clock_domain_id", &c.clock_domain_id),
    ] {
        required(n, v)?;
    }
    if c.expected_primitive_ids.is_empty() {
        return Err(ErrorKind::Missing("expected_primitive_ids"));
    }
    let mut expected_ids = BTreeSet::new();
    for id in &c.expected_primitive_ids {
        required("expected_primitive_id", id)?;
        if !expected_ids.insert(id) {
            return Err(ErrorKind::DuplicateExpectedPrimitive(id.clone()));
        }
    }
    for (n, v) in [
        ("platform_config_sha256", &c.platform_config_sha256),
        ("tool_config_sha256", &c.tool_config_sha256),
        ("payload_config_sha256", &c.payload_config_sha256),
        ("collision_model_sha256", &c.collision_model_sha256),
        ("transform_sha256", &c.transform_sha256),
        ("design_revision_sha256", &c.design_revision_sha256),
        ("trajectory_profile_sha256", &c.trajectory_profile_sha256),
    ] {
        if v.len() != 64
            || !v
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ErrorKind::BadDigest(n));
        }
    }
    Ok(())
}

fn validate_evidence(e: &EvidenceRefs) -> Result<(), ErrorKind> {
    for (n, v) in [
        ("path_deviation_ref", &e.path_deviation),
        ("localization_ref", &e.localization),
        ("tracking_ref", &e.tracking),
        ("model_uncertainty_ref", &e.model_uncertainty),
        ("numerical_enclosure_ref", &e.numerical_enclosure),
        ("stopping_reachability_ref", &e.stopping_reachability),
        ("policy_clearance_ref", &e.policy_clearance),
    ] {
        required(n, v)?;
    }
    Ok(())
}
fn required(n: &'static str, v: &str) -> Result<(), ErrorKind> {
    if v.trim().is_empty() {
        Err(ErrorKind::Missing(n))
    } else {
        Ok(())
    }
}
fn sum(v: [u64; 6]) -> Result<u64, ErrorKind> {
    v.into_iter()
        .try_fold(0_u64, |a, b| a.checked_add(b).ok_or(ErrorKind::Overflow))
}
fn chord_box(a: Point3Mm, b: Point3Mm, m: AxisMm) -> Result<AabbMm, ErrorKind> {
    let (x, y, z) = (
        i64::try_from(m.x).map_err(|_| ErrorKind::Overflow)?,
        i64::try_from(m.y).map_err(|_| ErrorKind::Overflow)?,
        i64::try_from(m.z).map_err(|_| ErrorKind::Overflow)?,
    );
    Ok(AabbMm {
        min: Point3Mm {
            x: a.x.min(b.x).checked_sub(x).ok_or(ErrorKind::Overflow)?,
            y: a.y.min(b.y).checked_sub(y).ok_or(ErrorKind::Overflow)?,
            z: a.z.min(b.z).checked_sub(z).ok_or(ErrorKind::Overflow)?,
        },
        max: Point3Mm {
            x: a.x.max(b.x).checked_add(x).ok_or(ErrorKind::Overflow)?,
            y: a.y.max(b.y).checked_add(y).ok_or(ErrorKind::Overflow)?,
            z: a.z.max(b.z).checked_add(z).ok_or(ErrorKind::Overflow)?,
        },
    })
}
fn expand(b: AabbMm, m: AxisMm) -> Result<AabbMm, ErrorKind> {
    let (x, y, z) = (
        i64::try_from(m.x).map_err(|_| ErrorKind::Overflow)?,
        i64::try_from(m.y).map_err(|_| ErrorKind::Overflow)?,
        i64::try_from(m.z).map_err(|_| ErrorKind::Overflow)?,
    );
    Ok(AabbMm {
        min: Point3Mm {
            x: b.min.x.checked_sub(x).ok_or(ErrorKind::Overflow)?,
            y: b.min.y.checked_sub(y).ok_or(ErrorKind::Overflow)?,
            z: b.min.z.checked_sub(z).ok_or(ErrorKind::Overflow)?,
        },
        max: Point3Mm {
            x: b.max.x.checked_add(x).ok_or(ErrorKind::Overflow)?,
            y: b.max.y.checked_add(y).ok_or(ErrorKind::Overflow)?,
            z: b.max.z.checked_add(z).ok_or(ErrorKind::Overflow)?,
        },
    })
}
fn put_u64(o: &mut Vec<u8>, v: u64) {
    o.extend_from_slice(&v.to_be_bytes());
}
fn put_i64(o: &mut Vec<u8>, v: i64) {
    o.extend_from_slice(&v.to_be_bytes());
}
fn put_str(o: &mut Vec<u8>, v: &str) {
    put_u64(o, v.len() as u64);
    o.extend_from_slice(v.as_bytes());
}
fn put_point(o: &mut Vec<u8>, p: Point3Mm) {
    put_i64(o, p.x);
    put_i64(o, p.y);
    put_i64(o, p.z);
}
fn put_axis(o: &mut Vec<u8>, a: AxisMm) {
    put_u64(o, a.x);
    put_u64(o, a.y);
    put_u64(o, a.z);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> Context {
        Context {
            project_id: "p".into(),
            work_package_id: "wp".into(),
            task_id: "t".into(),
            trajectory_id: "traj".into(),
            robot_id: "r".into(),
            platform_config_sha256: "a".repeat(64),
            tool_config_sha256: "b".repeat(64),
            payload_config_sha256: "c".repeat(64),
            collision_model_sha256: "d".repeat(64),
            primitive_set_ref: "manifest".into(),
            expected_primitive_ids: vec!["tool".into()],
            calibration_ref: "cal".into(),
            controller_profile_ref: "controller".into(),
            frame_id: "frame".into(),
            transform_sha256: "e".repeat(64),
            design_revision_sha256: "f".repeat(64),
            site_epoch: "epoch".into(),
            clock_domain_id: "clock".into(),
            trajectory_profile_sha256: "1".repeat(64),
        }
    }
    fn ev() -> EvidenceRefs {
        EvidenceRefs {
            path_deviation: "path".into(),
            localization: "loc".into(),
            tracking: "track".into(),
            model_uncertainty: "model".into(),
            numerical_enclosure: "numeric".into(),
            stopping_reachability: "stop".into(),
            policy_clearance: "clearance".into(),
        }
    }
    fn seg(i: u32, id: &str, t0: u64, t1: u64, a: Point3Mm, b: Point3Mm) -> Segment {
        Segment {
            slice_index: i,
            primitive_id: id.into(),
            start_ms: t0,
            end_ms: t1,
            from_mm: a,
            to_mm: b,
            radius_mm: 10,
            margins: MarginsMm {
                path_deviation: AxisMm { x: 2, y: 2, z: 2 },
                localization: AxisMm { x: 3, y: 3, z: 3 },
                tracking: AxisMm { x: 4, y: 4, z: 4 },
                model_uncertainty: AxisMm { x: 1, y: 1, z: 1 },
                numerical_enclosure: AxisMm { x: 1, y: 1, z: 1 },
                stopping_reachability: AxisMm { x: 5, y: 0, z: 2 },
                policy_clearance: AxisMm { x: 2, y: 2, z: 2 },
            },
            evidence: ev(),
        }
    }

    #[test]
    fn known_answer_separates_physical_stopping_and_policy_bounds() {
        let s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 100, y: 0, z: 0 },
        );
        let r = build(&context(), &[s]).unwrap();
        let x = &r.slices[0];
        assert_eq!(
            x.physical_swept_outer.min,
            Point3Mm {
                x: -21,
                y: -21,
                z: -21
            }
        );
        assert_eq!(
            x.physical_swept_outer.max,
            Point3Mm {
                x: 121,
                y: 21,
                z: 21
            }
        );
        assert_eq!(
            x.stopping_outer.min,
            Point3Mm {
                x: -26,
                y: -21,
                z: -23
            }
        );
        assert_eq!(
            x.coordination_outer.min,
            Point3Mm {
                x: -28,
                y: -23,
                z: -25
            }
        );
    }

    #[test]
    fn curved_arc_known_answer_is_contained_by_chord_bound() {
        let mut s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm {
                x: 1_000,
                y: 0,
                z: 0,
            },
            Point3Mm {
                x: 0,
                y: 1_000,
                z: 0,
            },
        );
        s.radius_mm = 50;
        s.margins = MarginsMm {
            path_deviation: AxisMm {
                x: 208,
                y: 208,
                z: 0,
            },
            ..MarginsMm::default()
        };
        let r = build(&context(), &[s]).unwrap();
        assert!(r.slices[0].physical_swept_outer.contains_sphere(
            Point3Mm {
                x: 707,
                y: 707,
                z: 0
            },
            50
        ));
    }

    #[test]
    fn canonical_bytes_ignore_input_order_but_bind_tool_revision() {
        let mut ctx = context();
        ctx.expected_primitive_ids = vec!["base".into(), "tool".into()];
        let a = seg(
            0,
            "base",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        let b = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 1, z: 0 },
        );
        let x = build(&ctx, &[a.clone(), b.clone()]).unwrap();
        let y = build(&ctx, &[b.clone(), a.clone()]).unwrap();
        assert_eq!(x.canonical_bytes(), y.canonical_bytes());
        let mut c = context();
        c.tool_config_sha256 = "9".repeat(64);
        c.expected_primitive_ids = vec!["base".into(), "tool".into()];
        assert_ne!(
            x.canonical_bytes(),
            build(&c, &[a, b]).unwrap().canonical_bytes()
        );
    }

    #[test]
    fn rejects_temporal_gap_and_overlap() {
        let a = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 100, y: 0, z: 0 },
        );
        let b = seg(
            1,
            "tool",
            101,
            200,
            Point3Mm { x: 100, y: 0, z: 0 },
            Point3Mm { x: 200, y: 0, z: 0 },
        );
        assert_eq!(
            build(&context(), &[a, b]).unwrap_err(),
            ErrorKind::TimeDiscontinuity
        );
    }

    #[test]
    fn rejects_missing_primitive_in_slice_and_discontinuous_centers() {
        let mut ctx = context();
        ctx.expected_primitive_ids = vec!["base".into(), "tool".into()];
        let a = seg(
            0,
            "base",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        let b = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        let c = seg(
            1,
            "base",
            100,
            200,
            Point3Mm { x: 10, y: 0, z: 0 },
            Point3Mm { x: 20, y: 0, z: 0 },
        );
        assert_eq!(
            build(&ctx, &[a.clone(), b, c.clone()]).unwrap_err(),
            ErrorKind::PrimitiveSetChanged(1)
        );
        let d = seg(
            1,
            "base",
            100,
            200,
            Point3Mm { x: 9, y: 0, z: 0 },
            Point3Mm { x: 20, y: 0, z: 0 },
        );
        let mut only_base_context = context();
        only_base_context.expected_primitive_ids = vec!["base".into()];
        assert!(matches!(
            build(&only_base_context, &[a, d]),
            Err(ErrorKind::CenterDiscontinuity(_))
        ));
    }

    #[test]
    fn rejects_missing_stopping_bound_reference_and_invalid_digest() {
        let mut s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        s.evidence.stopping_reachability.clear();
        assert_eq!(
            build(&context(), &[s]).unwrap_err(),
            ErrorKind::Missing("stopping_reachability_ref")
        );
        let mut c = context();
        c.tool_config_sha256 = "not-a-sha".into();
        let s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        assert_eq!(
            build(&c, &[s]).unwrap_err(),
            ErrorKind::BadDigest("tool_config_sha256")
        );
    }

    #[test]
    fn rejects_whole_trajectory_omitting_declared_tool_or_payload_primitive() {
        let mut c = context();
        c.expected_primitive_ids = vec!["base".into(), "tool".into()];
        let only_base = seg(
            0,
            "base",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        assert_eq!(
            build(&c, &[only_base]).unwrap_err(),
            ErrorKind::PrimitiveSetChanged(0)
        );
    }

    #[test]
    fn rejects_integer_and_coordinate_overflow() {
        let mut s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm::default(),
            Point3Mm { x: 10, y: 0, z: 0 },
        );
        s.margins.path_deviation.x = u64::MAX;
        assert_eq!(build(&context(), &[s]).unwrap_err(), ErrorKind::Overflow);
        let s = seg(
            0,
            "tool",
            0,
            100,
            Point3Mm {
                x: i64::MIN,
                y: 0,
                z: 0,
            },
            Point3Mm {
                x: i64::MIN + 1,
                y: 0,
                z: 0,
            },
        );
        assert_eq!(build(&context(), &[s]).unwrap_err(), ErrorKind::Overflow);
    }
}
