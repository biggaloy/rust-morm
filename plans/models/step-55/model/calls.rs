// Checkouts, the call shapes, the world's events and the invariants checked at delivery.
// A check's name starting "lit " is T-0319's sentence as 93854c2 has it, read literally;
// "ez " is the review's recommended sentence; "new " is the final one.
use crate::core::*;
use crate::machine::*;

pub async fn checkout(p: usize, _other: bool) -> Conn {
    checkout_but(p, 0, false).await
}
// a checkout that takes a connection other than `own` when one is idle; `holding`: its call keeps one meanwhile
pub async fn checkout_but(p: usize, own: u64, holding: bool) -> Conn {
    let mut conn;
    loop {
        let (idle, can) = {
            let s = w();
            (s.pools[p].idle.len(), s.pools[p].slots < s.c.max && s.pools[p].open < s.c.budget)
        };
        if idle > 0 {
            let s = w();
            let mut k = s.rng.n(idle as u64) as usize;
            if s.pools[p].idle[k].id == own && idle > 1 && s.c.fault == 44 {
                s.fail("I12 its own connection again, with another idle", String::new());
            } else if s.pools[p].idle[k].id == own && idle > 1 {
                k = (k + 1) % idle;
            }
            conn = s.pools[p].idle.swap_remove(k);
            break;
        }
        if can {
            w().pools[p].slots += 1;
            match open_try(p).await {
                Ok(o) => {
                    conn = mk_conn(p, o);
                    break;
                }
                Err(_) => {
                    w().pools[p].slots -= 1;
                    sleep(20 * MS).await;
                    continue;
                }
            }
        }
        if holding {
            w().fail("I12 a call waits for a connection while it holds one", String::new());
        }
        if w().pools[p].slots < w().c.max {
            // the budget is full for the moment: an extra is open
            sleep(2 * MS).await;
            continue;
        }
        wait(p).await;
    }
    machine_checkout(p, &mut conn).await;
    conn
}
pub async fn machine_checkout(p: usize, conn: &mut Conn) {
    loop {
        let won = {
            let s = w();
            let now = s.now;
            let open = s.pools[p].win.is_none_or(|x| now >= x.2);
            s.pools[p].co_t.push(now);
            let mut won = false;
            if s.c.fault == 21 {
                if s.pools[p].due != Act::None && open {
                    s.cas21(p);
                }
            } else if s.c.fault == 20 {
                won = s.pools[p].due != Act::None && open;
            } else if s.pools[p].due != Act::None && !s.pools[p].starting && open {
                s.pools[p].starting = true;
                if s.c.fault == 23 {
                    s.pools[p].sgen += 1;
                }
                won = true;
            }
            // I2: at the first checkout after its window ends a deferred request is taken
            let starting = s.pools[p].starting;
            let starting = starting || (s.c.fault == 20 && won);
            let bad = s.pools[p].owed.iter().any(|o| !o.2 && o.1 <= now) && !starting;
            let lit = s.pools[p].owed.iter().any(|o| o.2 && o.1 <= now);
            if bad {
                s.fail("I2", format!("{}", now));
            }
            if lit {
                s.fail("lit I2 (an adoption took its place)", format!("{}", now));
            }
            s.pools[p].owed.retain(|o| o.1 > now || (!o.2 && starting));
            won
        };
        if won {
            let (d, due) = {
                let s = w();
                let lw = s.c.lockwait;
                (s.rng.r(0, lw), s.pools[p].due)
            };
            sleep(d).await;
            let s = w();
            s.begin_step(p);
            if s.c.fault == 20 {
                let (n, sn, ri) = (s.pools[p].n, s.pools[p].snap, s.pools[p].rises);
                s.do_take_at(p, due, n, sn, ri, true);
            } else {
                s.finish_take(p);
            }
        }
        let (sg, cg, pooled) = {
            let s = w();
            (s.pools[p].sgen, s.pools[p].cgen, s.c.pooled)
        };
        if conn.cgen < cg && !pooled {
            w().pools[p].open -= 1;
            {
                let s = w();
                let now = s.now;
                s.pools[p].sopens.push(now);
                *s.pools[p].cg_repl.entry(cg).or_insert(0) += 1;
            }
            match open_try(p).await {
                Ok(o) => {
                    *conn = mk_conn(p, o);
                    continue;
                }
                Err(_) => {
                    // the slot stays this call's; the pool's backoff
                    loop {
                        sleep(20 * MS).await;
                        if w().pools[p].open >= w().c.budget {
                            continue;
                        }
                        if let Ok(o) = open_try(p).await {
                            *conn = mk_conn(p, o);
                            break;
                        }
                    }
                    continue;
                }
            }
        }
        if conn.cc != (sg, cg) {
            let s = w();
            if s.c.pooled { conn.cache.clear() } else { s.sess[conn.sess].copies.clear() }
            conn.cc = (sg, cg);
            conn.sgen = sg;
        }
        return;
    }
}
pub fn release(p: usize, conn: Conn) {
    let s = w();
    if conn.id == 0 {
        s.pools[p].slots -= 1;
    } else {
        s.pools[p].idle.push(conn);
    }
    s.wake(p);
}
fn release_held(p: usize, conn: Conn) {
    let s = w();
    if conn.id != 0 {
        s.pools[p].open -= 1;
    }
    s.pools[p].slots -= 1;
    s.wake(p);
    if let Some(ep) = conn.mark {
        s.inc("marked held releases");
        *s.pools[p].ep_repl.entry(ep).or_insert(0) += 1;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum At {
    Out,
    Held,
    HeldApp,
    Tx { retry: u32 },
    Sp { retry: u32 },
}

fn bound_lit(s: &Sim) -> u64 {
    s.db.last_change.max(s.db.last_refusal_end) + s.c.w
}
fn loud_lit(s: &Sim, sp: &Spec, o: &Out, v: u32) -> bool {
    let since = s.db.types[v as usize].1;
    let newlab = matches!(sp.label, Label::New(x) if x > v && x <= s.cur_ver()) || matches!(sp.label, Label::Newest);
    matches!(o.cause, Cause::Kept(_)) && o.comp.is_some_and(|c| c.at <= since) && matches!(sp.stmt, F | TRIG) && newlab && o.sess_opened <= since
}

// an error returned as it is
pub fn check_pg(p: usize, cx: &Cx, sp: &Spec, o: &Out, at: At) {
    let s = w();
    let now = s.now;
    let err = o.r.unwrap_err();
    if o.cause == Cause::Inj {
        if matches!(at, At::Sp { .. }) && err == Err::S40 {
            s.fail("lit I11 (a serialization failure returned as it is fails the whole transaction)", String::new());
        }
        return;
    }
    if cx.rolled && matches!(at, At::Sp { .. }) {
        s.fail("I11 as it is after a whole rollback", String::new());
    }
    if matches!(at, At::Sp { .. }) && cx.reads > 0 && err != Err::A0 {
        s.fail("I11 read", String::new());
    }
    let vis = s.visible(o);
    let pooled = s.c.pooled;
    let wd = s.c.w;
    let held = matches!(at, At::Held | At::HeldApp);
    let app = at == At::HeldApp;
    let then = now >= bound_lit(s);
    let xx = app && err == Err::XX && cx.cached_mode;
    let notold = cx.tested == Some(false);
    let d = format!("t={} stmt={} {:?} {:?} {:?} tested={:?} ptest={}", now, sp.stmt, err, o.cause, at, cx.tested, cx.ptest);
    // ---- I14 with pending entries off (twenty-third review, FA2): every 22P02 the second row takes inside a
    // transaction then returns Error::SchemaChanged at every such call, in both modes, so with the caches on none comes
    // back as it is from an attempt before one that returned Error::SchemaChanged; .retry(n)'s later attempts, which run
    // unnamed, return it as it is (checked in call_tx)
    if matches!(at, At::Tx { .. } | At::Sp { .. }) && matches!(err, Err::P22 { ret: false, .. }) {
        let off = s.c.pend_max == 0 || s.c.e == 0;
        if off && s.c.caches && !cx.retried {
            s.fail("I14 entries off: a 22P02 the second row takes returned as it is before a SchemaChanged", d.clone());
        }
        if off && cx.retried {
            s.inc("entries off: a 22P02 as it is on a later attempt");
        }
        // the twenty-fourth review's FB3: with entries on, while a statement's entry lasts its runs are unnamed, so a
        // bad label's 22P02 inside a savepoint comes as it is, which the savepoint clears; I14 fails the other outcome
        if !off && matches!(at, At::Sp { .. }) && !cx.cached_mode && !cx.retried {
            s.inc("a 22P02 the second row takes, as it is in a savepoint while its entry lasts");
        }
    }
    // ---- I9's loud half ----
    if let Err::P22 { named: Named::Kept(v), .. } = err {
        let since = s.db.types[v as usize].1;
        if !loud_lit(s, sp, o, v) {
            s.fail("lit I9 loud half", d.clone());
            if !vis {
                s.inc("lit I9 loud: not visible");
            } else if app && castlike(sp.stmt) {
                s.inc("lit I9 loud: cast in the app's transaction");
            } else {
                s.inc("lit I9 loud: other");
            }
        }
        if vis {
            let cp = o.copy.unwrap_or(0);
            let ez = loud_lit(s, sp, o, v) || (app && (casts(sp.stmt) || castlike(sp.stmt)) && cx.cached_mode && o.copy_at <= since && s.seen_copy.insert(cp));
            if !ez {
                s.fail("new I9 loud half", d.clone());
            }
            if loud_lit(s, sp, o, v) {
                s.inc("kept type's errors");
                if held {
                    s.inc("kept type's errors, held");
                }
                if then {
                    s.inc("kept type's errors from tc+W on");
                }
            } else {
                s.inc("cast's 22P02 naming the kept type in the app's transaction");
            }
        }
        if !casts(sp.stmt) && (cx.reqs > 0 || cx.made_entry || cx.replaced) {
            s.fail("lit I9 loud: a request, entry or replacement", format!("{} reqs={} entry={} repl={} other={} confirm={}", d, cx.reqs, cx.made_entry, cx.replaced, cx.other, cx.confirm_run));
        }
    }
    match o.cause {
        Cause::Stmt(_) if vis => {
            // the case given up: a kept type renamed by hand without the reset
            if let Err::P22 { named: Named::Ret(v), .. } = err {
                if let Some(t0) = s.noreset_at.get(&v).copied() {
                    s.inc("a stale cast over a type renamed without the reset, visible");
                    if !pooled {
                        if cx.ptest {
                            return;
                        }
                        if !notold && now >= s.db.last_refusal_end + wd {
                            s.fail("R-1271 given up, not judged not old", d.clone());
                        }
                        if now >= t0 + wd && now >= s.db.last_refusal_end + wd && now >= s.db.refused_until {
                            s.fail("R-1271 given up, from t0+W on", d);
                        }
                    } else {
                        if cx.ptest {
                            // the parameter test's, as straight to Postgres: a call whose own bound text holds
                            // _morm_retired_, met now that a statement converting a column's text takes the row
                            s.inc("given up, pooled, by the parameter test");
                            return;
                        }
                        match at {
                            At::Out | At::Held => s.inc("given up, pooled, outside a transaction"),
                            At::Tx { retry: 0 } | At::Sp { retry: 0 } => s.inc("given up, pooled, in a transaction with no retry"),
                            _ => s.inc("given up, pooled, in a transaction under retry"),
                        }
                        let b = cx.att.min(cx.started);
                        if (s.db.last_reconnect > t0 && b > s.db.last_reconnect) || b > t0 + 4 * S {
                            s.fail("R-1271 given up, pooled, after RECONNECT or server_lifetime", d.clone());
                        }
                        if !cx.cool {
                            s.fail("R-1271 given up, pooled, not from the cool-down", d);
                        }
                    }
                    return;
                }
            }
            match at {
                At::Out | At::Held => s.fail("I7 outside a transaction", d),
                At::HeldApp => {
                    s.inc("stale statement's error in the app's transaction");
                    if let Some(id) = o.copy {
                        if !s.app_copy.insert(id) {
                            s.fail("I7 held, a stale copy failing twice", d);
                        }
                    }
                }
                _ => s.fail("I7 in a transaction, as it is", d),
            }
        }
        Cause::Path if vis => match at {
            At::Out | At::Held => s.fail("I8 search_path visible outside", d),
            At::HeldApp => s.inc("search_path's 0A000 in the app's transaction"),
            _ => s.fail("I8 search_path as it is in a transaction", d),
        },
        Cause::Sess(t0) => {
            if pooled {
                if vis {
                    if matches!(at, At::Out) && !cx.cool && !cx.other && !cx.ptest {
                        s.fail("I8 pooled, outside a transaction, as it is with no cool-down and no retry", d.clone());
                    }
                    s.inc("stale sessions' errors as they are, pooled");
                    if s.db.last_reconnect > t0 && now > s.db.last_reconnect {
                        s.fail("lit I8 never visible after RECONNECT", d.clone());
                        if cx.att.min(cx.started) > s.db.last_reconnect {
                            s.fail("new I8 never visible after RECONNECT", d.clone());
                        }
                    }
                    if now > t0 + 4 * S {
                        s.fail("lit I8 never visible after server_lifetime", d.clone());
                        if cx.att.min(cx.started) > t0 + 4 * S {
                            s.fail("new I8 never visible after server_lifetime", d);
                        }
                    }
                }
                return;
            }
            if held {
                // T-0319 at 93854c2, read literally
                if then {
                    if !xx {
                        s.fail("lit I8 held, from then on", d.clone());
                    }
                } else if !notold {
                    s.fail("lit I8 held, before then", d.clone());
                    if cx.tested.is_none() && xx {
                        let e = o.sess_opened;
                        let _ = e;
                        s.inc("lit held before: the XX000");
                    }
                }
                // the review's sentence
                if !xx {
                    if then {
                        if vis {
                            s.fail("ez I8 held, from then on", d.clone());
                        }
                    } else if !notold {
                        s.fail("ez I8 held, before then, every error", d.clone());
                        if vis {
                            s.fail("ez I8 held, before then, visible errors", d.clone());
                        }
                    }
                }
                // the final sentence
                if !xx && vis && !cx.ptest && (then || !notold) {
                    s.fail(if then { "new I8 held, from then on" } else { "new I8 held, before then" }, d.clone());
                }
                if vis {
                    if xx {
                        s.inc("XX000 in the app's transaction");
                        if then {
                            s.inc("XX000 in the app's transaction from tc+W on");
                        }
                    } else if then {
                        s.inc("held visible stale session from tc+W on");
                    } else {
                        s.inc("held not old, before tc+W");
                    }
                }
                return;
            }
            if !vis {
                return;
            }
            s.inc("stale sessions' errors as they are");
            if then {
                if cx.ptest {
                    s.fail("lit I8 visible from then on (parameter test)", d.clone());
                } else {
                    s.fail(if matches!(at, At::Out) { "I8 outside a transaction" } else if matches!(at, At::Sp { .. }) { "I8 in a savepoint" } else { "I8 in a transaction" }, d.clone());
                }
            }
            if now >= t0 + wd && now >= s.db.last_refusal_end + wd && now >= s.db.refused_until && !cx.ptest {
                s.fail("R-1271 from t0+W on", d.clone());
            }
            if !notold && now >= s.db.last_refusal_end + wd {
                if cx.ptest {
                    s.fail("lit R-1271 before t0+W, not judged not old (parameter test)", d);
                } else {
                    s.fail("R-1271 before t0+W, not judged not old", d);
                }
            }
        }
        _ => {}
    }
}
pub fn check_ok(p: usize, cx: &Cx, sp: &Spec, o: &Out, _at: At) {
    let s = w();
    if !matches!(sp.stmt, CAST | F | ST | STC | NOC | TWO) {
        return;
    }
    let v = match o.r {
        Ok(v) => v,
        _ => return,
    };
    if o.oracle == Ok(v) {
        return;
    }
    let (st, since, left) = s.db.types[v as usize];
    // judged at the run's own instant: a change since can't have helped it
    let at = if castlike(sp.stmt) { o.copy_at } else { o.comp.map_or(0, |c| c.at) };
    let d = format!("t={} run={} since={} left={} at={} stmt={} ver={} state={:?} label={:?}", s.now, o.t, since, left, at, sp.stmt, v, st, sp.label);
    if st == TS::Retired && matches!(sp.label, Label::RetReal) {
        // only text copied from the retired type's own labels, through plpgsql compiled before the relabel
        if !s.c.retired_real || s.c.fault == 40 {
            s.fail("I9 a renamed label guessed", d);
        } else if !o.comp.is_some_and(|c| c.at <= since) {
            s.fail("I9 a copied label, not plpgsql compiled before the relabel", d);
        } else {
            s.inc("copied labels taken");
        }
        return;
    }
    if !(left <= o.t && at <= left && (st == TS::Kept || since >= o.t)) {
        s.fail("I9 silent", d);
        return;
    }
    s.inc("silent wrong answers");
    if castlike(sp.stmt) && o.cached {
        if let Some((c, _)) = s.pools[p].confs.iter().find(|c| c.0 > left) {
            if o.t > c + s.c.w {
                let dd = format!("{} run={} left={} conf={} copy_at={} cid={:?} acts={:?} adopts={:?}", d, o.t, left, c, o.copy_at, o.copy, s.pools[p].acts.iter().map(|a| (a.t, a.kind)).filter(|a| a.0 > left).take(4).collect::<Vec<_>>(), s.pools[p].adopts.iter().filter(|a| **a > left).take(4).collect::<Vec<_>>());
                s.fail("lit I9 silent past W of the first confirmation", dd);
                let f = s.pools[p].req_t.iter().find(|c| **c > left);
                // the first checkout after that, its take finished
                if f.is_some_and(|f| { let e = f + s.c.w; let k = s.pools[p].co_t.partition_point(|x| *x <= e); k < s.pools[p].co_t.len() && s.pools[p].co_t[k] + s.c.lockwait < cx.started }) {
                    if o.copy_by == p {
                        let dd = format!("{} run={} left={} req={:?} acts={:?} adopts={:?} copy_at={}", d, o.t, left, f, s.pools[p].acts.iter().map(|a| a.t).filter(|a| *a > left).take(4).collect::<Vec<_>>(), s.pools[p].adopts.iter().filter(|a| **a > left).take(4).collect::<Vec<_>>(), o.copy_at);
                        s.fail("new I9 silent past W of the first request", dd);
                    } else {
                        s.fail("mid I9 silent past W of the first request, another process's copy", d);
                    }
                }
            }
        }
    }
}
// Error::SchemaChanged
pub fn check_sc(p: usize, cx: &Cx, sp: &Spec, o: &Out, at: At, conn: u64) {
    let s = w();
    let now = s.now;
    let d = format!("t={} stmt={} {:?} {:?}", now, sp.stmt, o.cause, at);
    let retry0 = matches!(at, At::Tx { retry: 0 } | At::Sp { retry: 0 });
    // as decided, every cached statement's 22P02 from enum_in takes the second row, so a bad label of a cast that
    // isn't stale gets Error::SchemaChanged first inside a transaction: counted, and I14 below bounds it
    if matches!(o.r, Err(Err::P22 { named: Named::Cur, .. })) && casts(sp.stmt) && o.comp.is_none() {
        s.inc(if sp.stmt == TWO { "a bad label beside a second type: SchemaChanged" } else { "a bad label of a lone cast: SchemaChanged" });
    }
    // the two answers' cost: a bad label or bad stored text the second row takes, a kept type's stale function's
    // error among them, gets Error::SchemaChanged first, and never from a run begun while its statement's entry
    // exists, so at most once per E per statement beyond calls already running, and once more after anything that
    // ends the entry sooner
    if matches!(o.r, Err(Err::P22 { ret: false, .. })) && !matches!(o.cause, Cause::Stmt(_) | Cause::Sess(_) | Cause::Path) {
        s.inc(if matches!(o.cause, Cause::Kept(_)) { "SchemaChanged for a kept function's error" } else { "SchemaChanged for bad text" });
        if retry0 {
            s.inc("SchemaChanged for bad text or a kept function's error, no retry");
            let (st, e) = (cx.started, s.c.e);
            let cr = s.pools[p].created.get(&sp.stmt).cloned().unwrap_or_default();
            if let Some(c) = cr.iter().rev().find(|c| **c < st) {
                let rem = s.pools[p].removals.get(&sp.stmt).cloned().unwrap_or_default();
                if st < c + e && !rem.iter().any(|r| *r >= *c && *r <= st) {
                    s.fail("I14 the second row's cost: a bad text's SchemaChanged in a run begun while its entry exists", d.clone());
                }
            }
        }
    }
    match o.cause {
        Cause::Sess(t0) => {
            if !s.c.pooled && retry0 && now >= bound_lit(s) {
                let c = s.connfail.entry(conn).or_insert(0);
                *c += 1;
                if *c > 1 {
                    s.fail("I8 once per connection", d.clone());
                }
            }
            if s.c.pooled {
                if s.db.last_reconnect > t0 && now > s.db.last_reconnect {
                    s.fail("lit I8 never visible after RECONNECT", d.clone());
                    if cx.att.min(cx.started) > s.db.last_reconnect {
                        s.fail("new I8 never visible after RECONNECT", d.clone());
                    }
                }
                if now > t0 + 4 * S {
                    s.fail("lit I8 never visible after server_lifetime", d.clone());
                    if cx.att.min(cx.started) > t0 + 4 * S {
                        s.fail("new I8 never visible after server_lifetime", d);
                    }
                }
            }
        }
        Cause::Stmt(_) | Cause::Path if retry0 => {
            if !s.c.pooled && !s.c.caches {
                s.fail("a stale statement with the caches off", d.clone());
            }
            let path = o.cause == Cause::Path;
            if !path && !s.c.pooled {
                return;
            }
            if !path && o.r == Err(Err::XX) && sp.stmt == F {
                return;
            }
            let e = s.c.e;
            let v = s.pools[p].fails_tx.entry((sp.stmt, path)).or_default().clone();
            let prev = v.iter().rev().find(|x| now - x.0 < e).copied();
            if let Some((tf, _)) = prev {
                let act = s.pools[p].acts.iter().any(|a| a.t > tf && a.t <= now);
                let lit = if path { cx.started > tf && !act } else { true };
                if lit {
                    s.fail(if path { "lit I8 search_path once per E" } else { "lit I8 stale statement once per E" }, d.clone());
                }
                if !path && cx.started > tf && !act {
                    s.fail("mid I8 stale statement once per E, running calls and actions apart", d.clone());
                }
            }
            // final: a run begun while the statement's entry exists never fails so
            let st = cx.started;
            let cr = s.pools[p].created.get(&sp.stmt).cloned().unwrap_or_default();
            if let Some(c) = cr.iter().rev().find(|c| **c < st) {
                let rem = s.pools[p].removals.get(&sp.stmt).cloned().unwrap_or_default();
                if st < c + e && !rem.iter().any(|r| *r >= *c && *r <= st) {
                    s.fail("new I8 a failure begun while its entry exists", d);
                }
            }
            s.pools[p].fails_tx.entry((sp.stmt, path)).or_default().push((now, cx.started));
        }
        _ => {}
    }
}

pub async fn call_out(p: usize, sp: Spec) {
    w().inc("calls");
    let mut conn = checkout(p, false).await;
    let mut cx = Cx { out: true, att: w().now, ..Cx::default() };
    let g = {
        let s = w();
        let g = s.c.co_gap;
        s.rng.r(0, g)
    };
    if g > 0 {
        sleep(g).await;
    }
    let r = run_call(p, &mut conn, &mut cx, &sp).await;
    if cx.runs > 3 {
        w().fail("R-1271 more than three runs of a call", String::new());
    }
    if cx.reads > 1 {
        w().inc("calls outside with two reads");
        if !cx.confirm_run && !cx.fresh_sess {
            w().fail("R-1271 more than one catalog read", String::new());
        }
        if cx.reads > 2 {
            w().fail("R-1271 more than two catalog reads", String::new());
        }
    }
    deliver(p, &cx, &sp, &r, At::Out, conn.id);
    release(p, conn);
}
fn deliver(p: usize, cx: &Cx, sp: &Spec, r: &Res, at: At, conn: u64) {
    match r {
        Res::Ok(o) => check_ok(p, cx, sp, o, at),
        Res::Pg(o) => check_pg(p, cx, sp, o, at),
        Res::Sc(o) => check_sc(p, cx, sp, o, at, conn),
    }
}

pub async fn call_held(p: usize, sps: Vec<Spec>, app: bool) {
    let g = w().c.held_gap;
    call_held_gap(p, sps, app, g).await
}
pub async fn call_held_gap(p: usize, sps: Vec<Spec>, app: bool, gap: u64) {
    w().inc("calls");
    w().inc("held calls");
    if app {
        w().inc("held calls in the app's transaction");
    }
    let mut conn = checkout(p, false).await;
    let mut cx = Cx { out: true, held: true, app, rr: app, att: w().now, ..Cx::default() };
    for (i, sp) in sps.iter().enumerate() {
        let mut sp = *sp;
        if !app {
            sp.path = 0;
        }
        let sp = &sp;
        if i > 0 {
            let g = {
                let s = w();
                s.rng.r(0, gap)
            };
            sleep(g).await;
        }
        loop {
            let r = run_call(p, &mut conn, &mut cx, sp).await;
            deliver(p, &cx, sp, &r, if app { At::HeldApp } else { At::Held }, conn.id);
            if app && !matches!(r, Res::Ok(_)) && {
                let s = w();
                let x = s.c.held_rerun;
                s.rng.p(x)
            } {
                // ROLLBACK, BEGIN, and the statement again
                sleep(MS).await;
                cx.app_failed = false;
                cx.tx_snap = None;
                w().inc("held reruns");
                continue;
            }
            break;
        }
    }
    release_held(p, conn);
}

pub async fn call_tx(p: usize, sps: Vec<(Spec, bool)>, retry: u32, rr: bool) {
    w().inc("calls");
    w().inc("transactions");
    let mut conn = checkout(p, false).await;
    let mut retried = false;
    let mut due_mark = None;
    let mut stmt_attempts = 0;
    let mut sess_attempts = 0;
    let mut need = 0u32;
    let g = {
        let s = w();
        let g = s.c.co_gap;
        s.rng.r(0, g)
    };
    if g > 0 {
        sleep(g).await;
    }
    if retry == 0 && sps.iter().any(|(x, _)| x.stmt == PATH) {
        w().inc("search_path: transactions, no retry");
    }
    for attempt in 0..=retry {
        let pooled = w().c.pooled;
        let mut cx = Cx { out: false, rr, retried, ..Cx::default() };
        cx.due_mark = due_mark.take();
        if pooled {
            let j = crate::calls::acq(conn.last_srv).await;
            conn.last_srv = j;
            cx.srv = Some(j);
        }
        cx.att = w().now;
        let mut end: Option<(Res, bool, Spec)> = None;
        for (i, (sp, insp)) in sps.iter().enumerate() {
            if i > 0 {
                let g = {
                    let s = w();
                    let g = s.c.gap2;
                    s.rng.r(0, g)
                };
                sleep(g).await;
            }
            // an attempt may bind other values
            let sp = {
                let s = w();
                if attempt > 0 && !s.c.one_value && s.rng.p(0.25) { crate::world::respec(s, *sp) } else { *sp }
            };
            cx.sp = *insp;
            cx.rolled = false;
            let r = run_call(p, &mut conn, &mut cx, &sp).await;
            match &r {
                Res::Ok(o) => check_ok(p, &cx, &sp, o, At::Tx { retry }),
                Res::Pg(o) => {
                    if cx.reads > 1 {
                        w().fail("R-1271 more than one catalog read in a transaction", String::new());
                    }
                    check_pg(p, &cx, &sp, o, if *insp { At::Sp { retry } } else { At::Tx { retry } });
                    if o.r == Err(Err::S40) {
                        end = Some((r, true, sp));
                        break;
                    }
                    if *insp && {
                        let s = w();
                        s.rng.p(0.5)
                    } {
                        w().inc("errors a savepoint cleared");
                        continue;
                    }
                    end = Some((r, false, sp));
                    break;
                }
                Res::Sc(_) => {
                    end = Some((r, true, sp));
                    break;
                }
            }
        }
        if let Some(j) = cx.srv {
            crate::calls::rel(j);
        }
        match end {
            None => break,
            Some((r, again, sp)) => {
                let mut last = !again || attempt == retry;
                if let Res::Sc(o) = &r {
                    if !pooled && matches!(o.cause, Cause::Stmt(_)) {
                        if let Some(id) = o.copy {
                            if o.cached && !w().sc_copy.insert(id) {
                                w().fail("R-1271 a stale copy failing twice", String::new());
                            }
                        }
                    }
                    match o.cause {
                        Cause::Stmt(_) | Cause::Path if !matches!(o.cause, Cause::Sess(_)) => {
                            stmt_attempts += 1;
                            if stmt_attempts > 1 {
                                w().fail("I7 stale statements on more than one attempt", format!("t={} stmt={}", w().now, sp.stmt));
                            }
                        }
                        Cause::Sess(_) => sess_attempts += 1,
                        _ => {}
                    }
                    if retry == 0 {
                        check_sc(p, &cx, &sp, o, At::Tx { retry }, conn.id);
                    }
                    // FB1's counts: a search_path switch's SchemaChanged with no .retry(n), with entries on and off
                    if o.cause == Cause::Path && retry == 0 {
                        let s = w();
                        s.inc("search_path: SchemaChanged, no retry");
                        if s.c.pend_max == 0 || s.c.e == 0 {
                            s.inc("entries off: search_path SchemaChanged, no retry");
                        }
                    }
                    // I14's later attempts: run unnamed, they return a bad text's or a kept function's 22P02 as it is
                    if cx.retried && matches!(o.r, Err(Err::P22 { ret: false, .. })) && !matches!(o.cause, Cause::Stmt(_) | Cause::Sess(_) | Cause::Path) {
                        w().fail("I14 a later attempt's SchemaChanged for a 22P02 the second row takes", format!("t={} stmt={} {:?}", w().now, sp.stmt, o.cause));
                    }
                    if !cx.retried && matches!(o.r, Err(Err::P22 { ret: false, .. })) && !matches!(o.cause, Cause::Stmt(_) | Cause::Sess(_) | Cause::Path) {
                        let s = w();
                        if s.c.pend_max == 0 || s.c.e == 0 {
                            s.inc("entries off: SchemaChanged for bad text or a kept function's error");
                        }
                    }
                    retried = w().c.fault != 29 || true;
                    if w().c.fault == 29 {
                        retried = true;
                    }
                    need += 1;
                    due_mark = cx.mark_next.take();
                    if let Some(ep) = cx.discard {
                        // the transaction's connection closes
                        {
                            let s = w();
                            s.pools[p].open -= 1;
                            *s.pools[p].ep_repl.entry(ep).or_insert(0) += 1;
                        }
                        if last {
                            conn.id = 0;
                        } else {
                            {
                                let s = w();
                                let now = s.now;
                                s.pools[p].sopens.push(now);
                                *s.pools[p].ep_open.entry(ep).or_insert(0) += 1;
                            }
                            match open_try(p).await {
                                Ok(o) => conn = mk_conn(p, o),
                                Err(_) => {
                                    let s = w();
                                    s.begin_step(p);
                                    if let Some(e) = s.pools[p].epoch.as_mut() {
                                        if e.id == ep {
                                            e.susp = true;
                                        }
                                    }
                                    conn.id = 0;
                                    last = true;
                                }
                            }
                        }
                    }
                } else if again {
                    need += 1;
                    if w().c.fault == 29 {
                        retried = false;
                    }
                }
                if last {
                    break;
                }
            }
        }
    }
    let _ = (need, sess_attempts);
    release(p, conn);
}
pub async fn acq(prefer: usize) -> usize {
    acquire(prefer).await
}
pub fn rel(j: usize) {
    release_srv(j)
}
