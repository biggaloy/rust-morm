// rustc -O --edition 2024 main.rs -o model; ./model <setting|all> <fault> <seed0> <count> [straight|pooled|both]
mod calls;
mod core;
mod machine;
mod world;
use crate::core::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

fn settings(pooled: bool) -> Vec<Cfg> {
    let b = || Cfg::base(pooled);
    let mut v = vec![
        b(),
        Cfg { name: "entries off", pend_max: 0, ..b() },
        Cfg { name: "caps of 1", pend_max: 1, cool_max: 1, ..b() },
        Cfg { name: "W five times E", e: 200 * MS, ..b() },
        Cfg { name: "budget 8", budget: 8, ..b() },
        Cfg { name: "caches off", caches: false, ..b() },
        Cfg { name: "no client input", client: false, ..b() },
        Cfg { name: "several, one server", several: true, ..b() },
        Cfg { name: "two servers", servers: 2, several: true, ..b() },
        Cfg { name: "three servers", servers: 3, several: true, ..b() },
        Cfg { name: "heavy", world: 0.2, ..b() },
        Cfg { name: "landing always", land: 1.0, ..b() },
        Cfg { name: "two servers, landing always, heavy", servers: 2, several: true, land: 1.0, world: 0.2, ..b() },
        Cfg { name: "savepoints", sp: 0.9, tx: 0.6, two: 0.9, held: if pooled { 0. } else { 0.1 }, ..b() },
        Cfg { name: "RR and SERIALIZABLE only", rr: 1.0, ..b() },
        Cfg { name: "quiet", world: 0.01, mix: [2., 0., 1., 4., 1., 0., 1., 0., 0.], ..b() },
        Cfg { name: "hand relabels", mix: [2., 1., 1., 4., 1., 1., 2., 1., 4.], ..b() },
        Cfg { name: "long transactions", gap2: 300 * MS, two: 0.9, gap: 80_000.0, ..b() },
        Cfg { name: "slow", gap: 800_000.0, world: 0.2, mix: [1., 0., 0., 0., 4., 0., 0., 0., 0.], mig_refresh: 0.0, ..b() },
        Cfg { name: "transactions without retry", tx: 0.8, retry0: true, held: 0.0, ..b() },
    ];
    if pooled {
        v.push(Cfg { name: "two processes", procs: 2, ..b() });
        v.push(Cfg { name: "three processes, heavy", procs: 3, world: 0.15, ..b() });
    } else {
        v.push(Cfg { name: "held", held: 0.3, held_app: 0.8, ..b() });
        v.push(Cfg { name: "held, quiet", held: 0.3, held_app: 0.8, world: 0.01, mix: [2., 0., 1., 4., 1., 0., 1., 0., 0.], ..b() });
        v.push(Cfg { name: "long holds", long: 1.0, gap: 120_000.0, ..b() });
        v.push(Cfg { name: "long holds, held", long: 1.0, held: 0.3, gap: 250_000.0, ..b() });
        v.push(Cfg { name: "long holds, quiet", long: 1.0, held: 0.2, gap: 200_000.0, world: 0.01, mix: [4., 0., 2., 2., 2., 0., 0., 0., 0.], ..b() });
        v.push(Cfg { name: "long holds, hand relabels", long: 1.0, held: 0.2, gap: 200_000.0, mix: [2., 1., 1., 4., 1., 1., 2., 1., 4.], ..b() });
        v.push(Cfg { name: "role limit", role_limit: 4, ..b() });
        v.push(Cfg { name: "role limit, heavy", role_limit: 4, world: 0.2, ..b() });
        v.push(Cfg { name: "refusals", world: 0.15, mix: [4., 1., 2., 1., 2., 0., 2., 6., 0.], ..b() });
        v.push(Cfg { name: "held, no rerun", held_rerun: 0.0, ..b() });
    }
    v
}

// T-0319's setting for each switch
fn fault_setting(f: u32, pooled: bool) -> Cfg {
    let b = || Cfg::base(pooled);
    let hand = [0., 0., 1., 1., 1., 0., 0., 0., 0.];
    match f {
        4 => Cfg { name: "f4", only: Some(F), world: 0.02, mix: hand, ..b() },
        5 => Cfg { name: "f5", only: Some(F), world: 0.004, mix: [1., 0., 0., 0., 0., 0., 0., 0., 0.], mig_refresh: 0.0, mig_col: 0.0, tx: 0.0, held: 0.0, ..b() },
        12 => Cfg { name: "f12", world: 0.2, mix: [3., 3., 1., 1., 1., 0., 0., 0., 0.], ..b() },
        14 => Cfg { name: "f14", servers: 2, several: true, ..b() },
        15 => Cfg { name: "f15", rr: 1.0, tx: 0.6, two: 1.0, gap2: 300 * MS, gap: 30_000.0, world: 0.3, mix: [3., 0., 0., 3.5, 3.5, 0., 0., 0., 0.], one_value: true, ..b() },
        16 => Cfg { name: "f16", tx: 0.9, two: 1.0, gap2: 300 * MS, sp: 1.0, gap: 40_000.0, world: 0.2, held: 0.05, ..b() },
        17 => Cfg { name: "f17", gap: 30_000.0, held: 0.4, held_app: 1.0, held_two: 1.0, held_gap: 300 * MS, tx: 0.36, two: 1.0, gap2: 300 * MS, rr: 1.0, world: 0.3, mix: [3., 0., 0., 3.5, 3.5, 0., 0., 0., 0.], one_value: true, ..b() },
        21 => Cfg { name: "f21", land: 1.0, ..b() },
        22 => Cfg { name: "f22", land: 1.0, tx: 0.9, two: 1.0, gap: 50_000.0, world: 0.2, mix: [0., 0., 0., 1., 1., 0., 0., 0., 0.], ..b() },
        23 => Cfg { name: "f23", lockwait: 20 * MS, tx: 0.0, held: 0.0, world: 0.2, mix: [0., 0., 1., 0., 1., 0., 0., 0., 0.], ..b() },
        24 => Cfg { name: "f24", budget: 8, only: Some(ST), burst: 1.0, burst_lo: 4, burst_hi: 8, gap: 50_000.0, world: 0.3, mix: [0., 0., 0., 0., 0., 0., 1., 0., 0.], ..b() },
        25 => Cfg { name: "f25", co_gap: 50 * MS, ..b() },
        27 => Cfg { name: "f27", tx: 0.6, retry0: true, co_gap: 50 * MS, timeout: 0.15, one_value: true, ..b() },
        28 => Cfg { name: "f28", two: 1.0, world: 0.2, ..b() },
        29 => Cfg { name: "f29", two: 1.0, world: 0.2, ser: 0.25, retry2: true, ..b() },
        30 => Cfg { name: "f30", role_limit: 4, ..b() },
        31 => Cfg { name: "f31", tx: 0.6, two: 0.9, gap2: 300 * MS, sp: 0.9, gap: 80_000.0, world: 0.2, mix: [1., 0.5, 0.5, 0.5, 0.5, 0.25, 6., 0.25, 0.], ..b() },
        132 => Cfg { name: "f132", long: 1.0, gap: 120_000.0, ..b() },
        _ => Cfg { name: "defaults", ..b() },
    }
}

fn run(c: &Cfg, seed0: u64, count: u64, threads: usize) -> (BTreeMap<&'static str, (u32, String)>, BTreeMap<&'static str, u64>) {
    let next = Arc::new(AtomicU64::new(0));
    let acc = Arc::new(Mutex::new((BTreeMap::<&'static str, (u32, String)>::new(), BTreeMap::<&'static str, u64>::new())));
    let mut hs = vec![];
    for _ in 0..threads {
        let (next, acc, c) = (next.clone(), acc.clone(), c.clone());
        hs.push(std::thread::spawn(move || {
            loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                if k >= count {
                    break;
                }
                let mut s = Sim::new(c.clone(), seed0 + k);
                s.spawn(Box::pin(world::driver()), 0);
                run_loop(&mut s);
                world::end_checks(&mut s);
                let mut a = acc.lock().unwrap();
                if s.fails.keys().any(|k| !k.starts_with("lit ") && !k.starts_with("ez ") && !k.starts_with("mid ")) {
                    a.0.entry("ANY").or_insert((0, String::new())).0 += 1;
                }
                for (k2, (_, m)) in s.fails.iter() {
                    let e = a.0.entry(k2).or_insert((0, format!("seed {}: {}", seed0 + k, m)));
                    e.0 += 1;
                }
                for (k2, n) in s.cnt.iter() {
                    let e = a.1.entry(k2).or_insert(0);
                    if k2.ends_with("(max)") { *e = (*e).max(*n) } else { *e += n }
                }
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    Arc::try_unwrap(acc).unwrap().into_inner().unwrap()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let which = a.get(1).map_or("all", |x| x.as_str());
    let fault: u32 = a.get(2).map_or(0, |x| x.parse().unwrap());
    let seed0: u64 = a.get(3).map_or(0, |x| x.parse().unwrap());
    let count: u64 = a.get(4).map_or(256, |x| x.parse().unwrap());
    let mode = a.get(5).map_or("both", |x| x.as_str());
    let verbose = std::env::var("V").is_ok();
    let threads = std::env::var("T").map_or(11, |x| x.parse().unwrap());
    if let Some(x) = which.strip_prefix("set") {
        let f: u32 = x.parse().unwrap();
        let mut c = fault_setting(f, mode == "pooled");
        c.fault = fault;
        let (f1, _) = run(&c, seed0, count, threads);
        for (k, (x, m)) in &f1 {
            println!("{} {} e.g. {}", x, k, m);
        }
        return;
    }
    if which == "faults" {
        let pl = [1, 6, 7, 8, 9, 10, 14, 20, 22, 23, 25, 26, 27, 28, 29, 33];
        for f in (0..34u32).chain([132]) {
            for pooled in [false, true] {
                if (pooled && !pl.contains(&f)) || (!pooled && [22, 23, 26, 27].contains(&f)) || f == 0 {
                    continue;
                }
                let mut c = fault_setting(f, pooled);
                let (f0, _) = run(&c, seed0, count, threads);
                c.fault = if f == 132 { 32 } else { f };
                let (f1, _n1) = run(&c, seed0, count, threads);
                if std::env::var("V").is_ok() { println!("{:?}", _n1); }
            let show = |x: &BTreeMap<&'static str, (u32, String)>| x.iter().filter(|(k, _)| !k.starts_with("lit ") && !k.starts_with("ez ") && !k.starts_with("mid ")).map(|(k, v)| format!("{}={}", k, v.0)).collect::<Vec<_>>().join("; ");
                println!("switch {} [{}]: {}   | no fault: {}", f, if pooled { "pooled" } else { "straight" }, show(&f1), show(&f0));
            }
        }
        return;
    }
    for pooled in [false, true] {
        if (mode == "straight" && pooled) || (mode == "pooled" && !pooled) {
            continue;
        }
        for mut c in settings(pooled) {
            if which != "all" && c.name != which {
                continue;
            }
            c.fault = fault;
            for f in ["row7", "noreset", "a1s", "real", "trig", "both"] {
                if std::env::var("OPT").is_ok_and(|o| o.split(',').any(|x| x == f)) {
                    match f {
                        "row7" => c.row7_first = true,
                        "a1s" => c.a1_session = true,
                        "real" => c.retired_real = true,
                        "trig" => c.trig = true,
                        "noreset" => c.noreset = true,
                        _ => c.both_stmt = true,
                    }
                }
            }
            let t = std::time::Instant::now();
            let (f, n) = run(&c, seed0, count, threads);
            println!("== {} [{}] fault {} seeds {}..{} ({:.1}s)", c.name, if pooled { "pooled" } else { "straight" }, fault, seed0, seed0 + count, t.elapsed().as_secs_f64());
            for (k, (x, m)) in &f {
                println!("   FAIL {:4}/{} {}{}", x, count, k, if verbose { format!("   e.g. {}", m) } else { String::new() });
            }
            if verbose {
                for (k, x) in &n {
                    println!("        {} = {}", k, x);
                }
            }
        }
    }
}
