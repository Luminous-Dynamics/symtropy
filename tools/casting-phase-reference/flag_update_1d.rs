//! Independent Rust reimplementation of the 1D isothermal flag-update enthalpy method.
//!
//! Algorithm source: Peters et al. (2025), IJHMT 249, 127166, Sections 4.1-4.4.
//! This is NOT a port of the authors' MATLAB supplementary code.
//!
//! The 1D sparse system is reduced to contiguous tridiagonal segments:
//! mixed nodes impose v=0 and recover latent enthalpy from the PDE row;
//! solid/liquid nodes solve for Kirchhoff potential v with fixed latent S.
//!
//! Compile directly with rustc; this tool is outside the Bevy workspace.

use std::collections::HashSet;

const RHO: f64 = 1.0;
const CP_S: f64 = 2.5e6;
const CP_L: f64 = 2.5e6;
const K_S: f64 = 2.0;
const K_L: f64 = 2.0;
const LATENT: f64 = 1.0e8;
const TW: f64 = -10.0;
const TM: f64 = 0.0;
const TINF: f64 = 2.0;
const DOMAIN: f64 = 6.0;
const END_TIME: f64 = 288.0 * 3600.0;
const LAMBDA: f64 = 0.322_390_632_922_300_86;
const SQRT_PI: f64 = 1.772_453_850_905_516_027_3;
const MAX_FLAG_ITERATIONS: usize = 64;
const MAX_STEPS: usize = 10_000;
const SOLVE_RESIDUAL_TOL: f64 = 5.0e-10;
const ERF_MAX: f64 = 1.25;

#[derive(Clone, Copy, Debug)]
struct Campaign {
    intervals: usize,
    time_steps: usize,
    domain_m: f64,
    end_time_s: f64,
}

impl Campaign {
    fn published(intervals: usize, time_steps: usize) -> Self {
        Self { intervals, time_steps, domain_m: DOMAIN, end_time_s: END_TIME }
    }

    fn validate(self) -> Result<ValidatedCampaign, String> {
        if self.intervals < 8 || self.time_steps == 0 || self.time_steps > MAX_STEPS
            || !self.domain_m.is_finite() || self.domain_m <= 0.0
            || !self.end_time_s.is_finite() || self.end_time_s <= 0.0
        {
            return Err("invalid intervals, timesteps, domain or end time".into());
        }
        let grid_points = self.intervals + 1;
        let dx = self.domain_m / self.intervals as f64;
        let dt = self.end_time_s / self.time_steps as f64;
        let alpha_s = K_S / (RHO * CP_S);
        let alpha_l = K_L / (RHO * CP_L);
        if !dx.is_finite() || dx <= 0.0 || !dt.is_finite() || dt <= 0.0 {
            return Err("invalid dx or dt".into());
        }
        if !alpha_s.is_finite() || !alpha_l.is_finite() || alpha_s <= 0.0 || alpha_l <= 0.0 {
            return Err("invalid phase diffusivity".into());
        }
        Ok(ValidatedCampaign {
            grid_points,
            dx,
            dt,
            alpha_s,
            alpha_l,
            latent_density: RHO * LATENT,
            wall_potential: K_S * TW,
        })
    }
}

struct ValidatedCampaign {
    grid_points: usize,
    dx: f64,
    dt: f64,
    alpha_s: f64,
    alpha_l: f64,
    latent_density: f64,
    wall_potential: f64,
}

#[derive(Clone, Debug)]
struct StepSolution {
    potential: Vec<f64>,
    latent: Vec<f64>,
    flags: Vec<i8>,
    iterations: usize,
    tridiagonal_segments: usize,
    max_pde_residual_relative: f64,
}

#[derive(Debug)]
struct Summary {
    intervals: usize,
    grid_points: usize,
    time_steps: usize,
    dx_m: f64,
    dt_s: f64,
    final_front_exact_m: f64,
    interface_error_mean_abs_m: f64,
    interface_final_error_m: f64,
    temperature_mean_abs_error_c: f64,
    temperature_rms_error_c: f64,
    mean_flag_iterations: f64,
    max_flag_iterations: usize,
    total_tridiagonal_segments: usize,
    max_pde_residual_relative: f64,
    wall_heat_j_m2: f64,
    enthalpy_drop_j_m2: f64,
    energy_residual_relative: f64,
    final_interface_m: f64,
}

fn erf(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x.abs() > ERF_MAX {
        return Err("erf argument outside its admitted [-1.25,1.25] range".into());
    }
    if x == 0.0 { return Ok(0.0); }
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    let mut compensation = 0.0;
    for n in 1..=160 {
        term *= -x2 / n as f64 * (2 * n - 1) as f64 / (2 * n + 1) as f64;
        let adjusted = term - compensation;
        let next = sum + adjusted;
        compensation = (next - sum) - adjusted;
        sum = next;
        if term.abs() <= 0.2 * f64::EPSILON * sum.abs() { break; }
    }
    Ok(2.0 * sum / SQRT_PI)
}

fn erfc(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x < 0.0 { return Err("erfc argument must be finite and nonnegative".into()); }
    if x <= ERF_MAX { return Ok(1.0 - erf(x)?); }
    let xx = x * x;
    if !xx.is_finite() { return Ok(0.0); }
    let a = 0.5;
    let mut b = xx + 0.5;
    let mut c = 1.0e300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=256 {
        let fi = i as f64;
        let an = -fi * (fi - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1.0e-300 { d = 1.0e-300; }
        c = b + an / c;
        if c.abs() < 1.0e-300 { c = 1.0e-300; }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= 3.0e-15 { break; }
        if i == 256 { return Err("erfc continued fraction did not converge".into()); }
    }
    let log_gamma_half = 0.572_364_942_924_700_087_1;
    Ok((-xx + a * xx.ln() - log_gamma_half).exp() * h)
}

fn exact_front(time_s: f64) -> f64 {
    2.0 * LAMBDA * (8.0e-7 * time_s).sqrt()
}

fn exact_temperature(x: f64, time_s: f64) -> Result<f64, String> {
    if !x.is_finite() || x < 0.0 || !time_s.is_finite() || time_s <= 0.0 {
        return Err("invalid exact field query".into());
    }
    let a = 2.0 * (8.0e-7 * time_s).sqrt();
    let eta = x / a;
    if eta <= LAMBDA {
        Ok(TW + (TM - TW) * erf(eta)? / erf(LAMBDA)?)
    } else {
        Ok(TINF + (TM - TINF) * erfc(eta)? / erfc(LAMBDA)?)
    }
}

#[inline]
fn gamma_for_flag(flag: i8, alpha_s: f64, alpha_l: f64) -> Result<f64, String> {
    match flag {
        -1 => Ok(1.0 / alpha_s),
        0 => Ok(0.0),
        1 => Ok(1.0 / alpha_l),
        _ => Err("invalid phase flag".into()),
    }
}

#[inline]
fn fixed_latent_for_flag(flag: i8, latent_density: f64) -> Result<Option<f64>, String> {
    match flag {
        -1 => Ok(Some(0.0)),
        0 => Ok(None), // mixed-phase latent component is the PDE unknown
        1 => Ok(Some(latent_density)),
        _ => Err("invalid phase flag".into()),
    }
}

fn thomas_solve(lower: &[f64], diagonal: &[f64], upper: &[f64], rhs: &[f64]) -> Result<Vec<f64>, String> {
    let n = diagonal.len();
    if n == 0 || lower.len() != n || upper.len() != n || rhs.len() != n {
        return Err("invalid tridiagonal system dimensions".into());
    }
    let c = upper.to_vec();
    let mut d = rhs.to_vec();
    let mut b = diagonal.to_vec();
    let mut scale = 1.0_f64;
    for &v in &b { scale = scale.max(v.abs()); }
    for i in 1..n {
        if !b[i - 1].is_finite() || b[i - 1].abs() <= 64.0 * f64::EPSILON * scale {
            return Err("tridiagonal pivot is singular or numerically unsafe".into());
        }
        let m = lower[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        d[i] -= m * d[i - 1];
    }
    if !b[n - 1].is_finite() || b[n - 1].abs() <= 64.0 * f64::EPSILON * scale {
        return Err("final tridiagonal pivot is singular or numerically unsafe".into());
    }
    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = (d[i] - c[i] * x[i + 1]) / b[i];
    }
    if x.iter().any(|v| !v.is_finite()) { return Err("tridiagonal solve produced non-finite values".into()); }
    Ok(x)
}

/// Solve the linear block system for a fixed flag vector.
/// Mixed nodes have v=0; all other nodes form independent tridiagonal segments.
/// The last node uses the paper's Neumann ghost-point coefficient -2r.
fn solve_fixed_flags(
    e_old: &[f64],
    flags: &[i8],
    dt: f64,
    dx: f64,
    alpha_s: f64,
    alpha_l: f64,
    latent_density: f64,
    wall_potential: f64,
) -> Result<(Vec<f64>, Vec<f64>, usize), String> {
    let nodes = flags.len();
    if e_old.len() != nodes || nodes < 2 || flags[0] != -1 {
        return Err("invalid flag-system state or missing fixed solid wall flag".into());
    }
    let intervals = nodes - 1;
    let r = dt / (dx * dx);
    if !r.is_finite() || r <= 0.0 { return Err("invalid implicit diffusion coefficient".into()); }

    let mut v = vec![0.0; nodes];
    let mut s = vec![0.0; nodes];
    v[0] = wall_potential;
    s[0] = 0.0;

    let mut segments = 0usize;
    let mut i = 1usize;
    while i <= intervals {
        if flags[i] == 0 {
            v[i] = 0.0;
            i += 1;
            continue;
        }

        let start = i;
        while i <= intervals && flags[i] != 0 {
            i += 1;
        }
        let end = i - 1;
        let len = end - start + 1;
        segments += 1;

        let mut lower = vec![0.0; len];
        let mut diagonal = vec![0.0; len];
        let mut upper = vec![0.0; len];
        let mut rhs = vec![0.0; len];

        for j in 0..len {
            let node = start + j;
            let gamma = gamma_for_flag(flags[node], alpha_s, alpha_l)?;
            let fixed_s = fixed_latent_for_flag(flags[node], latent_density)?
                .ok_or("mixed-phase node entered a non-mixed tridiagonal segment")?;
            diagonal[j] = gamma + 2.0 * r;
            rhs[j] = e_old[node] - fixed_s;

            if j == 0 {
                if node == 1 {
                    rhs[j] += r * wall_potential;
                } // otherwise the left neighbour is mixed and v=0.
            } else {
                lower[j] = if node == intervals { -2.0 * r } else { -r };
            }

            if j + 1 < len {
                upper[j] = -r;
            }
        }

        let values = thomas_solve(&lower, &diagonal, &upper, &rhs)?;
        for j in 0..len {
            v[start + j] = values[j];
        }
    }

    for node in 1..=intervals {
        match flags[node] {
            -1 => { s[node] = 0.0; }
            0 => {
                let lap_scaled = if node == intervals {
                    2.0 * r * (v[node - 1] - v[node])
                } else {
                    r * (v[node - 1] - 2.0 * v[node] + v[node + 1])
                };
                s[node] = e_old[node] + lap_scaled;
            }
            1 => { s[node] = latent_density; }
            _ => return Err("invalid flag in latent recovery".into()),
        }
    }
    Ok((v, s, segments))
}

fn update_flags(flags: &[i8], v: &[f64], s: &[f64], latent_density: f64) -> Result<Vec<i8>, String> {
    if flags.len() != v.len() || flags.len() != s.len() {
        return Err("flag update array length mismatch".into());
    }
    let mut next = flags.to_vec();
    next[0] = -1;
    for i in 1..flags.len() {
        let new_flag = match flags[i] {
            1 if v[i] <= 0.0 => 0,
            1 => 1,
            0 if s[i] <= 0.0 => -1,
            0 if s[i] >= latent_density => 1,
            0 => 0,
            -1 if v[i] >= 0.0 => 0,
            -1 => -1,
            _ => return Err("invalid phase flag during update".into()),
        };
        next[i] = new_flag;
    }
    Ok(next)
}

fn solve_one_step(
    e_old: &[f64],
    old_flags: &[i8],
    campaign: &ValidatedCampaign,
) -> Result<StepSolution, String> {
    let mut flags = old_flags.to_vec();
    let mut seen: HashSet<Vec<i8>> = HashSet::new();
    let mut total_segments = 0usize;

    for iteration in 1..=MAX_FLAG_ITERATIONS {
        if !seen.insert(flags.clone()) {
            return Err("FLAG_CYCLE: flag iteration revisited a previous state".into());
        }
        let (v, s, segments) = solve_fixed_flags(
            e_old,
            &flags,
            campaign.dt,
            campaign.dx,
            campaign.alpha_s,
            campaign.alpha_l,
            campaign.latent_density,
            campaign.wall_potential,
        )?;
        total_segments += segments;

        if v.iter().any(|x| !x.is_finite()) || s.iter().any(|x| !x.is_finite()) {
            return Err("non-finite solution of flag linear system".into());
        }
        let next = update_flags(&flags, &v, &s, campaign.latent_density)?;
        if next == flags {
            let mut e_new = vec![0.0; flags.len()];
            e_new[0] = RHO * CP_S * TW;
            let mut max_residual = 0.0_f64;
            let mut scale = 1.0_f64;
            for i in 1..flags.len() {
                let gamma = gamma_for_flag(flags[i], campaign.alpha_s, campaign.alpha_l)?;
                e_new[i] = s[i] + gamma * v[i];
                scale = scale.max(e_new[i].abs()).max(e_old[i].abs());
                let lap = if i + 1 == flags.len() {
                    2.0 * (v[i - 1] - v[i]) / (campaign.dx * campaign.dx)
                } else {
                    (v[i - 1] - 2.0 * v[i] + v[i + 1]) / (campaign.dx * campaign.dx)
                };
                let residual = e_new[i] - campaign.dt * lap - e_old[i];
                max_residual = max_residual.max(residual.abs());
            }
            let relative_residual = max_residual / scale;
            if !relative_residual.is_finite() || relative_residual > SOLVE_RESIDUAL_TOL {
                return Err(format!("flag block solution residual {relative_residual} exceeds tolerance"));
            }

            let mut temperatures = vec![0.0; flags.len()];
            for i in 0..flags.len() {
                temperatures[i] = match flags[i] {
                    -1 => v[i] / K_S,
                    0 => 0.0,
                    1 => v[i] / K_L,
                    _ => return Err("invalid terminal flag".into()),
                };
                if !temperatures[i].is_finite() {
                    return Err("non-finite temperature after flag solve".into());
                }
            }

            return Ok(StepSolution {
                potential: v,
                latent: s,
                flags,
                iterations: iteration,
                tridiagonal_segments: total_segments,
                max_pde_residual_relative: relative_residual,
            });
        }
        flags = next;
    }
    Err(format!("FLAG_NO_CONVERGENCE: exceeded {MAX_FLAG_ITERATIONS} consistency updates"))
}

fn latent_interface(latent: &[f64], dx: f64, latent_density: f64) -> Result<f64, String> {
    let threshold = 0.5 * latent_density;
    for i in 0..latent.len() - 1 {
        if latent[i] <= threshold && latent[i + 1] >= threshold {
            let delta = latent[i + 1] - latent[i];
            if delta <= 0.0 { return Err("latent interface is not strictly increasing".into()); }
            let fraction = ((threshold - latent[i]) / delta).clamp(0.0, 1.0);
            return Ok((i as f64 + fraction) * dx);
        }
    }
    Err("INTERFACE_NOT_FOUND: S=rho*L/2 not bracketed".into())
}

fn temperature_from_step(step: &StepSolution) -> Result<Vec<f64>, String> {
    let mut t = vec![0.0; step.flags.len()];
    for i in 0..t.len() {
        t[i] = match step.flags[i] {
            -1 => step.potential[i] / K_S,
            0 => 0.0,
            1 => step.potential[i] / K_L,
            _ => return Err("invalid flag in temperature conversion".into()),
        };
        if !t[i].is_finite() { return Err("non-finite temperature".into()); }
    }
    Ok(t)
}

fn trapezoid_total_enthalpy(e: &[f64], dx: f64) -> f64 {
    let n = e.len();
    let mut sum = 0.5 * e[0] + 0.5 * e[n - 1];
    for value in &e[1..n - 1] { sum += value; }
    sum * dx
}

fn run_campaign(campaign: Campaign) -> Result<Summary, String> {
    let v = campaign.validate()?;
    let nodes = v.grid_points;
    let intervals = campaign.intervals;
    let mut e = vec![0.0; nodes];
    let mut flags = vec![1_i8; nodes];
    let mut latent = vec![v.latent_density; nodes];
    let mut potential = vec![K_L * TINF; nodes];

    // Dirichlet wall node is solid with S=0, T=Tw, v=K_s*Tw.
    flags[0] = -1;
    latent[0] = 0.0;
    potential[0] = v.wall_potential;
    e[0] = RHO * CP_S * TW;
    for i in 1..nodes {
        e[i] = v.latent_density + RHO * CP_L * TINF;
    }

    let initial_total = trapezoid_total_enthalpy(&e, v.dx);
    let mut wall_heat = 0.0_f64;
    let mut interface_error_sum = 0.0_f64;
    let mut temperature_abs_sum = 0.0_f64;
    let mut temperature_square_sum = 0.0_f64;
    let mut total_flag_iterations = 0_usize;
    let mut max_flag_iterations = 0_usize;
    let mut total_segments = 0_usize;
    let mut max_pde_residual = 0.0_f64;

    for step_index in 0..campaign.time_steps {
        let solution = solve_one_step(&e, &flags, &v)?;
        let t_num = temperature_from_step(&solution)?;
        let interface = latent_interface(&solution.latent, v.dx, v.latent_density)?;
        let time = (step_index + 1) as f64 * v.dt;

        for i in 0..nodes {
            let x = i as f64 * v.dx;
            let t_exact = exact_temperature(x, time)?;
            let error = t_num[i] - t_exact;
            temperature_abs_sum += error.abs();
            temperature_square_sum += error * error;
        }
        interface_error_sum += (interface - exact_front(time)).abs();

        // Kirchhoff potential v=K*T. Outward heat removed at the cold wall
        // is -d(v)/dx at x=0, accumulated with backward/forward sign.
        let q_out_rate = (solution.potential[1] - solution.potential[0]) / v.dx;
        wall_heat += q_out_rate * v.dt;
        if !wall_heat.is_finite() || wall_heat < 0.0 {
            return Err("invalid wall heat accumulation".into());
        }

        total_flag_iterations += solution.iterations;
        max_flag_iterations = max_flag_iterations.max(solution.iterations);
        total_segments += solution.tridiagonal_segments;
        max_pde_residual = max_pde_residual.max(solution.max_pde_residual_relative);

        e = solution.latent.iter().zip(solution.potential.iter())
            .zip(solution.flags.iter())
            .map(|((s, pot), flag)| {
                let gamma = match *flag {
                    -1 => 1.0 / v.alpha_s,
                    0 => 0.0,
                    1 => 1.0 / v.alpha_l,
                    _ => f64::NAN,
                };
                s + gamma * pot
            })
            .collect();
        flags = solution.flags;
        latent = solution.latent;
        potential = solution.potential;
        if e.iter().any(|x| !x.is_finite()) {
            return Err("non-finite enthalpy after timestep".into());
        }
    }

    let final_total = trapezoid_total_enthalpy(&e, v.dx);
    let enthalpy_drop = initial_total - final_total;
    let energy_residual = enthalpy_drop - wall_heat;
    let energy_relative = energy_residual.abs() / wall_heat.max(1.0);
    let final_interface = latent_interface(&latent, v.dx, v.latent_density)?;

    if max_pde_residual > SOLVE_RESIDUAL_TOL {
        return Err("linear-system residual exceeded terminal tolerance".into());
    }
    if !energy_relative.is_finite() || energy_relative > 5.0e-9 {
        return Err(format!("global energy residual {energy_relative} exceeds tolerance"));
    }
    if flags[intervals] != 1 || (potential[intervals] / K_L - TINF).abs() > 1.0e-2 {
        return Err("far boundary is no longer in the initial superheated-liquid state".into());
    }

    let count = (campaign.time_steps * nodes) as f64;
    Ok(Summary {
        intervals,
        grid_points: nodes,
        time_steps: campaign.time_steps,
        dx_m: v.dx,
        dt_s: v.dt,
        final_front_exact_m: exact_front(campaign.end_time_s),
        interface_error_mean_abs_m: interface_error_sum / campaign.time_steps as f64,
        interface_final_error_m: (final_interface - exact_front(campaign.end_time_s)).abs(),
        temperature_mean_abs_error_c: temperature_abs_sum / count,
        temperature_rms_error_c: (temperature_square_sum / count).sqrt(),
        mean_flag_iterations: total_flag_iterations as f64 / campaign.time_steps as f64,
        max_flag_iterations,
        total_tridiagonal_segments: total_segments,
        max_pde_residual_relative: max_pde_residual,
        wall_heat_j_m2: wall_heat,
        enthalpy_drop_j_m2: enthalpy_drop,
        energy_residual_relative: energy_relative,
        final_interface_m: final_interface,
    })
}

fn all_summaries() -> Result<Vec<Summary>, String> {
    [(48, 72), (96, 144), (192, 288), (384, 576), (768, 1152)]
        .into_iter()
        .map(|(n, nt)| run_campaign(Campaign::published(n, nt)))
        .collect()
}

fn verify() -> Result<(), String> {
    let values = all_summaries()?;
    let mut last_temp = f64::INFINITY;
    let mut last_front = f64::INFINITY;
    for s in &values {
        if !s.temperature_mean_abs_error_c.is_finite()
            || s.temperature_mean_abs_error_c >= last_temp
        {
            return Err("temperature field error failed mesh refinement".into());
        }
        if !s.interface_error_mean_abs_m.is_finite()
            || s.interface_error_mean_abs_m >= last_front
        {
            return Err("interface error failed mesh refinement".into());
        }
        if s.energy_residual_relative > 5.0e-9 || s.max_pde_residual_relative > SOLVE_RESIDUAL_TOL {
            return Err("energy or block-system residual failed".into());
        }
        if s.max_flag_iterations > MAX_FLAG_ITERATIONS {
            return Err("flag consistency iteration cap exceeded".into());
        }
        last_temp = s.temperature_mean_abs_error_c;
        last_front = s.interface_error_mean_abs_m;
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("intervals,grid_points,nt,dx_m,dt_s,exact_front_m,interface_final_m,interface_final_abs_error_m,interface_mean_abs_error_m,temperature_mean_abs_error_c,temperature_rms_error_c,mean_flag_iterations,max_flag_iterations,total_tridiagonal_segments,max_block_residual_relative,wall_heat_j_m2,enthalpy_drop_j_m2,energy_residual_relative,status");
    for s in all_summaries()? {
        println!(
            "{},{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.8},{},{},{:.12e},{:.12e},{:.12e},{:.12e},PASS",
            s.intervals, s.grid_points, s.time_steps, s.dx_m, s.dt_s,
            s.final_front_exact_m, s.final_interface_m, s.interface_final_error_m, s.interface_error_mean_abs_m,
            s.temperature_mean_abs_error_c, s.temperature_rms_error_c,
            s.mean_flag_iterations, s.max_flag_iterations, s.total_tridiagonal_segments,
            s.max_pde_residual_relative, s.wall_heat_j_m2, s.enthalpy_drop_j_m2,
            s.energy_residual_relative
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("FLAG_UPDATE_OK: 1D implicit flag consistency, field/interface refinement and energy closure")),
        [a] if a == "--verify" => verify().map(|()| println!("FLAG_UPDATE_OK: 1D implicit flag consistency, field/interface refinement and energy closure")),
        [a] if a == "--csv" => emit_csv(),
        [a] if a == "--help" || a == "-h" => { println!("Usage: flag-update-1d [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(e) = result { eprintln!("FLAG_UPDATE_ERROR: {e}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_and_latent_constraints_match_three_flags() {
        assert!((gamma_for_flag(-1, 1.0e-6, 2.0e-6).unwrap() - 1.0e6).abs() < 1e-8);
        assert_eq!(gamma_for_flag(0, 1.0e-6, 2.0e-6).unwrap(), 0.0);
        assert!((gamma_for_flag(1, 1.0e-6, 2.0e-6).unwrap() - 5.0e5).abs() < 1e-8);
        assert_eq!(fixed_latent_for_flag(-1, 100.0).unwrap(), Some(0.0));
        assert_eq!(fixed_latent_for_flag(0, 100.0).unwrap(), None);
        assert_eq!(fixed_latent_for_flag(1, 100.0).unwrap(), Some(100.0));
    }

    #[test]
    fn thomas_matches_dense_extended_system_on_small_mixed_flag_patterns() {
        // A deterministic small dense oracle checks the segment-reduced solve.
        for pattern in [
            vec![-1, -1, 1, 1, -1, -1],
            vec![-1, 1, 0, 1, 0, -1],
            vec![-1, 0, -1, 0, 1, 1],
            vec![-1, 1, 1, 1, 1, -1],
        ] {
            let e_old = vec![-25.0, 110.0, 120.0, 130.0, 140.0, 150.0];
            let (v, s, _) = solve_fixed_flags(&e_old, &pattern, 0.1, 1.0, 1.0, 1.0, 100.0, -10.0).unwrap();
            let (vd, sd) = dense_flag_reference(&e_old, &pattern, 0.1, 1.0, 1.0, 1.0, 100.0, -10.0);
            for i in 0..pattern.len() {
                assert!((v[i] - vd[i]).abs() < 1.0e-10);
                assert!((s[i] - sd[i]).abs() < 1.0e-10);
            }
        }
    }

    fn dense_flag_reference(
        e_old: &[f64], flags: &[i8], dt: f64, dx: f64,
        alpha_s: f64, alpha_l: f64, latent_density: f64, wall: f64
    ) -> (Vec<f64>, Vec<f64>) {
        let intervals = flags.len() - 1;
        let size = 2 * intervals;
        let r = dt / (dx * dx);
        let mut a = vec![vec![0.0; size]; size];
        let mut b = vec![0.0; size];
        for node in 1..=intervals {
            let row = node - 1;
            let gamma = gamma_for_flag(flags[node], alpha_s, alpha_l).unwrap();
            let fixed_s = fixed_latent_for_flag(flags[node], latent_density).unwrap();
            a[row][node - 1] = gamma + 2.0 * r;
            if node == 1 {
                b[row] = e_old[node] - fixed_s.unwrap_or(0.0) + r * wall;
            } else {
                a[row][node - 2] = if node == intervals { -2.0 * r } else { -r };
                b[row] = e_old[node] - fixed_s.unwrap_or(0.0);
            }
            if node < intervals {
                a[row][node] = -r;
            }
            a[row][intervals + node - 1] = 1.0;

            let crow = intervals + node - 1;
            match flags[node] {
                1 | -1 => {
                    a[crow][intervals + node - 1] = 1.0;
                    b[crow] = fixed_s.expect("solid/liquid flag has fixed latent contribution");
                }
                0 => {
                    a[crow][node - 1] = 1.0;
                    b[crow] = 0.0;
                }
                _ => panic!("invalid test flag"),
            }
        }
        let solution = gaussian_elimination(a, b);
        let mut v = vec![wall; flags.len()];
        let mut s = vec![0.0; flags.len()];
        for node in 1..=intervals {
            v[node] = solution[node - 1];
            s[node] = solution[intervals + node - 1];
        }
        (v, s)
    }

    fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
        let n = b.len();
        for col in 0..n {
            let mut pivot = col;
            for row in col + 1..n {
                if a[row][col].abs() > a[pivot][col].abs() { pivot = row; }
            }
            a.swap(col, pivot); b.swap(col, pivot);
            let p = a[col][col];
            assert!(p.abs() > 1.0e-14);
            for j in col..n { a[col][j] /= p; }
            b[col] /= p;
            for row in 0..n {
                if row == col { continue; }
                let factor = a[row][col];
                for j in col..n { a[row][j] -= factor * a[col][j]; }
                b[row] -= factor * b[col];
            }
        }
        b
    }

    #[test]
    fn flag_updates_follow_published_consistency_rules() {
        let flags = vec![-1, 1, 0, -1];
        let v = vec![-20.0, -0.1, 0.0, 0.2];
        let s = vec![0.0, 100.0, -1.0, 0.0];
        let next = update_flags(&flags, &v, &s, 100.0).unwrap();
        assert_eq!(next, vec![-1, 0, -1, 0]);
    }

    #[test]
    fn small_campaign_conserves_energy_and_converges_flags() {
        let result = run_campaign(Campaign::published(48, 72)).unwrap();
        assert!(result.energy_residual_relative < 5.0e-9);
        assert!(result.max_flag_iterations <= MAX_FLAG_ITERATIONS);
        assert!(result.temperature_mean_abs_error_c.is_finite());
    }

    #[test]
    fn all_published_refinements_have_decreasing_error_and_closed_energy() {
        verify().unwrap();
    }

    #[test]
    fn invalid_campaigns_reject() {
        assert!(Campaign::published(4, 72).validate().is_err());
        assert!(Campaign::published(48, 0).validate().is_err());
        assert!(Campaign::published(48, MAX_STEPS + 1).validate().is_err());
    }
}
