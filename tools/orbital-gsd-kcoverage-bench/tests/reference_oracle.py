#!/usr/bin/env python3
"""Independent standard-library 3D voxel-surface visibility oracle."""

import hashlib
import json
import math
import sys
from collections import deque
from pathlib import Path

CASE_SCHEMA = "luminous.orbital-gsd-kcoverage.case.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-gsd-kcoverage.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-GSD-KCOVER-VOXEL-001"
FACES = [
    ((-1,0,0),(-1,0,0)), ((1,0,0),(1,0,0)),
    ((0,-1,0),(0,-1,0)), ((0,1,0),(0,1,0)),
    ((0,0,-1),(0,0,-1)), ((0,0,1),(0,0,1)),
]


def add(a,b): return tuple(x+y for x,y in zip(a,b))
def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def mul(a,s): return tuple(x*s for x in a)
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def norm(a): return math.sqrt(dot(a,a))
def unit(a):
    n=norm(a)
    if n<=1e-12: return None
    return mul(a,1.0/n)
def angle(a,b):
    a=unit(a); b=unit(b)
    if a is None or b is None: return None
    return math.acos(max(-1.0,min(1.0,dot(a,b))))
def quant(value,scale):
    scaled=value*scale
    return int(math.floor(scaled+0.5)) if scaled>=0 else int(math.ceil(scaled-0.5))
def feature_id(cell): return "V-%d-%d-%d"%tuple(cell)


class Grid:
    def __init__(self, inp):
        self.origin=tuple(inp["origin_m"])
        self.s=inp["voxel_size_m"]
        self.dims=tuple(inp["dimensions"])
        self.occupied={tuple(c) for c in inp["occupied_cells"]}
        padded=tuple(n+2 for n in self.dims)
        queue=deque([(0,0,0)])
        exterior_padded={(0,0,0)}
        while queue:
            cell=queue.popleft()
            for delta,_ in FACES:
                nxt=add(cell,delta)
                if any(nxt[i]<0 or nxt[i]>=padded[i] for i in range(3)):
                    continue
                original=tuple(nxt[i]-1 for i in range(3))
                inside=all(0<=original[i]<self.dims[i] for i in range(3))
                if inside and original in self.occupied:
                    continue
                if nxt not in exterior_padded:
                    exterior_padded.add(nxt)
                    queue.append(nxt)
        self.exterior=set()
        self.interior=set()
        for x in range(self.dims[0]):
            for y in range(self.dims[1]):
                for z in range(self.dims[2]):
                    cell=(x,y,z)
                    if cell in self.occupied:
                        continue
                    if (x+1,y+1,z+1) in exterior_padded:
                        self.exterior.add(cell)
                    else:
                        self.interior.add(cell)

    def centre(self,c):
        return tuple(self.origin[i]+(c[i]+0.5)*self.s for i in range(3))

    def occupied_cell(self,c):
        return all(0<=c[i]<self.dims[i] for i in range(3)) and tuple(c) in self.occupied

    def surfaces(self):
        result=[]
        for c in sorted(self.occupied):
            normals=[]
            for delta,normal in FACES:
                neighbour=add(c,delta)
                if self.occupied_cell(neighbour):
                    continue
                inside=all(0<=neighbour[i]<self.dims[i] for i in range(3))
                if not inside or neighbour in self.exterior:
                    normals.append(normal)
            if not normals:
                continue
            summed=tuple(sum(normal[i] for normal in normals) for i in range(3))
            if norm(summed)<=1e-9:
                for index,normal in enumerate(normals):
                    result.append({
                        "feature_id":f"{feature_id(c)}-N{index}",
                        "cell":list(c),
                        "normal_q1e6":[quant(v,1e6) for v in normal],
                        "normal_source":"EXTERIOR_FACE_MULTI_NORMAL"
                    })
            else:
                normal=unit(summed)
                result.append({
                    "feature_id":feature_id(c),
                    "cell":list(c),
                    "normal_q1e6":[quant(v,1e6) for v in normal],
                    "normal_source":"NORMALIZED_EXTERIOR_FACE_SUM"
                })
        return sorted(result,key=lambda row:row["feature_id"])

    def occluded(self,origin,target,target_cell):
        segment=sub(target,origin)
        distance=norm(segment)
        direction=unit(segment)
        if direction is None:
            return True
        tolerance=self.s*math.sqrt(3.0)/2.0
        end=max(0.0,distance-tolerance)
        cell=tuple(math.floor((origin[i]-self.origin[i])/self.s) for i in range(3))
        step=[0,0,0]
        tmax=[math.inf,math.inf,math.inf]
        tdelta=[math.inf,math.inf,math.inf]
        for axis in range(3):
            if direction[axis]>0:
                step[axis]=1
                boundary=self.origin[axis]+(cell[axis]+1)*self.s
                tmax[axis]=(boundary-origin[axis])/direction[axis]
                tdelta[axis]=self.s/direction[axis]
            elif direction[axis]<0:
                step[axis]=-1
                boundary=self.origin[axis]+cell[axis]*self.s
                tmax[axis]=(boundary-origin[axis])/direction[axis]
                tdelta[axis]=-self.s/direction[axis]
        target=tuple(target_cell)
        t=0.0
        for _ in range(1024):
            if t>end:
                break
            if self.occupied_cell(cell) and tuple(cell)!=target:
                return True
            next_t=min(tmax)
            if not math.isfinite(next_t) or next_t>end:
                break
            tied=[axis for axis in range(3) if abs(tmax[axis]-next_t)<=1e-10]
            for mask in range(1,1<<len(tied)):
                neighbour=list(cell)
                for bit,axis in enumerate(tied):
                    if mask&(1<<bit):
                        neighbour[axis]+=step[axis]
                if self.occupied_cell(neighbour) and tuple(neighbour)!=target:
                    return True
            cell=list(cell)
            for axis in tied:
                cell[axis]+=step[axis]
                tmax[axis]+=tdelta[axis]
            cell=tuple(cell)
            t=next_t
        return False


def cross(a,b):
    return (
        a[1]*b[2]-a[2]*b[1],
        a[2]*b[0]-a[0]*b[2],
        a[0]*b[1]-a[1]*b[0],
    )


def evaluate(case,raw,source_commit):
    if case["schema"]!=CASE_SCHEMA or case["benchmark_id"]!=BENCHMARK_ID:
        raise ValueError("CASE_IDENTITY_INVALID")
    if (case["evidence_class"]!="synthetic"
            or case["claim_ceiling"]!="synthetic_3d_voxel_surface_candidate_view_visibility_only"):
        raise ValueError("CASE_CLAIM_CEILING_INVALID")
    cam=case["camera"]
    aug=case["augmentation"]
    if (not 0<cam["horizontal_fov_rad"]<math.pi
            or cam["horizontal_pixel_count"]<=0
            or cam["gsd_max_m_per_pixel"]<=0
            or not 0<=cam["max_oblique_angle_rad"]<math.pi/2):
        raise ValueError("CAMERA_MODEL_INVALID")
    if aug["requested_k"]<=0 or aug["requested_k"]>20:
        raise ValueError("REQUESTED_K_OUT_OF_BOUNDS")
    if (len(aug["tilt_angles_deg"])>90 or len(aug["azimuth_angles_deg"])>12
            or any(not math.isfinite(v) or v<1 or v>90 for v in aug["tilt_angles_deg"])
            or any(not math.isfinite(v) or v<0 or v>=360 for v in aug["azimuth_angles_deg"])):
        raise ValueError("AUGMENTATION_ANGLE_INPUT_INVALID")
    if len(aug["tilt_angles_deg"])!=len(set(aug["tilt_angles_deg"])):
        raise ValueError("DUPLICATE_TILT_ANGLE")
    if len(aug["azimuth_angles_deg"])!=len(set(aug["azimuth_angles_deg"])):
        raise ValueError("DUPLICATE_AZIMUTH_ANGLE")

    grid=Grid(case["grid"])
    fov_scale=2*math.tan(cam["horizontal_fov_rad"]/2)
    slant_range=(
        cam["gsd_max_m_per_pixel"]*cam["horizontal_pixel_count"]
        *math.cos(cam["max_oblique_angle_rad"])/fov_scale
    )
    perpendicular=slant_range*math.cos(cam["max_oblique_angle_rad"])
    surfaces=grid.surfaces()
    views=[]

    def visible_features(position,position_direction):
        optical=mul(position_direction,-1.0)
        visible=[]
        for target in surfaces:
            target_normal=tuple(value/1e6 for value in target["normal_q1e6"])
            target_centre=grid.centre(target["cell"])
            ray=sub(target_centre,position)
            distance=norm(ray)
            if distance<=1e-12 or distance>slant_range+1e-10:
                continue
            deviation=angle(position_direction,target_normal)
            if deviation is None or deviation>cam["max_oblique_angle_rad"]+1e-10:
                continue
            fov=angle(optical,ray)
            if fov is None or fov>cam["horizontal_fov_rad"]/2+1e-10:
                continue
            gsd=distance*fov_scale/(cam["horizontal_pixel_count"]*math.cos(deviation))
            if gsd>cam["gsd_max_m_per_pixel"]+1e-12:
                continue
            if grid.occluded(position,target_centre,target["cell"]):
                continue
            visible.append(target["feature_id"])
        return sorted(visible)

    # Initial candidate viewpoints along each surface-patch normal.
    for surface in surfaces:
        normal=unit(tuple(value/1e6 for value in surface["normal_q1e6"]))
        centre=grid.centre(surface["cell"])
        position=add(centre,mul(normal,perpendicular))
        if grid.occluded(position,centre,surface["cell"]):
            continue
        visible=visible_features(position,normal)
        views.append({
            "view_id":f"VIEW-{len(views)}",
            "source_feature_id":surface["feature_id"],
            "source_cell":surface["cell"],
            "position_direction_q1e6":[quant(value,1e6) for value in normal],
            "position_um":[quant(value,1e6) for value in position],
            "optical_axis_q1e6":[quant(value,1e6) for value in mul(normal,-1.0)],
            "generation_kind":"INITIAL_NORMAL",
            "tilt_degrees_millideg":None,
            "azimuth_degrees_millideg":None,
            "visible_feature_ids":visible
        })

    initial_candidate_view_count=len(views)
    initial_coverage=[
        sum(surface["feature_id"] in view["visible_feature_ids"] for view in views)
        for surface in surfaces
    ]
    undercovered=[
        surface["feature_id"]
        for surface,count in zip(surfaces,initial_coverage)
        if count<aug["requested_k"]
    ]
    upper_bound=(
        len(undercovered)*len(aug["tilt_angles_deg"])*len(aug["azimuth_angles_deg"])
    )
    if initial_candidate_view_count+upper_bound>20000:
        raise ValueError("CANDIDATE_VIEW_LIMIT_EXCEEDED")

    augmentation_candidate_attempt_count = 0
    augmentation_rejected_max_oblique_count = 0
    augmentation_rejected_source_occlusion_count = 0
    augmentation_rejected_source_visibility_count = 0

    # Deterministic local tangent basis, using the least-aligned Cartesian axis.
    for surface,count in zip(surfaces,initial_coverage):
        if count>=aug["requested_k"]:
            continue
        normal=unit(tuple(value/1e6 for value in surface["normal_q1e6"]))
        reference=(
            (1.0,0.0,0.0) if abs(normal[0])<=abs(normal[1]) and abs(normal[0])<=abs(normal[2])
            else (0.0,1.0,0.0) if abs(normal[1])<=abs(normal[2])
            else (0.0,0.0,1.0)
        )
        tangent_one=unit(sub(reference,mul(normal,dot(reference,normal))))
        tangent_two=unit(cross(normal,tangent_one))
        centre=grid.centre(surface["cell"])

        for tilt_deg in aug["tilt_angles_deg"]:
            beta=math.radians(tilt_deg)
            for azimuth_deg in aug["azimuth_angles_deg"]:
                augmentation_candidate_attempt_count += 1
                phi=math.radians(azimuth_deg)
                tangent=add(mul(tangent_one,math.cos(phi)),mul(tangent_two,math.sin(phi)))
                direction=unit(add(mul(normal,math.cos(beta)),mul(tangent,math.sin(beta))))
                source_deviation=angle(direction,normal)
                if source_deviation is None or source_deviation>cam["max_oblique_angle_rad"]+1e-10:
                    augmentation_rejected_max_oblique_count += 1
                    continue
                position=add(centre,mul(direction,perpendicular))
                if grid.occluded(position,centre,surface["cell"]):
                    augmentation_rejected_source_occlusion_count += 1
                    continue
                visible=visible_features(position,direction)
                if surface["feature_id"] not in visible:
                    augmentation_rejected_source_visibility_count += 1
                    continue
                views.append({
                    "view_id":f"VIEW-{len(views)}",
                    "source_feature_id":surface["feature_id"],
                    "source_cell":surface["cell"],
                    "position_direction_q1e6":[quant(value,1e6) for value in direction],
                    "position_um":[quant(value,1e6) for value in position],
                    "optical_axis_q1e6":[quant(value,1e6) for value in mul(direction,-1.0)],
                    "generation_kind":"AUGMENTED_TILT_AZIMUTH",
                    "tilt_degrees_millideg":quant(tilt_deg,1e3),
                    "azimuth_degrees_millideg":quant(azimuth_deg,1e3),
                    "visible_feature_ids":visible
                })

    augmented_candidate_view_count=len(views)-initial_candidate_view_count
    accounted_augmentation_candidates = (
        augmented_candidate_view_count
        + augmentation_rejected_max_oblique_count
        + augmentation_rejected_source_occlusion_count
        + augmentation_rejected_source_visibility_count
    )
    if accounted_augmentation_candidates != augmentation_candidate_attempt_count:
        raise ValueError("AUGMENTATION_CANDIDATE_ACCOUNTING_MISMATCH")
    coverage=[]
    for index,surface in enumerate(surfaces):
        seen=sorted(
            view["view_id"] for view in views
            if surface["feature_id"] in view["visible_feature_ids"]
        )
        coverage.append({
            "feature_id":surface["feature_id"],
            "initial_candidate_view_count":initial_coverage[index],
            "candidate_view_count":len(seen),
            "seen_by_view_ids":seen
        })
    requested_k_covered=all(row["candidate_view_count"]>=aug["requested_k"] for row in coverage)
    any_unobservable=any(row["candidate_view_count"]==0 for row in coverage)
    disposition=(
        "INFEASIBLE" if any_unobservable
        else "FEASIBLE" if requested_k_covered
        else "INDETERMINATE"
    )

    return {
        "schema":RECEIPT_SCHEMA,
        "benchmark_id":BENCHMARK_ID,
        "source_commit":source_commit.lower(),
        "case_id":case["case_id"],
        "coordinate_frame_id":case["coordinate_frame_id"],
        "source_input_sha256":hashlib.sha256(raw).hexdigest(),
        "evidence_class":"synthetic",
        "claim_ceiling":case["claim_ceiling"],
        "disposition":disposition,
        "grid_dimensions":grid.dims,
        "occupied_cell_count":len(grid.occupied),
        "exterior_empty_cell_count":len(grid.exterior),
        "interior_empty_cell_count":len(grid.interior),
        "surface_patch_count":len(surfaces),
        "multi_normal_patch_count":sum(
            row["normal_source"]=="EXTERIOR_FACE_MULTI_NORMAL" for row in surfaces
        ),
        "horizontal_fov_microrad":quant(cam["horizontal_fov_rad"],1e6),
        "horizontal_pixel_count":cam["horizontal_pixel_count"],
        "gsd_max_nanom_per_pixel":quant(cam["gsd_max_m_per_pixel"],1e9),
        "max_oblique_angle_microrad":quant(cam["max_oblique_angle_rad"],1e6),
        "slant_range_max_um":quant(slant_range,1e6),
        "perpendicular_distance_um":quant(perpendicular,1e6),
        "requested_k":aug["requested_k"],
        "initial_candidate_view_count":initial_candidate_view_count,
        "augmentation_candidate_attempt_count":augmentation_candidate_attempt_count,
        "augmentation_rejected_max_oblique_count":augmentation_rejected_max_oblique_count,
        "augmentation_rejected_source_occlusion_count":augmentation_rejected_source_occlusion_count,
        "augmentation_rejected_source_visibility_count":augmentation_rejected_source_visibility_count,
        "augmented_candidate_view_count":augmented_candidate_view_count,
        "undercovered_feature_ids_before_augmentation":undercovered,
        "requested_k_covered_after_augmentation":requested_k_covered,
        "candidate_view_count":len(views),
        "visible_relation_count":sum(len(view["visible_feature_ids"]) for view in views),
        "surface_features":surfaces,
        "candidate_views":views,
        "feature_coverage":coverage,
        "authority_granted":False,
        "structural_acceptance_authorized":False,
        "flight_authorized":False,
        "physical_validation":"NOT_ASSESSED"
    }

def main():
    if len(sys.argv)!=3:
        raise SystemExit("usage: reference_oracle.py <case.json> <40-hex-source-commit>")
    raw=Path(sys.argv[1]).read_bytes()
    source_commit=sys.argv[2]
    if len(source_commit)!=40 or any(char not in "0123456789abcdefABCDEF" for char in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    case=json.loads(raw)
    print(json.dumps(evaluate(case,raw,source_commit),indent=2,sort_keys=True))


if __name__=="__main__":
    main()
