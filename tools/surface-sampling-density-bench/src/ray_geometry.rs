use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-sampling-density.geometry-corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-sampling-density.geometry-receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-RAY-TRIANGLE-001";
pub const CLAIM_CEILING: &str = "synthetic_pinhole_pixel_centre_rays_and_nearest_triangle_intersections_only";
pub const CASE_COUNT: usize = 4;
const FRAME_TOLERANCE: f64 = 1.0e-9;
const BARYCENTRIC_TOLERANCE: f64 = 1.0e-12;
const RAY_TIE_EPSILON_M: f64 = 1.0e-9;
const MINIMUM_RAY_DISTANCE_M: f64 = 1.0e-9;
const MAX_PIXELS_PER_VIEW: u64 = 65_536;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<GeometryCase>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryCase {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub area_unit: String,
    pub cameras: Vec<PinholeCamera>,
    pub elements: Vec<GeometryElement>,
    pub expected_hit_counts: Vec<ExpectedHitCount>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PinholeCamera {
    pub view_id: String,
    pub origin_m: [f64; 3],
    pub forward_axis: [f64; 3],
    pub right_axis: [f64; 3],
    pub up_axis: [f64; 3],
    pub horizontal_fov_rad: f64,
    pub width_px: u32,
    pub height_px: u32,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryElement {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub triangles: Vec<Triangle>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Triangle {
    pub triangle_id: String,
    pub vertices_m: [[f64; 3]; 3],
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedHitCount {
    pub element_id: String,
    pub view_id: String,
    pub hit_pixel_count: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ElementViewHitCount {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub view_id: String,
    pub hit_pixel_count: u64,
    pub expected_hit_pixel_count: u64,
    pub expected_matches: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewRaySummary {
    pub view_id: String,
    pub rays_cast: u64,
    pub hit_pixel_count: u64,
    pub background_pixel_count: u64,
    pub tie_pixel_count: u64,
    pub triangle_tests: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GeometryCaseReceipt {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub ray_model: String,
    pub area_provenance: String,
    pub element_count: usize,
    pub triangle_count: usize,
    pub view_count: usize,
    pub expected_hit_count_rows: usize,
    pub expected_matches: usize,
    pub expected_mismatches: usize,
    pub total_rays_cast: u64,
    pub total_hit_pixels: u64,
    pub total_background_pixels: u64,
    pub total_tie_pixels: u64,
    pub element_view_hits: Vec<ElementViewHitCount>,
    pub views: Vec<ViewRaySummary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GeometryCorpusReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub fixture_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<GeometryCaseReceipt>,
}

#[derive(Debug, Clone, Copy)]
struct V3 { x: f64, y: f64, z: f64 }
impl V3 {
    fn new(v: [f64;3])->Self { Self{x:v[0],y:v[1],z:v[2]} }
    fn add(self,b:Self)->Self { Self{x:self.x+b.x,y:self.y+b.y,z:self.z+b.z} }
    fn sub(self,b:Self)->Self { Self{x:self.x-b.x,y:self.y-b.y,z:self.z-b.z} }
    fn mul(self,k:f64)->Self { Self{x:self.x*k,y:self.y*k,z:self.z*k} }
    fn dot(self,b:Self)->f64 { self.x*b.x+self.y*b.y+self.z*b.z }
    fn cross(self,b:Self)->Self { Self{x:self.y*b.z-self.z*b.y,y:self.z*b.x-self.x*b.z,z:self.x*b.y-self.y*b.x} }
    fn norm(self)->f64 { self.dot(self).sqrt() }
    fn unit(self)->Option<Self> { let n=self.norm(); (n.is_finite()&&n>MINIMUM_RAY_DISTANCE_M).then(||self.mul(1.0/n)) }
    fn finite(self)->bool { self.x.is_finite()&&self.y.is_finite()&&self.z.is_finite() }
}
struct Tri<'a> { element:&'a str,id:&'a str,a:V3,b:V3,c:V3 }
struct Hit<'a> { element:&'a str,triangle:&'a str,t:f64 }

fn validate_camera(c:&PinholeCamera)->Result<(),String>{
    let o=V3::new(c.origin_m); let f=V3::new(c.forward_axis);
    let r=V3::new(c.right_axis); let u=V3::new(c.up_axis);
    if c.view_id.trim().is_empty()||!o.finite()||!f.finite()||!r.finite()||!u.finite()
        ||!c.horizontal_fov_rad.is_finite()||c.horizontal_fov_rad<=0.0
        ||c.horizontal_fov_rad>=std::f64::consts::PI||c.width_px==0||c.height_px==0
        ||c.width_px>256||c.height_px>256
        ||u64::from(c.width_px)*u64::from(c.height_px)>MAX_PIXELS_PER_VIEW{
        return Err("CAMERA_MODEL_BOUNDS_INVALID".into());
    }
    if (f.norm()-1.0).abs()>FRAME_TOLERANCE||(r.norm()-1.0).abs()>FRAME_TOLERANCE
        ||(u.norm()-1.0).abs()>FRAME_TOLERANCE||f.dot(r).abs()>FRAME_TOLERANCE
        ||f.dot(u).abs()>FRAME_TOLERANCE||r.dot(u).abs()>FRAME_TOLERANCE
        ||(r.cross(u).dot(f)+1.0).abs()>FRAME_TOLERANCE{
        return Err("CAMERA_BASIS_MUST_BE_ORTHONORMAL_AND_RIGHT_HANDED".into());
    }
    Ok(())
}

fn intersect(o:V3,d:V3,t:&Tri<'_>)->Option<f64>{
    let e1=t.b.sub(t.a); let e2=t.c.sub(t.a);
    let p=d.cross(e2); let det=e1.dot(p);
    if !det.is_finite()||det.abs()<=1.0e-12{return None;}
    let inv=1.0/det; let tv=o.sub(t.a); let u=tv.dot(p)*inv;
    if !u.is_finite()||u < -BARYCENTRIC_TOLERANCE||u>1.0+BARYCENTRIC_TOLERANCE{return None;}
    let q=tv.cross(e1); let v=d.dot(q)*inv;
    if !v.is_finite()||v < -BARYCENTRIC_TOLERANCE||u+v>1.0+BARYCENTRIC_TOLERANCE{return None;}
    let distance=e2.dot(q)*inv;
    (distance.is_finite()&&distance>MINIMUM_RAY_DISTANCE_M).then_some(distance)
}

fn pixel_ray(c:&PinholeCamera,col:u32,row:u32)->Result<V3,String>{
    let f=V3::new(c.forward_axis); let r=V3::new(c.right_axis); let u=V3::new(c.up_axis);
    let h=(c.horizontal_fov_rad*0.5).tan();
    let x=(2.0*(f64::from(col)+0.5)/f64::from(c.width_px)-1.0)*h;
    let y=(1.0-2.0*(f64::from(row)+0.5)/f64::from(c.height_px))*h
        *f64::from(c.height_px)/f64::from(c.width_px);
    f.add(r.mul(x)).add(u.mul(y)).unit().ok_or_else(||"CAMERA_PIXEL_RAY_NORMALIZATION_FAILED".into())
}

fn validate_case(c:&GeometryCase)->Result<(),String>{
    if c.case_id.trim().is_empty()||c.coordinate_frame_id.trim().is_empty()
        ||c.area_unit!="SQUARE_MICROMETRE"||c.cameras.is_empty()||c.elements.is_empty(){
        return Err("GEOMETRY_CASE_ID_FRAME_UNITS_OR_COLLECTIONS_INVALID".into());
    }
    let mut views=BTreeSet::new();
    for camera in &c.cameras{validate_camera(camera)?;if !views.insert(camera.view_id.as_str()){return Err("DUPLICATE_CAMERA_VIEW_ID".into());}}
    let mut elements=BTreeSet::new();let mut triangles=BTreeSet::new();
    for e in &c.elements{
        if e.element_id.trim().is_empty()||e.mesh_revision_id.trim().is_empty()||e.area_um2==0
            ||e.triangles.is_empty()||!elements.insert(e.element_id.as_str()){
            return Err("SURFACE_ELEMENT_ID_AREA_OR_TRIANGLES_INVALID".into());
        }
        for t in &e.triangles{
            if t.triangle_id.trim().is_empty()||!triangles.insert(t.triangle_id.as_str()){return Err("TRIANGLE_ID_DUPLICATE_OR_BLANK".into());}
            if t.vertices_m.iter().flatten().any(|v|!v.is_finite()){return Err("TRIANGLE_VERTEX_NONFINITE".into());}
            let a=V3::new(t.vertices_m[0]);let b=V3::new(t.vertices_m[1]);let d=V3::new(t.vertices_m[2]);
            if b.sub(a).cross(d.sub(a)).norm()<=1.0e-12{return Err("TRIANGLE_DEGENERATE".into());}
        }
    }
    let required:BTreeSet<(&str,&str)>=elements.iter().flat_map(|e|views.iter().map(move|v|(*e,*v))).collect();
    let actual:BTreeSet<(&str,&str)>=c.expected_hit_counts.iter().map(|r|(r.element_id.as_str(),r.view_id.as_str())).collect();
    if actual.len()!=c.expected_hit_counts.len()||actual!=required{return Err("EXPECTED_HIT_MATRIX_MUST_MATCH_ELEMENT_VIEW_PRODUCT".into());}
    Ok(())
}

fn count_view(c:&PinholeCamera,tris:&[Tri<'_>])->Result<(BTreeMap<String,u64>,ViewRaySummary),String>{
    let rays=u64::from(c.width_px)*u64::from(c.height_px);
    let mut counts=BTreeMap::new();let mut hits=0_u64;let mut ties=0_u64;let mut tests=0_u64;
    let origin=V3::new(c.origin_m);
    for row in 0..c.height_px{for col in 0..c.width_px{
        let d=pixel_ray(c,col,row)?;let mut best:Option<Hit<'_>>=None;let mut tied=false;
        for t in tris{
            tests=tests.checked_add(1).ok_or_else(||"TRIANGLE_TEST_COUNT_OVERFLOW".to_owned())?;
            let Some(distance)=intersect(origin,d,t) else{continue};
            let action=best.as_ref().map(|old|{
                if distance<old.t-RAY_TIE_EPSILON_M{1_u8}
                else if (distance-old.t).abs()<=RAY_TIE_EPSILON_M{2_u8}else{0_u8}
            });
            match action{
                None|Some(1)=>{best=Some(Hit{element:t.element,triangle:t.id,t:distance});tied=false;}
                Some(2)=>{
                    tied=true;let old=best.as_ref().expect("tie has old hit");let nearest=old.t.min(distance);
                    if (t.element,t.id)<(old.element,old.triangle){best=Some(Hit{element:t.element,triangle:t.id,t:nearest});}
                    else if let Some(old)=best.as_mut(){old.t=nearest;}
                }
                Some(0)=>{},
                _=>unreachable!("intersection comparator"),
            }
        }
        if let Some(hit)=best{hits+=1;*counts.entry(hit.element.to_owned()).or_insert(0)+=1;if tied{ties+=1;}}
    }}
    Ok((counts,ViewRaySummary{view_id:c.view_id.clone(),rays_cast:rays,hit_pixel_count:hits,
        background_pixel_count:rays-hits,tie_pixel_count:ties,triangle_tests:tests}))
}

fn evaluate_case(c:&GeometryCase)->Result<GeometryCaseReceipt,String>{
    validate_case(c)?;
    let mut tris=Vec::new();
    for e in &c.elements{for t in &e.triangles{tris.push(Tri{element:&e.element_id,id:&t.triangle_id,
        a:V3::new(t.vertices_m[0]),b:V3::new(t.vertices_m[1]),c:V3::new(t.vertices_m[2])});}}
    tris.sort_by(|a,b|(a.element,a.id).cmp(&(b.element,b.id)));
    let expected:BTreeMap<(&str,&str),u64>=c.expected_hit_counts.iter().map(|r|((r.element_id.as_str(),r.view_id.as_str()),r.hit_pixel_count)).collect();
    let mut hit_rows=Vec::new();let mut view_rows=Vec::new();let mut total_rays=0_u64;let mut total_hits=0_u64;let mut total_ties=0_u64;
    let mut cameras:Vec<&PinholeCamera>=c.cameras.iter().collect();cameras.sort_by(|a,b|a.view_id.cmp(&b.view_id));
    let mut elements:Vec<&GeometryElement>=c.elements.iter().collect();elements.sort_by(|a,b|a.element_id.cmp(&b.element_id));
    for camera in cameras{
        let (counts,summary)=count_view(camera,&tris)?;
        total_rays=total_rays.checked_add(summary.rays_cast).ok_or_else(||"TOTAL_RAY_COUNT_OVERFLOW".to_owned())?;
        total_hits=total_hits.checked_add(summary.hit_pixel_count).ok_or_else(||"TOTAL_HIT_PIXEL_COUNT_OVERFLOW".to_owned())?;
        total_ties=total_ties.checked_add(summary.tie_pixel_count).ok_or_else(||"TOTAL_TIE_PIXEL_COUNT_OVERFLOW".to_owned())?;
        for e in &elements{
            let actual=*counts.get(&e.element_id).unwrap_or(&0);let want=*expected.get(&(e.element_id.as_str(),camera.view_id.as_str())).expect("complete expected-hit matrix");
            hit_rows.push(ElementViewHitCount{element_id:e.element_id.clone(),mesh_revision_id:e.mesh_revision_id.clone(),
                area_um2:e.area_um2,view_id:camera.view_id.clone(),hit_pixel_count:actual,
                expected_hit_pixel_count:want,expected_matches:actual==want});
        }
        view_rows.push(summary);
    }
    let matches=hit_rows.iter().filter(|r|r.expected_matches).count();let mismatches=hit_rows.len()-matches;
    Ok(GeometryCaseReceipt{case_id:c.case_id.clone(),coordinate_frame_id:c.coordinate_frame_id.clone(),
        ray_model:"IDEAL_PINHOLE_PIXEL_CENTRE_NEAREST_TRIANGLE;MOLLER_TRUMBORE_TWO_SIDED".into(),
        area_provenance:"CALLER_DECLARED_SYNTHETIC;AREA_NOT_DERIVED_OR_PHYSICALLY_MEASURED".into(),
        element_count:c.elements.len(),triangle_count:tris.len(),view_count:c.cameras.len(),
        expected_hit_count_rows:hit_rows.len(),expected_matches:matches,expected_mismatches:mismatches,
        total_rays_cast:total_rays,total_hit_pixels:total_hits,total_background_pixels:total_rays-total_hits,
        total_tie_pixels:total_ties,element_view_hits:hit_rows,views:view_rows})
}

pub fn audit_geometry_corpus(bytes:&[u8],source_commit:&str)->Result<GeometryCorpusReceipt,String>{
    if source_commit.len()!=40||!source_commit.bytes().all(|v|v.is_ascii_hexdigit()){return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".into());}
    let corpus:GeometryCorpus=serde_json::from_slice(bytes).map_err(|e|format!("RAY_GEOMETRY_CORPUS_PARSE_FAILED:{e}"))?;
    if corpus.schema!=CORPUS_SCHEMA||corpus.benchmark_id!=BENCHMARK_ID||corpus.evidence_class!="synthetic"
        ||corpus.claim_ceiling!=CLAIM_CEILING||corpus.cases.len()!=CASE_COUNT{
        return Err("RAY_GEOMETRY_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".into());
    }
    let mut ids=BTreeSet::new();let mut cases=Vec::with_capacity(corpus.cases.len());let mut matches=0_usize;
    for c in &corpus.cases{
        if !ids.insert(c.case_id.as_str()){return Err("RAY_GEOMETRY_CASE_ID_DUPLICATE".into());}
        let r=evaluate_case(c)?;matches+=usize::from(r.expected_mismatches==0);cases.push(r);
    }
    let digest=Sha256::digest(bytes);let hash=digest.iter().map(|b|format!("{b:02x}")).collect();
    Ok(GeometryCorpusReceipt{schema:RECEIPT_SCHEMA.into(),benchmark_id:BENCHMARK_ID.into(),
        source_commit:source_commit.to_ascii_lowercase(),fixture_sha256:hash,evidence_class:corpus.evidence_class,
        claim_ceiling:corpus.claim_ceiling,case_count:cases.len(),expected_outcome_matches:matches,
        expected_outcome_mismatches:cases.len()-matches,authority_granted:false,physical_validation:"NOT_ASSESSED".into(),cases})
}


#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str =
        include_str!("../../../docs/research/orbital-truss/fixtures/ray-geometry-v0_1.json");

    #[test]
    fn frozen_pixel_centre_ray_corpus_matches_all_analytic_counts() {
        let receipt = audit_geometry_corpus(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        assert_eq!(receipt.case_count, CASE_COUNT);
        assert_eq!(receipt.expected_outcome_matches, CASE_COUNT);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert_eq!(receipt.cases[0].total_hit_pixels, 5);
        assert_eq!(receipt.cases[1].total_hit_pixels, 5);
        assert_eq!(receipt.cases[2].total_hit_pixels, 10);
        assert_eq!(receipt.cases[3].total_hit_pixels, 4);
        assert_eq!(receipt.cases[3].total_tie_pixels, 2);
    }

    #[test]
    fn non_orthonormal_camera_basis_is_rejected() {
        let mut corpus: GeometryCorpus = serde_json::from_str(FIXTURE).unwrap();
        corpus.cases[0].cameras[0].right_axis = [0.9, 0.0, 0.0];
        assert_eq!(
            validate_camera(&corpus.cases[0].cameras[0]),
            Err("CAMERA_BASIS_MUST_BE_ORTHONORMAL_AND_RIGHT_HANDED".to_owned())
        );
    }

    #[test]
    fn degenerate_triangle_is_rejected() {
        let mut corpus: GeometryCorpus = serde_json::from_str(FIXTURE).unwrap();
        corpus.cases[0].elements[0].triangles[0].vertices_m =
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        assert_eq!(
            validate_case(&corpus.cases[0]),
            Err("TRIANGLE_DEGENERATE".to_owned())
        );
    }

    #[test]
    fn declared_area_is_preserved_for_downstream_density_accounting() {
        let corpus: GeometryCorpus = serde_json::from_str(FIXTURE).unwrap();
        let receipt = evaluate_case(&corpus.cases[0]).unwrap();
        assert_eq!(receipt.element_view_hits[0].area_um2, 2_000_000_000_000);
        assert_eq!(receipt.element_view_hits[0].hit_pixel_count, 5);
    }

    #[test]
    fn element_triangle_and_camera_ordering_do_not_change_receipt() {
        let mut corpus: GeometryCorpus = serde_json::from_str(FIXTURE).unwrap();
        let original = evaluate_case(&corpus.cases[3]).unwrap();
        corpus.cases[3].elements.reverse();
        corpus.cases[3].cameras.reverse();
        for element in &mut corpus.cases[3].elements {
            element.triangles.reverse();
        }
        assert_eq!(evaluate_case(&corpus.cases[3]).unwrap(), original);
    }
}
