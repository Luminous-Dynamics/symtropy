//! End-to-end smoke test: current Symthaea observation/cognition/output loop
//! running against Symtropy's actual PhysicsWorld.
//!
//! The actuator and observed body intentionally share one physics handle. This
//! is a single-body torsional probe, not a calibrated articulated robot model.
//! It proves that the real cognitive output crosses the actuator boundary,
//! changes Symtropy physics state, and that changed state is fed back as the
//! next observation. Task performance is evaluated in later benchmark phases.

use bevy::prelude::*;
use symthaea_bevy_brain::{CognitiveBrain, SymthaeaBrainPlugin};
use symtropy_bevy_core::{BevyPhysics, BevyPhysicsCorePlugin, PhysicsBody};
use symtropy_math::Point;
use symtropy_physics::body::BodyHandle;
use symtropy_robotics_bridge::agent::{RoboticJoint, spawn_robot};
use symtropy_robotics_bridge::RoboticBrainPlugin;

const DT: f64 = 1.0 / 60.0;
const HV_DIM: usize = 16_384;

#[derive(Resource, Clone, Copy)]
struct ProbeEntities {
    root: Entity,
    actuator: Entity,
    body: BodyHandle,
}

fn setup_probe(mut commands: Commands, mut physics: ResMut<BevyPhysics<3>>) {
    let body = physics
        .world
        .add_sphere(Point::new([0.0, 0.0, 0.0]), 0.4, 1.0);

    let mut brain = CognitiveBrain::new_with_hv_input(
        64,
        "symthaea-symtropy-real-closed-loop-smoke-v0.1",
        HV_DIM,
    );
    brain.cycle_interval = 1;

    let root = spawn_robot(
        &mut commands,
        "symthaea-real-loop-probe",
        Vec3::ZERO,
        brain,
        body,
    );
    let actuator = commands
        .spawn((
            RoboticJoint {
                joint_name: "single-body-torsion-probe".to_owned(),
                motor_index: 0,
            },
            PhysicsBody::new(body, 0.4),
            Transform::default(),
            GlobalTransform::default(),
            ChildOf(root),
        ))
        .id();

    commands.insert_resource(ProbeEntities {
        root,
        actuator,
        body,
    });
}

#[test]
fn real_symthaea_runs_a_sensor_cognition_actuation_physics_loop() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(BevyPhysicsCorePlugin::<3>::default())
        .add_plugins(SymthaeaBrainPlugin::default())
        .add_plugins(RoboticBrainPlugin)
        .add_systems(Startup, setup_probe);
    app.update();

    let entities = *app.world().resource::<ProbeEntities>();
    let mut initial_output: Option<Vec<f32>> = None;
    let mut initial_perception: Option<Vec<f32>> = None;
    let mut peak_abs_motor_output = 0.0_f32;
    let mut peak_abs_angular_velocity = 0.0_f64;

    for tick in 0..16 {
        app.world_mut().run_schedule(FixedUpdate);

        let (output, perception) = {
            let brain = app
                .world()
                .get::<CognitiveBrain>(entities.root)
                .expect("spawned root keeps its CognitiveBrain");
            assert!(
                brain.last_result.is_some(),
                "the cognitive-cycle system did not execute on tick {tick}"
            );
            assert!(
                !brain.motor_output.is_empty(),
                "real Symthaea emitted no motor channels on tick {tick}"
            );
            assert!(
                brain.motor_output.iter().all(|value| value.is_finite()),
                "motor output contains a non-finite value on tick {tick}"
            );
            let hv = brain
                .perception_hv
                .as_ref()
                .expect("robot sensor system feeds an HDC observation");
            assert_eq!(hv.values.len(), HV_DIM);
            assert!(hv.values.iter().all(|value| value.is_finite()));
            (brain.motor_output.clone(), hv.values.clone())
        };

        peak_abs_motor_output = peak_abs_motor_output.max(
            output
                .iter()
                .map(|value| value.abs())
                .fold(0.0_f32, f32::max),
        );

        if tick == 0 {
            let (index, largest) = output
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
                .expect("at least one motor output");
            assert!(
                largest.abs() > 1.0e-8,
                "real Symthaea motor output is zero-only; actuator coupling was not demonstrated"
            );
            initial_output = Some(output);
            initial_perception = Some(perception);
            app.world_mut()
                .get_mut::<RoboticJoint>(entities.actuator)
                .expect("actuator component exists")
                .motor_index = index;
        }

        // This manual step is the Symtropy engine step, not a Symthaea-owned
        // platform simulator. The next tick senses angular velocity from it.
        app.world_mut()
            .resource_mut::<BevyPhysics<3>>()
            .world
            .step(DT);

        let omega = app
            .world()
            .resource::<BevyPhysics<3>>()
            .world
            .body(entities.body)
            .expect("probe body remains in Symtropy")
            .angular_velocity
            .get(0, 1);
        assert!(omega.is_finite(), "physics produced non-finite angular velocity");
        peak_abs_angular_velocity = peak_abs_angular_velocity.max(omega.abs());
    }

    let final_brain = app
        .world()
        .get::<CognitiveBrain>(entities.root)
        .expect("root brain remains alive");
    let final_perception = &final_brain.perception_hv.as_ref().expect("final perception exists").values;
    assert!(
        initial_perception.as_ref().is_some_and(|initial| initial != final_perception),
        "Symtropy physics state did not feed back into the next Symthaea observation"
    );
    assert!(
        peak_abs_angular_velocity > 1.0e-12,
        "nonzero motor output did not alter the Symtropy physics state"
    );

    let output_changed = initial_output
        .as_ref()
        .is_some_and(|initial| initial != &final_brain.motor_output);
    eprintln!(
        "REAL_SYMTHAEA_SYMTROPY_LOOP ticks=16 peak_abs_motor_output={peak_abs_motor_output:.8} peak_abs_omega_xy={peak_abs_angular_velocity:.8} output_changed={output_changed}"
    );
}
