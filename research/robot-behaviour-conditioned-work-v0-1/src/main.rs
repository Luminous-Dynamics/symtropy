use robot_behaviour_conditioned_work::{evaluate, MissionSnapshot};

fn main() {
    let result = evaluate(&MissionSnapshot::default());
    println!("suite=ROBOT-BEHAVIOUR-CONDITIONED-WORK-001");
    println!("status={:?}", result.status);
    println!("ready_packages={}", result.ready_package_ids().len());
    for package_id in result.ready_package_ids() {
        println!("ready={package_id}");
    }
    println!("note=read-only evaluator; no robot command or work acceptance emitted");
}
