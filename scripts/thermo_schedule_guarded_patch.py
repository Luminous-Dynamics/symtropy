from pathlib import Path
import subprocess

expected = {
    "src/systems/thermodynamic.rs": "68dc7f7f85cee9c1b9700233ddaa9bc3af46131d",
    "src/plugin.rs": "11fc8c722157d66ff94be26985019213a9ebd22b",
    "src/systems/thermodynamic_schedule.rs": "53a1df174c3bd5b81e086297a04b38bda0c65743",
}
for path, want in expected.items():
    got = subprocess.check_output(["git", "hash-object", path], text=True).strip()
    if got != want:
        raise SystemExit(f"refusing patch: {path} blob {got} != expected {want}")

def replace_once(text, old, new, label):
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"refusing patch: {label} occurs {count} times")
    return text.replace(old, new, 1)

# Expose only the already-source-audited operational begin helpers.
thermo = Path("src/systems/thermodynamic.rs")
text = thermo.read_text()
for old, new in [
    ("fn regenerate_live_entity_accepted(\n", "pub(crate) fn regenerate_live_entity_accepted(\n"),
    ("fn transfer_from_finite_source(\n", "pub(crate) fn transfer_from_finite_source(\n"),
    ("fn epistemic_offload_factors(\n", "pub(crate) fn epistemic_offload_factors(\n"),
    ("fn maintenance_cost_with_offload(\n", "pub(crate) fn maintenance_cost_with_offload(\n"),
]:
    text = replace_once(text, old, new, f"thermodynamic target {old!r}")
thermo.write_text(text)

schedule = Path("src/systems/thermodynamic_schedule.rs")
text = schedule.read_text()
text = replace_once(
    text,
    "pub enum ThermodynamicScheduleFaultKind {\n    BeginRejected,\n",
    "pub enum ThermodynamicScheduleFaultKind {\n    BeginRejected,\n    BeginCensus,\n    CensusDriftBeforeConsequence,\n",
    "schedule fault enum",
)

anchor = """pub struct ThermodynamicScheduleFault {\n    pub tick_id: Option<u64>,\n    pub kind: ThermodynamicScheduleFaultKind,\n}\n\n"""
insert = """pub struct ThermodynamicScheduleFault {\n    pub tick_id: Option<u64>,\n    pub kind: ThermodynamicScheduleFaultKind,\n}\n\n#[derive(Copy, Clone, Debug, PartialEq, Eq)]\nenum BoundConsequenceMode {\n    Dynamic2d,\n    Kinematic3d,\n}\n\nimpl BoundConsequenceMode {\n    fn from_phase(phase: GamePhase) -> Option<Self> {\n        match phase {\n            GamePhase::Playing => Some(Self::Dynamic2d),\n            GamePhase::Playing3D => Some(Self::Kinematic3d),\n            _ => None,\n        }\n    }\n}\n\n"""
text = replace_once(text, anchor, insert, "bound consequence mode")
text = replace_once(
    text,
    "pub struct ThermodynamicScheduleState {\n    pending: Option<PendingThermodynamicTick>,\n",
    "pub struct ThermodynamicScheduleState {\n    pending: Option<PendingThermodynamicTick>,\n    active_handles: Option<Vec<BodyHandle>>,\n    active_mode: Option<BoundConsequenceMode>,\n",
    "schedule active control-volume fields",
)
text = replace_once(
    text,
    """    pub fn has_pending_transaction(&self) -> bool {\n        self.pending.is_some()\n    }\n\n    pub fn last_receipt""",
    """    pub fn has_pending_transaction(&self) -> bool {\n        self.pending.is_some()\n    }\n\n    pub fn active_handles(&self) -> Option<&[BodyHandle]> {\n        self.active_handles.as_deref()\n    }\n\n    pub fn last_receipt""",
    "schedule active census accessor",
)
text = replace_once(
    text,
    """    fn commit_receipt(&mut self, receipt: CadenceBoundThermodynamicTickReceipt) {\n        self.pending = None;\n        self.last_receipt = Some(receipt);\n    }\n""",
    """    fn commit_receipt(&mut self, receipt: CadenceBoundThermodynamicTickReceipt) {\n        self.pending = None;\n        self.active_handles = None;\n        self.active_mode = None;\n        self.last_receipt = Some(receipt);\n    }\n""",
    "schedule receipt control-volume rotation",
)

begin_old = '''pub fn transactional_thermodynamic_begin_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    mut physics: ResMut<PhysicsWorldRes>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    // A delayed close owns the previous tick. Never reset counters underneath it.
    if schedule.pending.is_some() || runtime.open_tick_id().is_some() {
        return;
    }

    if let Err(error) =
        begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &fixed_time)
    {
        warn!("thermodynamic fixed-tick begin rejected: {error:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginRejected);
        return;
    }

    let agent_data: Vec<_> = entities_query
        .iter()
        .map(|(body, transform)| (body.handle, transform.translation))
        .collect();
    apply_operational_begin_flows(&mut physics, &agent_data, &mut wells);
}
'''
begin_new = '''pub fn transactional_thermodynamic_begin_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    phase: Res<State<GamePhase>>,
    mut physics: ResMut<PhysicsWorldRes>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    if schedule.pending.is_some() || runtime.open_tick_id().is_some() {
        return;
    }
    let Some(bound_mode) = BoundConsequenceMode::from_phase(*phase.get()) else {
        return;
    };

    let mut agent_data: Vec<_> = entities_query
        .iter()
        .map(|(body, transform)| (body.handle, transform.translation))
        .collect();
    agent_data.sort_by_key(|(handle, _)| *handle);
    if let Some(pair) = agent_data.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        warn!("thermodynamic begin rejected duplicate body handle: {:?}", pair[0].0);
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginCensus);
        return;
    }
    if let Some((handle, _)) = agent_data
        .iter()
        .find(|(handle, _)| !physics.consciousness.entities.contains_key(handle))
    {
        warn!("thermodynamic begin rejected missing operational entity: {handle:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginCensus);
        return;
    }

    if let Err(error) = begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &fixed_time) {
        warn!("thermodynamic fixed-tick begin rejected: {error:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginRejected);
        return;
    }

    schedule.active_handles = Some(agent_data.iter().map(|(handle, _)| *handle).collect());
    schedule.active_mode = Some(bound_mode);
    apply_operational_begin_flows(&mut physics, &agent_data, &mut wells);
}
'''
text = replace_once(text, begin_old, begin_new, "transactional begin function")

consequence_old = '''fn consequence_for_phase(
    phase: GamePhase,
    physics: &mut PhysicsWorldRes,
    fixed_time: &Time<Fixed>,
) -> Option<(ThermodynamicConsequenceStatus, bool)> {
    match phase {
        GamePhase::Playing => {
            let succeeded = step_physics_world(physics, fixed_time.delta().as_secs_f64());
            Some((
                if succeeded {
                    ThermodynamicConsequenceStatus::Executed
                } else {
                    ThermodynamicConsequenceStatus::Rejected
                },
                succeeded,
            ))
        }
        GamePhase::Playing3D => Some((
            ThermodynamicConsequenceStatus::IntentionallyAbsent,
            false,
        )),
        _ => None,
    }
}
'''
consequence_new = '''fn consequence_for_bound_mode(
    mode: BoundConsequenceMode,
    physics: &mut PhysicsWorldRes,
    fixed_time: &Time<Fixed>,
) -> (ThermodynamicConsequenceStatus, bool) {
    match mode {
        BoundConsequenceMode::Dynamic2d => {
            let succeeded = step_physics_world(physics, fixed_time.delta().as_secs_f64());
            (
                if succeeded {
                    ThermodynamicConsequenceStatus::Executed
                } else {
                    ThermodynamicConsequenceStatus::Rejected
                },
                succeeded,
            )
        }
        BoundConsequenceMode::Kinematic3d => (
            ThermodynamicConsequenceStatus::IntentionallyAbsent,
            false,
        ),
    }
}
'''
text = replace_once(text, consequence_old, consequence_new, "bound consequence function")

signature_old = '''    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    phase: Res<State<GamePhase>>,
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud: ResMut<ThermodynamicHudState>,
    agent_query: Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    mut transform_query: Query<(&PhysicsBody, &mut Transform)>,
) {
    let handles: Vec<_> = agent_query.iter().map(|body| body.handle).collect();
'''
signature_new = '''    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud: ResMut<ThermodynamicHudState>,
    agent_query: Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    mut transform_query: Query<(&PhysicsBody, &mut Transform)>,
) {
    let handles = match schedule.active_handles.clone() {
        Some(handles) => handles,
        None => {
            let tick_id = runtime.open_tick_id();
            if tick_id.is_some() || schedule.pending.is_some() {
                error!("thermodynamic schedule lost its begin census for an active tick");
                schedule.note_fault(tick_id, ThermodynamicScheduleFaultKind::RetryInvariant);
                schedule.pending = Some(PendingThermodynamicTick::Poisoned);
            }
            return;
        }
    };
    let mode = match schedule.active_mode {
        Some(mode) => mode,
        None => {
            let tick_id = runtime.open_tick_id();
            error!("thermodynamic schedule lost its bound consequence mode");
            schedule.note_fault(tick_id, ThermodynamicScheduleFaultKind::RetryInvariant);
            schedule.pending = Some(PendingThermodynamicTick::Poisoned);
            return;
        }
    };

    let live_census_matches = || {
        let mut live: Vec<_> = agent_query.iter().map(|body| body.handle).collect();
        live.sort_unstable();
        let duplicate = live.windows(2).any(|pair| pair[0] == pair[1]);
        !duplicate && live == handles
    };
'''
text = replace_once(text, signature_old, signature_new, "finalizer frozen control volume")

old_consequence_call = '''            let Some((status, export)) =
                consequence_for_phase(*phase.get(), &mut physics, &fixed_time)
            else {
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            };
'''
new_consequence_call = '''            if !live_census_matches() {
                schedule.note_fault(
                    runtime.open_tick_id(),
                    ThermodynamicScheduleFaultKind::CensusDriftBeforeConsequence,
                );
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            }
            let (status, export) = consequence_for_bound_mode(mode, &mut physics, &fixed_time);
'''
if text.count(old_consequence_call) != 2:
    raise SystemExit(f"refusing patch: consequence call occurs {text.count(old_consequence_call)} times")
text = text.replace(old_consequence_call, new_consequence_call)
schedule.write_text(text)

plugin = Path("src/plugin.rs")
text = plugin.read_text()
text = replace_once(
    text,
    "            .init_resource::<systems::thermodynamic::ThermodynamicHudState>()\n",
    "            .init_resource::<systems::thermodynamic::ThermodynamicHudState>()\n            .init_resource::<systems::thermodynamic_runtime::ThermodynamicTransactionRuntime>()\n            .init_resource::<systems::thermodynamic_schedule::ThermodynamicScheduleState>()\n",
    "HUD resource anchor",
)
old_schedule = '''            .add_systems(FixedUpdate, (
                systems::thermodynamic::thermodynamic_enforcement_system,
                systems::engine_physics::physics_sync_transforms,
            ).chain().run_if(in_playing_or_3d))
'''
new_schedule = '''            .add_systems(
                FixedUpdate,
                systems::thermodynamic_schedule::transactional_thermodynamic_begin_system
                    .run_if(in_playing_or_3d),
            )
            .add_systems(
                FixedUpdate,
                systems::thermodynamic_schedule::transactional_physics_finalize_system
                    .after(systems::thermodynamic_schedule::transactional_thermodynamic_begin_system),
            )
'''
text = replace_once(text, old_schedule, new_schedule, "FixedUpdate schedule anchor")
plugin.write_text(text)
