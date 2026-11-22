// Luminous Productive Machine Platform — LMF-FDM-300 kinematic concept comparison v0.1
// Status: VISUAL ARCHITECTURE STUDY ONLY. Not fabrication CAD or a validated design.
// Both concepts use the same provisional 300 mm-class build envelope and frame envelope.
// A: fixed XY gantry, Z-moving build platform.
// B: fixed build platform, XY gantry moving in Z.
// Motion rails, belt paths, bearing blocks, motors, joints, electricals, heaters,
// guarding, thermal design, stiffness, tolerance, safety and service clearances are placeholders.
// No mechanical architecture is selected by this comparison.

$fn = 24;

// Shared screening envelope (mm; inherited from concept v0.1, not frozen)
frame_x = 520;
frame_y = 520;
frame_z = 620;
section = 20;
build_x = 300;
build_y = 300;
build_z = 300;
bed_x = 340;
bed_y = 340;
bed_t = 8;

// Display spacing and toggles
variant_spacing = 640;
show_build_volumes = true;
show_kinematic_guides = true;
show_labels = true;

module xbeam(length, section_size, x0, y0, z0) {
    translate([x0-length/2, y0-section_size/2, z0-section_size/2])
        cube([length, section_size, section_size]);
}

module ybeam(length, section_size, x0, y0, z0) {
    translate([x0-section_size/2, y0-length/2, z0-section_size/2])
        cube([section_size, length, section_size]);
}

module post(section_size, height, x0, y0, z0) {
    translate([x0-section_size/2, y0-section_size/2, z0])
        cube([section_size, section_size, height]);
}

module frame(offset_x=0) {
    color([0.28, 0.31, 0.35]) {
        // Lower and upper perimeter rings
        for (z0 = [section/2, frame_z-section/2]) {
            for (y0 = [-frame_y/2+section/2, frame_y/2-section/2])
                xbeam(frame_x, section, offset_x, y0, z0);
            for (x0 = [-frame_x/2+section/2, frame_x/2-section/2])
                ybeam(frame_y, section, offset_x+x0, 0, z0);
        }
        // Corner uprights
        for (x0 = [-frame_x/2+section/2, frame_x/2-section/2])
            for (y0 = [-frame_y/2+section/2, frame_y/2-section/2])
                post(section, frame_z-section, offset_x+x0, y0, section);
        // Base cross-members (concept only)
        xbeam(frame_x-2*section, section, offset_x, 0, 45);
        ybeam(frame_y-2*section, section, offset_x, 0, 45);
    }
}

module build_volume(offset_x, bed_z) {
    if (show_build_volumes) {
        color([0.08, 0.45, 0.92, 0.14])
            translate([offset_x-build_x/2, -build_y/2, bed_z+bed_t])
                cube([build_x, build_y, build_z]);
    }
}

module label_at(txt, offset_x, z0) {
    if (show_labels) {
        color([0.08, 0.08, 0.08])
            translate([offset_x-200, -frame_y/2-35, z0])
                rotate([90,0,0])
                    linear_extrude(height=1)
                        text(txt, size=18, font="Liberation Sans:style=Bold");
    }
}

// A — fixed XY gantry, Z-moving bed. Three Z-support points are shown as a
// conceptual arrangement; exact support count and drive architecture are open.
module option_a(offset_x) {
    frame(offset_x);

    bed_z = 150;
    // Fixed XY gantry at upper working region
    color([0.18, 0.35, 0.52]) {
        xbeam(frame_x-80, 20, offset_x, 0, bed_z+build_z+38);
        ybeam(frame_y-100, 16, offset_x-160, 0, bed_z+build_z+24);
        ybeam(frame_y-100, 16, offset_x+160, 0, bed_z+build_z+24);
        translate([offset_x-28, -28, bed_z+build_z+5]) cube([56,56,66]); // toolhead envelope
    }
    // Moving platform and representative three-point Z supports
    color([0.62, 0.65, 0.69])
        translate([offset_x-bed_x/2, -bed_y/2, bed_z])
            cube([bed_x, bed_y, bed_t]);
    if (show_kinematic_guides) {
        color([0.65, 0.43, 0.18]) {
            for (p = [[-155,-155],[155,-155],[0,155]]) {
                translate([offset_x+p[0], p[1], 55])
                    cylinder(h=bed_z-55, d=10);
            }
        }
    }
    build_volume(offset_x, bed_z);
    label_at("A  /  MOVING Z BED", offset_x, frame_z+32);
}

// B — fixed bed, Z-moving XY gantry. Four conceptual lift columns are shown;
// racking, synchronization, tramming and moving gantry mass require detailed study.
module option_b(offset_x) {
    frame(offset_x);

    bed_z = 155;
    // Fixed build platform
    color([0.62, 0.65, 0.69])
        translate([offset_x-bed_x/2, -bed_y/2, bed_z])
            cube([bed_x, bed_y, bed_t]);
    // XY gantry at a representative mid-height position
    gantry_z = bed_z+170;
    color([0.18, 0.35, 0.52]) {
        xbeam(frame_x-80, 20, offset_x, 0, gantry_z);
        ybeam(frame_y-100, 16, offset_x-160, 0, gantry_z-14);
        ybeam(frame_y-100, 16, offset_x+160, 0, gantry_z-14);
        translate([offset_x-28, -28, gantry_z-40]) cube([56,56,66]); // toolhead envelope
    }
    if (show_kinematic_guides) {
        color([0.65, 0.43, 0.18]) {
            // Four lift/guide columns placed near frame corners.
            for (x0 = [-205,205])
                for (y0 = [-205,205])
                    translate([offset_x+x0, y0, bed_z+20])
                        cylinder(h=frame_z-bed_z-55, d=10);
        }
        // Side linear guide placeholders
        color([0.40,0.42,0.46])
            for (x0 = [-225,225])
                translate([offset_x+x0-6, -185, bed_z+20])
                    cube([12,370,frame_z-bed_z-50]);
    }
    build_volume(offset_x, bed_z);
    label_at("B  /  Z-MOVING GANTRY", offset_x, frame_z+32);
}

// Render both architecture candidates side-by-side using identical envelope assumptions.
option_a(-variant_spacing/2);
option_b(variant_spacing/2);
