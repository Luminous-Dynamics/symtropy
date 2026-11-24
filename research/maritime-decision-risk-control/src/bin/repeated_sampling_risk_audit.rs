use maritime_decision_risk_control::repeated_sampling::{render_full_csv, render_smoke_csv};

fn main() {
    let full = std::env::args().skip(1).any(|argument| argument == "--full");
    let output = if full {
        render_full_csv()
    } else {
        render_smoke_csv()
    };
    print!("{output}");
}
