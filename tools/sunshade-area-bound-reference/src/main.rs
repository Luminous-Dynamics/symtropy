use sunshade_area_bound_reference::{
    evaluate_projected_mask_area_bound, full_plane_integrated_blockage_area, AreaBoundInput,
};

fn main() {
    let estimate = evaluate_projected_mask_area_bound(AreaBoundInput {
        source_to_shade_distance_m: 3.0,
        shade_to_target_distance_m: 1.0,
        integrated_target_attenuation_area_m2: 20.0,
        module_transmittance_fraction: 0.2,
    })
    .expect("frozen area-bound reference case must be valid");
    let full_plane_integral =
        full_plane_integrated_blockage_area(7.25, 4.0, 6.0)
            .expect("frozen full-plane integral case must be valid");

    println!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1.0\",\n",
            "  \"evidence_class\": \"synthetic_analytical_reference\",\n",
            "  \"source_to_shade_fraction\": {:.12},\n",
            "  \"projected_mask_union_area_lower_bound_m2\": {:.12},\n",
            "  \"full_plane_integrated_blockage_area_m2\": {:.12},\n",
            "  \"claim_ceiling\": \"common_parallel_planes_uniform_source_geometric_optics_uniform_transmittance_only\"\n",
            "}}"
        ),
        estimate.source_to_shade_fraction,
        estimate.projected_mask_union_area_lower_bound_m2,
        full_plane_integral
    );
}
