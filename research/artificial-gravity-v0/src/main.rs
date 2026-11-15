use symtropy_artificial_gravity::radius_for_gravity_m;

fn main() {
    let gravity_levels = [0.16_f64, 0.38, 1.0];
    let rotation_rates = [1.0_f64, 2.0, 3.5, 4.0, 6.0];

    println!("target_g,rpm,required_radius_m");
    for target_g in gravity_levels {
        for rpm in rotation_rates {
            match radius_for_gravity_m(target_g, rpm) {
                Ok(radius) => println!("{target_g:.2},{rpm:.1},{radius:.3}"),
                Err(error) => {
                    eprintln!("invalid design point ({target_g} g, {rpm} rpm): {error}");
                    std::process::exit(1);
                }
            }
        }
    }
}
