// Luminous Productive Machine Platform — LMF-FDM-300 envelope concept v0.1
// Status: CONCEPT GEOMETRY ONLY. Not a fabrication drawing or validated design.
// The model communicates a 300 mm-class CoreXY printer envelope and modular frame.
// Frame sizes, sections, joints, rails, belts, bed supports, thermal design,
// guarding, electrical protection and stiffness all require engineering review.
// Hardware licence is not yet assigned; do not redistribute derivatives as licensed.

$fn = 32;

// Editable design assumptions (millimetres)
build_x = 300;
build_y = 300;
build_z = 300;
frame_x = 520;
frame_y = 520;
frame_z = 620;
extrusion = 20;
rail_section = 20;
bed_x = 340;
bed_y = 340;
bed_t = 8;
bed_z = 142;
show_build_envelope = true;
show_gantry_concept = true;

module bar_x(length, section, z, y=0, x=0) {
    translate([x-length/2, y-section/2, z-section/2])
        cube([length, section, section]);
}

module bar_y(length, section, z, x=0, y=0) {
    translate([x-section/2, y-length/2, z-section/2])
        cube([section, length, section]);
}

module upright(section, height, x, y, z0=0) {
    translate([x-section/2, y-section/2, z0])
        cube([section, section, height]);
}

module frame_rails() {
    // Lower and upper rectangular rings.
    for (z = [extrusion/2, frame_z-extrusion/2]) {
        for (y = [-frame_y/2+extrusion/2, frame_y/2-extrusion/2])
            bar_x(frame_x, extrusion, z, y, 0);
        for (x = [-frame_x/2+extrusion/2, frame_x/2-extrusion/2])
            bar_y(frame_y, extrusion, z, x, 0);
    }
    // Four vertical corner uprights.
    for (x = [-frame_x/2+extrusion/2, frame_x/2-extrusion/2])
        for (y = [-frame_y/2+extrusion/2, frame_y/2-extrusion/2])
            upright(extrusion, frame_z-extrusion, x, y, extrusion);
    // Lower cross-members represent a base support concept.
    bar_x(frame_x-2*extrusion, extrusion, 55, 0, 0);
    bar_y(frame_y-2*extrusion, extrusion, 55, 0, 0);
    // Rear mid-height tie member; exact bracing is an engineering task.
    bar_x(frame_x-2*extrusion, extrusion, 300, -frame_y/2+extrusion/2, 0);
}

module bed_concept() {
    color("silver")
        translate([-bed_x/2, -bed_y/2, bed_z])
            cube([bed_x, bed_y, bed_t]);
    // Placeholder Z support columns; not a selected kinematic design.
    color("dimgray")
        for (x = [-175, 175])
            for (y = [-175, 175])
                translate([x-5, y-5, 60]) cube([10, 10, bed_z-60]);
}

module gantry_concept() {
    // Simplified beam, rail and toolhead envelope only; no belt path is defined.
    color("slategray")
        translate([-210, -10, bed_z+build_z+20])
            cube([420, 20, 20]);
    color("darkslategray")
        translate([-210, -22, bed_z+build_z+6])
            cube([420, 8, 8]);
    color("orangered")
        translate([-32, -32, bed_z+build_z-25])
            cube([64, 64, 55]);
    // Side rail placeholders.
    color("dimgray")
        for (y = [-205, 205])
            translate([-8, y-6, bed_z+build_z-4])
                cube([16, 12, 12]);
}

module build_envelope() {
    // Transparent conceptual volume above the build plate.
    color([0.15, 0.45, 0.9, 0.18])
        translate([-build_x/2, -build_y/2, bed_z+bed_t])
            cube([build_x, build_y, build_z]);
}

// Main scene
color("dimgray") frame_rails();
bed_concept();
if (show_gantry_concept) gantry_concept();
if (show_build_envelope) build_envelope();
