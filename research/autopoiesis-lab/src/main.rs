use symtropy_autopoiesis_lab::{AutopoieticNetwork, Component, InitialState};

fn main() {
    let mut network = AutopoieticNetwork::new(InitialState::default(), 10_000)
        .expect("valid research fixture");

    println!("toy catalytic network — not a biological-life or consciousness claim");
    println!("initial_viable={}", network.is_viable());

    network.run_steps(32).expect("baseline should run");
    network.knockout(Component::CatalystA).expect("knockout should be accounted");
    println!("after_catalyst_a_knockout={:.6}", network.component(Component::CatalystA));

    network.run_steps(600).expect("recovery window should run");
    println!("after_recovery_a={:.6}", network.component(Component::CatalystA));
    println!("catalyst_b={:.6}", network.component(Component::CatalystB));
    println!("membrane={:.6}", network.component(Component::Membrane));
    println!("resource_residual={:.12e}", network.resource_residual());
    println!("viable={}", network.is_viable());
}
