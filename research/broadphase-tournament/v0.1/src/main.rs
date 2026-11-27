use broadphase_tournament::{
    SceneFamily, TournamentConfig, TournamentRow, generate_scene, run_scale_tournament,
    run_tournament,
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
    if args.get(1).is_some_and(|mode| mode == "--scale") {
        let proxy_count = parse_arg(&args, 2, 10_000_usize);
        let frame_count = parse_arg(&args, 3, 32_usize);
        let repetitions = parse_arg(&args, 4, 1_u32);
        let config = TournamentConfig::new(repetitions);
        println!("{}", TournamentRow::csv_header());
        for family in [
            SceneFamily::SparseCoherent,
            SceneFamily::SleepingWakeups,
            SceneFamily::HighChurn,
        ] {
            let scene =
                generate_scene::<3>(family, proxy_count, frame_count, 0x2026_1126_0000_0001)
                    .expect("generate deterministic scale scene");
            let rows = run_scale_tournament(&scene, config)
                .expect("run exact-sweep-oracled scale tournament");
            for row in rows {
                println!("{}", row.to_csv());
            }
        }
        return;
    }

    let proxy_count = parse_arg(&args, 1, 128_usize);
    let frame_count = parse_arg(&args, 2, 40_usize);
    let repetitions = parse_arg(&args, 3, 2_u32);

    println!("{}", TournamentRow::csv_header());
    run_dimension::<2>(proxy_count, frame_count, repetitions);
    run_dimension::<3>(proxy_count, frame_count, repetitions);
    run_dimension::<4>(proxy_count, frame_count, repetitions);
}
