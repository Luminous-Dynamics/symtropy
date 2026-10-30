//! Explicit conservative finite-volume solver for the published 1D two-phase Stefan problem.
//! This is a transparent explicit baseline, not the Peters flag-update algorithm.
//! Compile directly with rustc; it is intentionally outside the Bevy workspace.

const RHO: f64 = 1.0;
const CP: f64 = 2.5e6;
const K: f64 = 2.0;
const LATENT: f64 = 1.0e8;
const TW: f64 = -10.0;
const TM: f64 = 0.0;
const TINF: f64 = 2.0;
const LENGTH: f64 = 6.0;
const END_TIME: f64 = 288.0 * 3600.0;
const CFL: f64 = 0.2;
const LAMBDA: f64 = 0.32239063292230086;
const SQRT_PI: f64 = 1.772453850905516;
const MAX_STEPS: usize = 100_000;
const MAX_OBS: usize = 1152;

#[derive(Clone, Copy)]
struct Case { n: usize, cfl: f64, length: f64, end_time: f64 }

impl Case {
    fn published(n: usize) -> Self { Self { n, cfl: CFL, length: LENGTH, end_time: END_TIME } }
    fn validate(self) -> Result<Validated, String> {
        if self.n < 8 || !self.length.is_finite() || self.length <= 0.0
            || !self.end_time.is_finite() || self.end_time <= 0.0
            || !self.cfl.is_finite() || self.cfl <= 0.0 || self.cfl > 0.2 {
            return Err("invalid mesh, geometry, time or CFL".into());
        }
        let alpha = K / (RHO * CP);
        let dx = self.length / self.n as f64;
        let max_dt = self.cfl * dx * dx / alpha;
        let nf = (self.end_time / max_dt).ceil();
        if !nf.is_finite() || nf < 1.0 || nf > MAX_STEPS as f64 {
            return Err("timestep budget exceeded".into());
        }
        let steps = nf as usize;
        let dt = self.end_time / steps as f64;
        let fo = alpha * dt / (dx * dx);
        if fo > self.cfl * (1.0 + 64.0 * f64::EPSILON) {
            return Err("effective Fourier number exceeds stability bound".into());
        }
        Ok(Validated { case: self, alpha, dx, dt, steps, fo })
    }
}

struct Validated { case: Case, alpha: f64, dx: f64, dt: f64, steps: usize, fo: f64 }

struct Summary {
    n: usize, dx: f64, dt: f64, steps: usize, observations: usize, fo: f64,
    front_exact: f64, front_half_mean_error: f64, front_half_final: f64,
    solid_equivalent: f64, temp_l1: f64, temp_l2: f64,
    q_wall: f64, enthalpy_drop: f64, energy_residual: f64, energy_relative: f64,
    far_temperature: f64, far_solid_fraction: f64,
}

fn erf(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x.abs() > 1.25 { return Err("erf argument outside [-1.25,1.25]".into()); }
    if x == 0.0 { return Ok(0.0); }
    let x2 = x*x;
    let mut term=x;
    let mut sum=x;
    let mut comp=0.0;
    for n in 1..=160 {
        term *= -x2 / n as f64 * (2*n-1) as f64 / (2*n+1) as f64;
        let y=term-comp;
        let next=sum+y;
        comp=(next-sum)-y;
        sum=next;
        if term.abs() < 0.2*f64::EPSILON*sum.abs() { break; }
    }
    Ok(2.0*sum/SQRT_PI)
}

// erfc(x)=Q(1/2,x^2), evaluated by the continued fraction for the
// regularized upper incomplete gamma. This avoids 1-erf cancellation in the tail.
fn erfc(x: f64) -> Result<f64,String> {
    if !x.is_finite() || x < 0.0 { return Err("invalid erfc argument".into()); }
    if x <= 1.25 { return Ok(1.0-erf(x)?); }
    let xx=x*x;
    if !xx.is_finite() { return Ok(0.0); }
    let a=0.5;
    let mut b=xx+0.5;
    let mut c=1.0e300;
    let mut d=1.0/b;
    let mut h=d;
    for i in 1..=256 {
        let fi=i as f64;
        let an=-fi*(fi-a);
        b+=2.0;
        d=an*d+b;
        if d.abs()<1.0e-300 { d=1.0e-300; }
        c=b+an/c;
        if c.abs()<1.0e-300 { c=1.0e-300; }
        d=1.0/d;
        let delta=d*c;
        h*=delta;
        if (delta-1.0).abs()<3.0e-15 { break; }
        if i==256 { return Err("erfc continued fraction failed to converge".into()); }
    }
    let log_gamma_half=0.5723649429247001;
    Ok((-xx + a*xx.ln() - log_gamma_half).exp()*h)
}

fn exact_front(t:f64)->f64 { 2.0*LAMBDA*(8.0e-7*t).sqrt() }

fn exact_temperature(x:f64,t:f64)->Result<f64,String> {
    if !x.is_finite() || x<0.0 || !t.is_finite() || t<=0.0 { return Err("invalid exact field query".into()); }
    let a=2.0*(8.0e-7*t).sqrt();
    let eta=x/a;
    if eta<=LAMBDA {
        Ok(TW+(TM-TW)*erf(eta)?/erf(LAMBDA)?)
    } else {
        Ok(TINF+(TM-TINF)*erfc(eta)?/erfc(LAMBDA)?)
    }
}

// Shifted enthalpy H=E-rho*L: liquid H>0; mixed phase -rho*L<=H<=0;
// solid H<-rho*L. Returns temperature and solid volume fraction.
fn invert(h:f64)->Result<(f64,f64),String> {
    if !h.is_finite() { return Err("non-finite enthalpy".into()); }
    let latent=RHO*LATENT;
    let sensible=RHO*CP;
    let (t,fs)=if h>0.0 {
        (TM+h/sensible,0.0)
    } else if h>=-latent {
        (TM,(-h/latent).clamp(0.0,1.0))
    } else {
        (TM+(h+latent)/sensible,1.0)
    };
    if !t.is_finite() || !(0.0..=1.0).contains(&fs) { return Err("invalid inverted enthalpy".into()); }
    Ok((t,fs))
}

// Linear interpolation of the point where solid fraction crosses 0.5.
// A virtual wall point at x=0 is fully solid (cold Dirichlet boundary).
fn half_solid_front(fs:&[f64],dx:f64)->Result<f64,String> {
    let mut xl=0.0;
    let mut fl=1.0;
    for (i,fr) in fs.iter().copied().enumerate() {
        let xr=(i as f64+0.5)*dx;
        if fl>=0.5 && fr<=0.5 {
            let den=fl-fr;
            if den<=0.0 { return Ok(xr); }
            return Ok(xl+((fl-0.5)/den).clamp(0.0,1.0)*(xr-xl));
        }
        xl=xr; fl=fr;
    }
    Err("interface contour did not cross within domain".into())
}

fn solve(case:Case)->Result<Summary,String> {
    let v=case.validate()?;
    let n=case.n;
    let dx=v.dx;
    let h0=RHO*CP*(TINF-TM);
    let mut h=vec![h0;n];
    let mut next=vec![h0;n];
    let mut tfield=vec![TINF;n];
    let mut fs=vec![0.0;n];
    let mut qwall=0.0;
    let obs=v.steps.min(MAX_OBS);
    let mut obs_index=1usize;
    let mut next_obs=(obs_index*v.steps).div_ceil(obs);
    let mut temp_abs=0.0;
    let mut temp_sq=0.0;
    let mut front_abs=0.0;
    let mut nobs=0usize;

    for step in 0..v.steps {
        for i in 0..n { tfield[i]=invert(h[i])?.0; }
        qwall += 2.0*K*(tfield[0]-TW)/dx*v.dt;

        for i in 0..n {
            let west=if i==0 { 2.0*K*(TW-tfield[i])/dx } else { K*(tfield[i-1]-tfield[i])/dx };
            let east=if i+1==n { 0.0 } else { K*(tfield[i+1]-tfield[i])/dx };
            next[i]=h[i]+v.dt*(west+east)/dx;
            if !next[i].is_finite() { return Err("explicit update produced non-finite enthalpy".into()); }
        }
        std::mem::swap(&mut h,&mut next);

        let done=step+1;
        if done==next_obs {
            for i in 0..n {
                let (tn,fr)=invert(h[i])?;
                fs[i]=fr;
                let x=(i as f64+0.5)*dx;
                let te=exact_temperature(x,done as f64*v.dt)?;
                let e=tn-te;
                temp_abs+=e.abs();
                temp_sq+=e*e;
            }
            let time=done as f64*v.dt;
            front_abs+=(half_solid_front(&fs,dx)?-exact_front(time)).abs();
            nobs+=1;
            obs_index+=1;
            if obs_index<=obs { next_obs=(obs_index*v.steps).div_ceil(obs); }
        }
    }
    if nobs!=obs { return Err("observation schedule incomplete".into()); }

    let mut enthalpy_sum=0.0;
    let mut solid_equivalent=0.0;
    for i in 0..n {
        let (tn,fr)=invert(h[i])?;
        tfield[i]=tn;
        fs[i]=fr;
        enthalpy_sum+=h[i];
        solid_equivalent+=fr*dx;
    }
    let front_final=half_solid_front(&fs,dx)?;
    let enthalpy_drop=(h0*n as f64-enthalpy_sum)*dx;
    let energy_residual=enthalpy_drop-qwall;
    let energy_relative=energy_residual.abs()/qwall;
    let far_t=tfield[n-1];
    let far_fs=fs[n-1];

    if far_fs!=0.0 || (far_t-TINF).abs()>1.0e-2 {
        return Err("DOMAIN_TOO_SHORT: far boundary not in superheated-liquid state".into());
    }
    if !qwall.is_finite() || qwall<=0.0 { return Err("invalid wall heat integral".into()); }
    if energy_relative>5.0e-12 { return Err("global energy closure failed".into()); }

    let count=(obs*n) as f64;
    Ok(Summary {
        n,dx_m:dx,dt_s:v.dt,steps:v.steps,observations:obs,fo:v.fo,
        front_exact:exact_front(case.end_time),
        front_half_mean_error:front_abs/obs as f64,
        front_half_final:front_final,
        solid_equivalent:solid_equivalent,
        temp_l1:temp_abs/count,
        temp_l2:(temp_sq/count).sqrt(),
        q_wall:qwall,enthalpy_drop,
        energy_residual,energy_relative,
        far_temperature:far_t,far_solid_fraction:far_fs
    })
}

fn all_summaries()->Result<Vec<Summary>,String> {
    [48,96,192,384,768].into_iter().map(|n|solve(Case::published(n))).collect()
}

fn verify()->Result<(),String> {
    let xs=all_summaries()?;
    let mut last_field=f64::INFINITY;
    let mut last_front=f64::INFINITY;
    for s in &xs {
        if s.temp_l1>=last_field { return Err("temperature L1 error did not decrease under refinement".into()); }
        if s.front_half_mean_error>=last_front { return Err("interface mean error did not decrease under refinement".into()); }
        if s.energy_relative>5.0e-12 || s.fo>CFL { return Err("energy or stability gate failed".into()); }
        last_field=s.temp_l1; last_front=s.front_half_mean_error;
    }
    let last=xs.last().ok_or("empty refinement set")?;
    if last.temp_l1>=2.0e-3 || last.front_half_mean_error>=5.0e-3 {
        return Err("finest-grid accuracy gate failed".into());
    }
    Ok(())
}

fn emit_csv()->Result<(),String> {
    println!("cells,dx_m,dt_s,steps,observations,effective_fourier,exact_front_m,half_solid_front_final_m,half_solid_front_mean_abs_error_m,solid_equivalent_length_m,temperature_mean_abs_error_c,temperature_rms_error_c,wall_heat_j_m2,enthalpy_decrease_j_m2,energy_residual_j_m2,energy_residual_relative,far_temperature_c,far_solid_fraction,status");
    for s in all_summaries()? {
        println!(
            "{},{:.12e},{:.12e},{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},PASS",
            s.n,s.dx,s.dt,s.steps,s.observations,s.fo,s.front_exact,s.front_half_final,
            s.front_half_mean_error,s.solid_equivalent,s.temp_l1,s.temp_l2,
            s.q_wall,s.enthalpy_drop,s.energy_residual,s.energy_relative,s.far_temperature,s.far_solid_fraction
        );
    }
    Ok(())
}

fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    let result=match args.as_slice() {
        []=>verify().map(|()|println!("TWO_PHASE_FV_OK: field/interface refinement and global energy closure")),
        [a] if a=="--verify"=>verify().map(|()|println!("TWO_PHASE_FV_OK: field/interface refinement and global energy closure")),
        [a] if a=="--csv"=>emit_csv(),
        [a] if a=="--help"||a=="-h"=>{println!("Usage: two-phase-fv [--verify|--csv|--help]");Ok(())},
        _=>Err("expected --verify, --csv or --help".into())
    };
    if let Err(e)=result { eprintln!("TWO_PHASE_FV_ERROR: {e}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_reference_parameters_and_front() {
        let c=Case::published(48).validate().unwrap();
        assert!((c.alpha-8.0e-7).abs()<1e-20);
        assert!((exact_front(END_TIME)-0.5872254890905362).abs()<2e-13);
    }

    #[test]
    fn erfc_tail_is_accurate_and_stable() {
        assert!((erfc(1.0).unwrap()-0.15729920705028513).abs()<1e-13);
        assert!((erfc(2.0).unwrap()-0.004677734981047265).abs()<1e-14);
        assert!((erfc(3.294).unwrap()-0.000003186464038245543).abs()<1e-17);
        assert_eq!(erfc(40.0).unwrap(),0.0);
    }

    #[test]
    fn shifted_enthalpy_has_liquid_mixed_and_solid_branches() {
        let c=Case::published(48).validate().unwrap();
        let latent=RHO*LATENT;
        let (tl,fl)=invert(1.0e6).unwrap();
        let (tm,fm)=invert(-0.5*latent).unwrap();
        let (ts,fs)=invert(-1.5*latent).unwrap();
        assert!(tl>TM && fl==0.0);
        assert!(tm==TM && (fm-0.5).abs()<f64::EPSILON);
        assert!(ts<TM && fs==1.0);
        assert!(c.initial_enthalpy>0.0);
    }

    #[test]
    fn energy_closure_holds_on_all_meshes() {
        for s in all_summaries().unwrap() {
            assert!(s.energy_relative<5e-12);
            assert_eq!(s.far_solid_fraction,0.0);
        }
    }

    #[test]
    fn field_and_half_fraction_interface_errors_decrease() {
        verify().unwrap();
    }

    #[test]
    fn invalid_physical_and_numerical_inputs_reject() {
        let mut c=Case::published(48); c.cfl=0.5; assert!(c.validate().is_err());
        let mut c=Case::published(48); c.length=0.01; assert!(c.validate().is_err());
        let mut c=Case::published(48); c.cfl=1e-12; assert!(c.validate().is_err());
        let mut c=Case::published(48); c.end_time=f64::NAN; assert!(c.validate().is_err());
    }
}
