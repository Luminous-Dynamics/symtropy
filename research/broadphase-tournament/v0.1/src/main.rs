use broadphase_tournament::{
    SceneFamily, TournamentConfig, TournamentRow, generate_scene, run_tournament,
};

fn run_dimension<const D: usize>(proxy_count: usize, frame_count: usize, repetitions: u32) {
    let config = TournamentConfig::new(repetitions);
    for family in SceneFamily::ALL {
        let scene = generate_scene::<D>(family, proxy_count, frame_count, 0x2026_1126_0000_0001)
            .expect("generate deterministic scene");
        let rows = run_tournament(&scene, config).expect("run architecture tournament");
        for row in rows {
            println!("{}", row.to_csv());
        }
    }
}

fn parse_arg<T: std::str::FromStr>(args: &[String], index: usize, default: T) -> T {
    args.get(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let proxy_count = parse_arg(&args, 1, 128_usize);
    let frame_count = parse_arg(&args, 2, 40_usize);
    let repetitions = parse_arg(&args, 3, 2_u32);

    println!("{}", TournamentRow::csv_header());
    run_dimension::<2>(proxy_count, frame_count, repetitions);
    run_dimension::<3>(proxy_count, frame_count, repetitions);
    run_dimension::<4>(proxy_count, frame_count, repetitions);
}
