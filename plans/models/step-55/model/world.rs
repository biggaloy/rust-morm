// The load: calls and world events, and the end-of-sequence checks.
use crate::calls::*;
use crate::core::*;

pub fn respec(s: &mut Sim, sp: Spec) -> Spec {
    let mut x = draw(s);
    x.stmt = sp.stmt;
    x.path = sp.path;
    x.shape = sp.shape;
    x
}
pub fn draw(s: &mut Sim) -> Spec {
    let k = match s.c.only {
        Some(x) => x,
        None => {
            let mut v = vec![CAST, COL, PATH, F, ST, H];
            for (on, x) in [(s.c.both_stmt, BOTH), (s.c.trig, TRIG), (s.c.stc, STC), (s.c.nocast, NOC), (s.c.twot, TWO)] {
                if on {
                    v.push(x);
                }
            }
            v[s.rng.n(v.len() as u64) as usize]
        }
    };
    let val = s.rng.n(8) as u8;
    let u = s.rng.f();
    let label = if !s.c.client {
        if u < 0.78 { Label::Shared } else { Label::Newest }
    } else if u < 0.70 {
        Label::Shared
    } else if u < 0.90 {
        Label::Newest
    } else if u < 0.95 {
        Label::Bad
    } else if (s.c.retired_real || s.c.fault == 40) && s.rng.p(0.5) {
        Label::RetReal
    } else {
        Label::RetX
    };
    if s.c.one_value {
        return Spec { stmt: k, val: 0, label, path: s.rng.n(2) as u8, shape: s.rng.n(2) as u8 };
    }
    let shape = if s.c.client { s.rng.n(2) as u8 } else { 0 };
    Spec { stmt: k, val, label, path: s.rng.n(2) as u8, shape }
}
fn call(s: &mut Sim, sp: Spec) {
    let now = s.now;
    let p = s.rng.n(s.c.procs as u64) as usize;
    let u = s.rng.f();
    let (tx, held, two) = (s.c.tx, s.c.held, s.c.two);
    if u < tx {
        let retry = if s.c.retry0 { 0 } else if s.c.retry2 { 2 } else { s.rng.n(3) as u32 };
        let rr0 = s.c.rr;
        let rr = s.rng.p(rr0);
        let n = if s.rng.p(two) { 2 } else { 1 };
        let mut v = vec![];
        for i in 0..n {
            let x = if i == 0 { sp } else { draw(s) };
            let q = s.c.sp;
            v.push((x, s.rng.p(q)));
        }
        s.spawn(Box::pin(call_tx(p, v, retry, rr)), now);
    } else if u < tx + held {
        let (two, ap, lg) = (s.c.held_two, s.c.held_app, s.c.long);
        let app = s.rng.p(ap);
        let mut v = vec![sp];
        if s.rng.p(lg) {
            let k = s.rng.r(2, 8);
            for _ in 1..k {
                let x = draw(s);
                v.push(x);
            }
            s.spawn(Box::pin(call_held_long(p, v, app)), now);
            return;
        }
        if s.rng.p(two) {
            let x = draw(s);
            v.push(x);
        }
        s.spawn(Box::pin(call_held(p, v, app)), now);
    } else {
        // outside a transaction a search_path can't be set for one
        let mut sp = sp;
        if !s.c.path_out {
            sp.path = 0;
        }
        s.spawn(Box::pin(call_out(p, sp)), now);
    }
}
async fn call_held_long(p: usize, v: Vec<Spec>, app: bool) {
    // kept for up to 3 s: the gaps are up to 3 s / 8 each
    call_held_gap(p, v, app, 3 * S / 8).await
}

fn bump(s: &mut Sim, n: i64) {
    let now = s.now;
    s.db.n = n;
    s.db.nhist.push((now, n));
    for k in 0..s.c.servers {
        let d = if s.c.servers > 1 { s.rng.r(0, 800 * MS) } else { 0 };
        let last = s.db.replay[k].last().unwrap().0;
        let t = (now + d).max(last + 1);
        s.db.replay[k].push((t, n));
    }
}
fn swap(s: &mut Sim, st: TS) {
    let now = s.now;
    let l = s.db.types.last_mut().unwrap();
    *l = (st, now, now);
    s.db.types.push((TS::Cur, now, u64::MAX));
    s.db.last_change = now;
}
fn col(s: &mut Sim) {
    let now = s.now;
    s.db.col += 1;
    s.db.colhist.push(now);
    s.db.last_change = now;
}
fn reconnect(s: &mut Sim) {
    s.db.last_reconnect = s.now;
    if s.c.fault == 105 || s.c.fault == 106 {
        return; // a break: PgBouncer keeps its sessions
    }
    for j in 0..s.srv.len() {
        if s.srv[j].busy {
            s.srv[j].kill = true;
        } else {
            s.srv[j].sess = s.new_sess();
        }
    }
}
fn world(s: &mut Sim) {
    let mix = s.c.mix;
    let tot: f64 = mix.iter().sum();
    let mut u = s.rng.f() * tot;
    let mut k = 0;
    while k < 8 && u >= mix[k] {
        u -= mix[k];
        k += 1;
    }
    let now = s.now;
    match k {
        0 => {
            swap(s, TS::Retired);
            s.db.resets += 1;
            let n = s.db.n + 1;
            bump(s, n);
            let (r, c) = (s.c.mig_refresh, s.c.mig_col);
            if s.rng.p(r) {
                s.db.refresh += 1;
            } else if s.rng.p(c) {
                col(s);
            }
            if s.c.pooled && s.rng.p(0.5) {
                reconnect(s);
            }
        }
        1 => {
            let n = (s.db.n - s.rng.r(1, 2) as i64).max(0);
            if n != s.db.n {
                bump(s, n);
            }
        }
        2 => swap(s, TS::Dropped),
        3 => {
            // how the kept type is renamed or moved changes only its message's text, which nothing reads now
            swap(s, TS::Kept)
        }
        4 => col(s),
        5 => {}
        6 => {
            s.db.stored_bad = s.c.client && !s.db.stored_bad;
            // FG1: text that stops holding _morm_retired_x holds a good label half the time, else NULL, no row, or
            // a value its function fails on another way
            // FG1_OFF=1 keeps the earlier world, a good label only, to compare the outputs with the last round's
            if s.c.client && !s.db.stored_bad && std::env::var_os("FG1_OFF").is_none() {
                s.db.stored_alt = match s.rng.r(0, 4) { 0 | 1 => 0, 2 => 1, 3 => 2, _ => 3 };
                // the thirtieth review's FH1: of the values its function fails on another way, a third fail with
                // another 22P02 from enum_in, another bad label, and a third with one from another routine, bad text
                // for a number; FH1_OFF=1 keeps the last round's world, which draws nothing more
                if s.db.stored_alt == 3 && std::env::var_os("FH1_OFF").is_none() {
                    s.db.stored_alt = match s.rng.r(0, 2) { 0 => 3, 1 => 4, _ => 5 };
                }
            }
            s.db.stored_nm = s.rng.p(0.5);
        }
        7 => {
            if !s.c.pooled {
                let d = s.rng.r(0, 500 * MS);
                s.db.refused_until = s.db.refused_until.max(now + d);
                s.db.last_refusal_end = s.db.refused_until;
            }
        }
        _ => {
            // a kept type and its labels renamed by hand, as R-1294 says, the reset with it
            let ks: Vec<usize> = (0..s.db.types.len()).filter(|i| s.db.types[*i].0 == TS::Kept).collect();
            if !ks.is_empty() {
                let i = ks[s.rng.n(ks.len() as u64) as usize];
                let left = s.db.types[i].2;
                s.db.types[i] = (TS::Retired, now, left);
                s.db.last_change = now;
                if !s.c.noreset {
                    s.db.resets += 1;
                } else {
                    s.noreset_at.insert(i as u32, now);
                }
                s.inc("hand relabels");
            }
        }
    }
}
pub async fn driver() {
    let n = w().c.events;
    for _ in 0..n {
        let g = {
            let s = w();
            let g = s.c.gap;
            s.rng.exp(g)
        };
        sleep(g).await;
        let s = w();
        let wp = s.c.world;
        if s.rng.p(wp) {
            world(s);
            continue;
        }
        let sp = draw(s);
        let b = s.c.burst;
        let k = if s.rng.p(b) { s.rng.r(s.c.burst_lo, s.c.burst_hi) } else { 1 };
        for _ in 0..k {
            call(s, sp);
        }
    }
}
// I1, I3, I4 from the records
pub fn end_checks(s: &mut Sim) {
    let ms = s.now / MS;
    s.inc_by("ms simulated", ms);
    // the twenty-fifth review's FC1: each search_path SchemaChanged with entries on and no .retry(n), classed by when
    // its call began: after another one's error and before the entry that came after that error was made, which R-1271
    // now leaves out beside the calls already running at the first, or otherwise
    let (mut after, mut other) = (0u64, 0u64);
    for p in 0..s.pools.len() {
        for (stmt, v) in s.pools[p].path_fx.iter() {
            let cr = s.pools[p].created.get(stmt).cloned().unwrap_or_default();
            for (i, &(_, st, _)) in v.iter().enumerate() {
                let hit = v.iter().enumerate().any(|(j, &(et, _, _))| {
                    j != i && et < st && cr.iter().find(|c| **c >= et).is_none_or(|c| st < *c)
                });
                if hit { after += 1 } else { other += 1 }
            }
        }
    }
    if after + other > 0 {
        s.inc_by("search_path, entries on: SchemaChanged begun after another's error, before its entry", after);
        s.inc_by("search_path, entries on: SchemaChanged otherwise", other);
    }
    // the twenty-sixth review's FD1: each such SchemaChanged classed against the earliest other one whose error came
    // before its own and whose entry wasn't made yet when its run began: its run already in flight at that error, its
    // transaction started before that error, or started after it, in the gap; with no such failure, the first. The
    // README's older gap clause, "the transactions that start while morm rolls back the first ... before it marks the
    // statement pending", counted only the last kind ("lit FD1"); R-1253's and R-1272's words now count runs, as I8 does
    let (mut first, mut fl, mut before, mut gap, mut lit) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut eg = String::new();
    for p in 0..s.pools.len() {
        for (stmt, v) in s.pools[p].path_fx.iter() {
            let cr = s.pools[p].created.get(stmt).cloned().unwrap_or_default();
            for (i, &(ei, st, tt)) in v.iter().enumerate() {
                let j = v
                    .iter()
                    .enumerate()
                    .filter(|&(j, &(et, _, _))| j != i && et < ei && cr.iter().find(|c| **c >= et).is_none_or(|c| st < *c))
                    .map(|(_, &(et, _, _))| et)
                    .min();
                match j {
                    None => first += 1,
                    Some(et) if st < et => {
                        fl += 1;
                        lit += 1;
                        if eg.is_empty() {
                            eg = format!("stmt={} run {} began before the earlier error {}, its transaction at {}", stmt, st, et, tt);
                        }
                    }
                    Some(et) if tt < et => {
                        before += 1;
                        lit += 1;
                        if eg.is_empty() {
                            eg = format!("stmt={} transaction began {} before the earlier error {}, its run at {}", stmt, tt, et, st);
                        }
                    }
                    Some(_) => gap += 1,
                }
            }
        }
    }
    if first + fl + before + gap > 0 {
        s.inc_by("search_path, entries on, FD1: SchemaChanged the first", first);
        s.inc_by("search_path, entries on, FD1: SchemaChanged in flight at an earlier one's error", fl);
        s.inc_by("search_path, entries on, FD1: SchemaChanged in a transaction started before it", before);
        s.inc_by("search_path, entries on, FD1: SchemaChanged in a transaction started in the gap", gap);
    }
    if lit > 0 {
        s.fail("lit FD1 R-1253 and R-1272 gap clause, transactions that start while morm rolls back", eg.clone());
    }
    let wd = s.c.w;
    let ch: Vec<u64> = s.db.nhist.iter().skip(1).map(|x| x.0).collect();
    let several = s.c.several;
    if std::env::var("DBG").is_ok() {
        let t = |x: &u64| *x > 14_000_000 && *x < 16_000_000;
        println!("n {:?}", s.db.nhist.iter().filter(|x| t(&x.0)).collect::<Vec<_>>());
        println!("epochs (id,t,snap,rises,open) {:?}", s.pools[0].epochs.iter().filter(|x| t(&x.1)).collect::<Vec<_>>());
        println!("acts (t,n,snap) {:?}", s.pools[0].acts.iter().filter(|x| t(&x.t)).map(|x| (x.t, x.n, x.snap)).collect::<Vec<_>>());
        println!("moves {:?}", s.pools[0].moved.iter().filter(|x| t(&x.0)).collect::<Vec<_>>());
    }
    for p in 0..s.pools.len() {
        // I1
        let mut lit_used = vec![false; ch.len()];
        let mut new_used = vec![false; ch.len()];
        let mut bad = vec![];
        for i in 1..s.pools[p].acts.len() {
            let (a, b) = (&s.pools[p].acts[i - 1], &s.pools[p].acts[i]);
            if b.t - a.t >= wd {
                continue;
            }
            if several {
                if b.rise_lit <= a.rise_lit {
                    bad.push(("lit I1 several (rose between them)", a.t, b.t));
                }
                if b.rise <= a.rise {
                    bad.push(("new I1 several", a.t, b.t));
                }
                continue;
            }
            let f = |sn: u64, used: &mut Vec<bool>| {
                for (j, c) in ch.iter().enumerate() {
                    if !used[j] && *c >= sn && *c <= b.t {
                        used[j] = true;
                        return true;
                    }
                }
                false
            };
            if !f(a.snap_lit, &mut lit_used) {
                bad.push(("lit I1", a.t, b.t));
            }
            if !f(a.snap, &mut new_used) {
                bad.push(("new I1", a.t, b.t));
            }
        }
        // I3
        let mut used = vec![false; ch.len()];
        for i in 1..s.pools[p].epochs.len() {
            let (a, b) = (s.pools[p].epochs[i - 1], s.pools[p].epochs[i]);
            if b.1 - a.1 >= wd {
                continue;
            }
            if several {
                if b.3 <= a.3 {
                    bad.push(("I3 several", a.1, b.1));
                }
                continue;
            }
            let mut ok = false;
            for (j, c) in ch.iter().enumerate() {
                if !used[j] && *c >= a.2 && *c <= b.1 {
                    used[j] = true;
                    ok = true;
                    break;
                }
            }
            if !ok {
                bad.push(("I3", a.1, b.1));
            }
        }
        for (k, a, b) in bad {
            s.fail(k, format!("{} {}", a, b));
        }
        // I12
        if p == 0 {
            let left = s.tasks.iter().filter(|t| t.is_some()).count();
            if left > 0 {
                s.fail("I12 a call that never ends", format!("{}", left));
            }
        }
        // I4
        let max = s.c.max as u32;
        let er: Vec<(u64, u32)> = s.pools[p].ep_repl.iter().map(|x| (*x.0, *x.1)).collect();
        for (id, n) in er {
            if n > max {
                s.fail("I4 per epoch replacements", format!("epoch {} {}", id, n));
            }
            let at = s.pools[p].epochs.iter().find(|e| e.0 == id).map_or(0, |e| e.4);
            if n as u64 > at {
                s.fail("I3 more than open at its start", format!("epoch {} {} of {}", id, n, at));
            }
        }
        let eo: Vec<u32> = s.pools[p].ep_open.values().copied().collect();
        for n in eo {
            if n > max + 2 {
                s.fail("I4 per epoch opens", format!("{}", n));
            }
        }
        let cr: Vec<u32> = s.pools[p].cg_repl.values().copied().collect();
        for n in cr {
            if n > max {
                s.fail("I4 per action", format!("{}", n));
            }
        }
        let so = s.pools[p].sopens.clone();
        if s.c.pooled && !so.is_empty() {
            s.fail("I4 pooled", String::new());
        }
        let mut lo = 0;
        let mv_t: Vec<u64> = s.pools[p].moved.iter().map(|x| x.0).collect();
        for hi in 0..so.len() {
            while so[hi] - so[lo] >= wd {
                lo += 1;
            }
            // the thirtieth review's FH2: I4 counts, per span, the moves of the pool's last-read <n> from
            // stale_cooldown before the span's start to its end, whenever the change was committed; the span checked
            // starts at its first open, which counts the fewest moves of any span holding these opens
            let mc = ch.iter().filter(|c| **c >= so[lo] && **c <= so[hi]).count() as u64;
            let m = mv_t.iter().filter(|t| **t + wd >= so[lo] && **t <= so[hi]).count() as u64;
            let n = (hi - lo + 1) as u64;
            if n > 4 * max as u64 + 4 + 2 * max as u64 * m {
                s.fail("I4 span", format!("{} opens, {} moves", n, m));
            }
            // the reading FH2 rules out, per migration committed within the span, as an older wording's check
            if n > 4 * max as u64 + 4 + 2 * max as u64 * mc {
                s.fail("lit FH2 I4 span opens, per change committed in the span", format!("{} opens, {} changes", n, mc));
            }
            let e = s.cnt.entry("most stale-driven opens in a span (max)").or_insert(0);
            *e = (*e).max(n);
        }
        // the twenty-eighth review's FF1: stale-driven replacements in any span of stale_cooldown, an epoch's and a
        // *replace* action's together. R-1268's "Shorter:" now says up to four times max in any span, plus a round of
        // each per migration, as R-1271's "Churn" and I4 do ("I4 span replacements, FF1"); its older wording, max per
        // period and twice that in any span, plus a round per migration, counted an epoch's alone ("lit FF1")
        let rt = s.pools[p].repl_t.clone();
        let mx = max as u64;
        let (mut lo, mut most, mut most_ep) = (0usize, 0u64, 0u64);
        let (mut lit, mut eg, mut over, mut ep_over) = (false, String::new(), String::new(), String::new());
        let (mut lit_c, mut lit_ce) = (String::new(), String::new());
        for hi in 0..rt.len() {
            while rt[hi].0 - rt[lo].0 >= wd {
                lo += 1;
            }
            // a change counts for the span when the pool can have learned it there: from W before the span's first
            // replacement, as a standby's replay or a read after the change can trail its commit
            let mc = ch.iter().filter(|c| **c + wd >= rt[lo].0 && **c <= rt[hi].0).count() as u64;
            // the twenty-ninth round: or when the pool did learn it there, by a read that moved its <n>, which can come
            // any time after the commit, since <n> is read only on a failure; it took the larger of the two counts.
            // The thirtieth review's FH2: the text now names the count, the moves of the pool's last-read <n> from
            // stale_cooldown before the span's start to its end, whenever the change was committed, and the checks
            // count those alone; the commit-time count is kept as an older reading's check ("lit FH2")
            let m = s.pools[p].moved.iter().filter(|x| x.0 + wd >= rt[lo].0 && x.0 <= rt[hi].0).count() as u64;
            let n = (hi - lo + 1) as u64;
            let ne =rt[lo..=hi].iter().filter(|x| x.1 == 0).count() as u64;
            if n > 4 * mx + 2 * mx * mc && lit_c.is_empty() {
                lit_c = format!("{} replacements in {}..{}, {} changes committed", n, rt[lo].0, rt[hi].0, mc);
            }
            if ne > 2 * mx + mx * mc && lit_ce.is_empty() {
                lit_ce = format!("{} of an epoch's in {}..{}, {} changes committed", ne, rt[lo].0, rt[hi].0, mc);
            }
            if n > 2 * mx + mx * m && !lit {
                lit = true;
                eg = format!("{} replacements in {}..{}, {} an epoch's, {} changes", n, rt[lo].0, rt[hi].0, ne, m);
            }
            // R-0981's bound on an epoch's replacements alone: 2 × max in any span, plus max per migration
            if ne > 2 * mx + mx * m && ep_over.is_empty() {
                ep_over = format!("{} of an epoch's in {}..{}, {} changes", ne, rt[lo].0, rt[hi].0, m);
                if std::env::var("DBGR").is_ok() {
                    let (a, b) = (rt[lo].0, rt[hi].0);
                    println!("span {}..{}", a, b);
                    println!("repl (t, epoch, site) {:?}", s.pools[p].repl_dbg.iter().filter(|x| x.0 + 2 * wd >= a && x.0 <= b + wd).collect::<Vec<_>>());
                    println!("epochs (id,t,snap,rises,open) {:?}", s.pools[p].epochs.iter().filter(|x| x.1 + 3 * wd >= a && x.1 <= b + wd).collect::<Vec<_>>());
                    println!("changes (t,n) {:?}", s.db.nhist.iter().filter(|x| x.0 + 10 * wd >= a && x.0 <= b + wd).collect::<Vec<_>>());
                    println!("moves (t,n,snap) {:?}", s.pools[p].moved.iter().filter(|x| x.0 + 10 * wd >= a && x.0 <= b + wd).collect::<Vec<_>>());
                }
            }
            if n > 4 * mx + 2 * mx * m && over.is_empty() {
                over = format!("{} replacements in {}..{}, {} changes", n, rt[lo].0, rt[hi].0, m);
            }
            most = most.max(n);
            most_ep = most_ep.max(ne);
        }
        if lit {
            s.fail("lit FF1 R-1268 shorter stale_cooldown, 2 × max replacements per span", eg);
        }
        if !over.is_empty() {
            s.fail("I4 span replacements, FF1", over);
        }
        if !ep_over.is_empty() {
            s.fail("I3 span, an epoch's replacements, R-0981", ep_over);
        }
        if !lit_c.is_empty() {
            s.fail("lit FH2 I4 span replacements, per change committed from W before", lit_c);
        }
        if !lit_ce.is_empty() {
            s.fail("lit FH2 R-0981 span, an epoch's, per change committed from W before", lit_ce);
        }
        if !rt.is_empty() {
            let (ne, na) = (rt.iter().filter(|x| x.1 == 0).count() as u64, rt.iter().filter(|x| x.1 == 1).count() as u64);
            s.inc_by("FF1 replacements: an epoch's", ne);
            s.inc_by("FF1 replacements: a replace action's", na);
            for (k, v) in [("FF1 most replacements in a span (max)", most), ("FF1 most of an epoch's in a span (max)", most_ep)] {
                let e = s.cnt.entry(k).or_insert(0);
                *e = (*e).max(v);
            }
        }
    }
}
