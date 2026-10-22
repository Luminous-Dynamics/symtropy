use symthaea_core::genesis::GenesisSeed;
use symthaea_multirotor::{
    FlightConfig, FlightController, FlightState, PhysicsSimulator, QuadrotorCommand,
    QuadrotorHdcEncoder,
};
use symtropy_real_symthaea_physics_loop::symtropy_flight::SymtropyFlightSimulator;

#[test]
fn real_symthaea_flight_controller_steps_symtropy_physics() {
    let config = FlightConfig {
        network_layers: 1,
        neurons_per_layer: 4,
        genesis_phrase: "symthaea-in-symtropy-flight-smoke-v0.1".to_owned(),
        ..FlightConfig::default()
    };
    let genesis = GenesisSeed::from_phrase(&config.genesis_phrase);
    let mut encoder = QuadrotorHdcEncoder::new(&genesis, config.num_levels);
    let mut controller = FlightController::new(&genesis, &config);
    let mut sim = SymtropyFlightSimulator::new();

    for _ in 0..16 {
        let state = sim.state().clone();
        let observation = encoder.encode(&state);
        let command = controller.forward(&observation, config.motor_dt() as f32).clamped();
        assert!(command.to_ctrl().iter().all(|value| value.is_finite()));
        sim.step(&command, config.motor_dt());
        assert!(sim.state().is_finite());
    }

    assert!(sim.elapsed_s() > 0.0);
    assert_eq!(sim.invalid_input_count(), 0);
}

#[test]
fn backend_resets_to_declared_state_and_clears_disturbance() {
    let mut sim = SymtropyFlightSimulator::new();
    sim.apply_external_force([0.002, -0.001, 0.0]);
    sim.reset_to_state(&FlightState::hover(0.7));
    assert!((sim.state().position[2] - 0.7).abs() < 1.0e-12);
    let x = sim.state().position[0];
    sim.step(&QuadrotorCommand::hover(), 0.01);
    assert!((sim.state().position[0] - x).abs() < 1.0e-6);
    assert_eq!(sim.invalid_input_count(), 0);
}
