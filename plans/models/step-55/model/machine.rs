// R-1270's procedure, step by step, and rows A to D.
use crate::core::*;
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct Cx {
    pub out: bool,
    pub held: bool,
    pub sp: bool,
    pub app: bool,
    pub app_failed: bool,
    pub rr: bool,
    pub tx_snap: Option<u64>,
    pub rolled: bool,
    pub retried: bool,
    pub srv: Option<usize>,
    pub discard: Option<u64>, // the epoch whose test decided it
    pub mark_next: Option<(Spec, u8)>, // row B2's or B3's mark, for the transaction's next attempt
    pub due_mark: Option<(Spec, u8)>,  // the same, on this attempt
    pub cached_mode: bool,
    pub conn: u64,
    pub nomark: bool,
    // what the last call did, for the checks
    pub tested: Option<bool>,
    pub reqs: u32,
    pub made_entry: bool,
    pub replaced: bool,
    pub reads: u32,
    pub cool: bool,
    pub other: bool,
    pub confirm_run: bool,
    pub fresh_sess: bool,
    pub runs: u32,
    pub epoch_t: Option<u64>,
    pub ptest: bool,
    pub started: u64,
    pub att: u64,
    pub first: Option<Result<u32, Err>>,
    pub carry: i64,
    pub tx_t: u64, // the twenty-sixth review's FD1: when the transaction began, before its checkout
}
impl Cx {
    pub fn fresh_call(&mut self) {
        self.tested = None;
        self.reqs = 0;
        self.made_entry = false;
        self.replaced = false;
        self.reads = 0;
        self.cool = false;
        self.other = false;
        self.confirm_run = false;
        self.fresh_sess = false;
        self.runs = 0;
        self.epoch_t = None;
        self.ptest = false;
        self.started = w().now;
        self.first = None;
        self.carry = w().pools[0].n.max(0);
    }
}
#[derive(Clone, Debug)]
pub enum Res {
    Ok(Out),
    Pg(Out),
    Sc(Out),
}
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Cached(Text),
    Keep(Text),
    Unnamed,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cls {
    None,
    Stmt,
    Sess,
}

impl Sim {
    pub fn begin_step(&mut self, p: usize) {
        let q = &mut self.pools[p];
        q.step_n0 = q.n;
        q.step_snap = q.snap;
        q.step_rise = q.rises;
        let now = self.now;
        q.entries.retain(|e| e.expiry > now);
        q.cool.retain(|e| e.1 > now);
    }
    fn win_open(&self, p: usize) -> bool {
        match self.pools[p].win {
            None => true,
            Some((_, _, end)) => self.now >= end,
        }
    }
    // a checkout's compare-and-swap landing between a step's reading of the word and its own
    // fault 21: the checkout's compare-and-swap takes the bits and raises the generation, and only then, later, starts the window
    pub fn cas21(&mut self, p: usize) {
        let (a, n, sn, ri) = {
            let q = &self.pools[p];
            (q.due, q.n, q.snap, q.rises)
        };
        self.pools[p].due = Act::None;
        self.rec_act(p, a, n, sn, ri);
        let (lw, wd) = (self.c.lockwait, self.c.w);
        let d = self.rng.r(0, lw);
        let at = self.now + d;
        self.spawn(
            Box::pin(async move {
                let s = w();
                s.begin_step(p);
                if a == Act::Replace { s.pools[p].cgen += 1 } else { s.pools[p].sgen += 1 }
                let (t, n) = (s.now, s.pools[p].n);
                s.pools[p].win = Some((t, n, t + wd));
                s.clear_tables(p);
            }),
            at,
        );
    }
    fn landing(&mut self, p: usize) -> bool {
        self.landing_at(p, false)
    }
    fn landing_at(&mut self, p: usize, entry: bool) -> bool {
        if self.pools[p].due != Act::None && !self.pools[p].starting && self.win_open(p) && self.c.land > 0.0 && {
            let l = self.c.land;
            self.rng.p(l)
        } {
            if self.c.fault == 22 {
                // the step's plain store overwrites the checkout's starting bit
                if entry && self.pools[p].owed.iter().any(|o| !o.2 && o.1 <= self.now) {
                    self.fail("I2", "a checkout's starting bit overwritten".into());
                }
                return false;
            }
            if self.c.fault == 21 {
                self.cas21(p);
                return true;
            }
            self.pools[p].starting = true;
            if self.c.fault == 23 {
                self.pools[p].sgen += 1;
            }
            self.inc("landings");
            let lw = self.c.lockwait;
            let d = self.rng.r(0, lw);
            let at = self.now + d;
            self.spawn(
                Box::pin(async move {
                    let s = w();
                    s.begin_step(p);
                    s.finish_take(p);
                }),
                at,
            );
            return true;
        }
        false
    }
    pub fn rec_act(&mut self, p: usize, a: Act, n: i64, snap: u64, rise: u64) {
        let now = self.now;
        let q = &mut self.pools[p];
        let cg = q.cgen;
        q.acts.push(ActRec { t: now, n, snap, snap_lit: q.snap, lr_lit: q.n, rise, rise_lit: q.rises, kind: a, cgen: cg });
        q.acts_t.push(now);
        q.owed.clear();
        self.inc("actions");
    }
    pub fn clear_tables(&mut self, p: usize) {
        self.clear_tables_k(p, 1)
    }
    pub fn clear_tables_k(&mut self, p: usize, k: u8) {
        let now = self.now;
        let cl: Vec<u8> = self.pools[p].cool.iter().filter(|e| e.1 > now).map(|e| e.0).collect();
        for s in cl {
            self.pools[p].cool_rm.entry(s).or_default().push((now, k));
        }
        let st: Vec<u8> = self.pools[p].entries.iter().map(|e| e.stmt).collect();
        for s in st {
            self.pools[p].removals.entry(s).or_default().push(now);
        }
        self.pools[p].entries.clear();
        self.pools[p].cool.clear();
    }
    fn do_take(&mut self, p: usize, a: Act, n: i64, snap: u64, rise: u64) {
        self.do_take_at(p, a, n, snap, rise, false)
    }
    pub fn do_take_at(&mut self, p: usize, a: Act, n: i64, snap: u64, rise: u64, ft: bool) {
        let fault = self.c.fault;
        let salt = self.id();
        let (wd, pooled, now) = (self.c.w, self.c.pooled, self.now);
        let q = &mut self.pools[p];
        if pooled {
            q.marker = (n, salt);
        }
        let raised = ft && fault == 23;
        if a == Act::Replace { q.cgen += 1 } else if !raised { q.sgen += 1 }
        q.due = Act::None;
        q.starting = false;
        q.win = Some((now, n, now + wd));
        self.clear_tables(p);
        self.rec_act(p, a, n, snap, rise);
    }
    pub fn finish_take(&mut self, p: usize) {
        if self.pools[p].starting {
            let (a, n0, sn, ri) = {
                let q = &self.pools[p];
                (q.due, q.step_n0, q.step_snap, q.step_rise)
            };
            if a == Act::None {
                self.pools[p].starting = false;
                return;
            }
            self.do_take_at(p, a, n0, sn, ri, true);
        }
    }
    fn gate_open(&mut self, p: usize) -> bool {
        if self.pools[p].starting {
            self.finish_take(p);
        }
        if self.c.fault == 1 {
            return true;
        }
        self.win_open(p) || self.pools[p].n != self.pools[p].win.unwrap().1
    }
    // a request judged in this step; true when taken
    fn req_step(&mut self, p: usize, a: Act, reading: bool) -> bool {
        loop {
            if self.gate_open(p) {
                if self.landing(p) {
                    self.finish_take(p);
                    continue;
                }
                let (d, n, sn, ri) = {
                    let q = &self.pools[p];
                    (q.due, q.n, q.snap, q.rises)
                };
                self.do_take(p, a.max(d), n, sn, ri);
                return true;
            }
            if reading {
                return false;
            }
            if self.landing(p) {
                self.finish_take(p);
                continue;
            }
            let end = self.pools[p].win.map_or(0, |x| x.2);
            let now = self.now;
            if self.c.fault == 6 {
                self.pools[p].owed.push((now, end, false));
                return false;
            }
            let q = &mut self.pools[p];
            q.due = q.due.max(a);
            q.owed.push((now, end, false));
            self.inc("deferred");
            return false;
        }
    }
    fn on_moved(&mut self, p: usize, r: i64) {
        let (wd, pooled, now) = (self.c.w, self.c.pooled, self.now);
        self.pools[p].n = r;
        let sn = self.pools[p].snap;
        self.pools[p].moved.push((now, r, sn));
        if !pooled && !self.pools[p].entries.is_empty() && self.landing(p) {
            self.finish_take(p);
        }
        self.clear_tables_k(p, 2);
        if pooled {
            let salt = self.id();
            let fr = self.c.fault == 26;
            // a checkout's compare-and-swap landing here takes nothing
            let _ = self.landing(p);
            let q = &mut self.pools[p];
            if r > q.highest {
                q.marker = (r, 0);
                q.highest = r;
            } else {
                q.marker = (r, if fr { 0 } else { salt });
            }
            q.sgen += 1;
            if q.due != Act::None {
                for o in q.owed.iter_mut() {
                    o.2 = true;
                }
            }
            q.due = Act::None;
            q.starting = false;
            q.win = Some((now, r, now + wd));
            q.adopts.push(now);
            q.acts_t.push(now);
            self.inc("adoptions");
        }
    }
    fn arrive(&mut self, p: usize, t: u64, r: i64, snap: u64, sent: u64, over: bool) -> bool {
        let really = self.c.several;
        let several = match self.c.fault { 10 => true, 14 => false, _ => really };
        let lit = self.pools[p].n;
        let fault = self.c.fault;
        if t < self.pools[p].horizon && fault == 13 && r > lit {
            // an overlapping read counts when its <n> is greater
            self.pools[p].rises += 1;
            if snap <= self.pools[p].applied_snap {
                self.fail("I10 newer", String::new());
            }
            self.pools[p].snap = snap;
            self.pools[p].applied_snap = self.pools[p].applied_snap.max(snap);
            self.on_moved(p, r);
            return true;
        }
        if t < self.pools[p].horizon && fault != 9 {
            self.inc("overlapping reads refused");
            return false;
        }
        let mvd = r > lit || (r < lit && !several);
        if fault == 11 {
            self.pools[p].horizon = t + 1;
        } else if fault != 12 || mvd {
            self.pools[p].horizon = self.pools[p].tickets;
        }
        if snap < sent {
            let m = format!("{} snap {} sent {}", self.now, snap, sent);
            self.fail("I10 snapshot", m);
        }
        let mut mv = false;
        if r > lit || (r < lit && !several) {
            if snap <= self.pools[p].applied_snap && lit != r {
                let m = format!("{} older snapshot", self.now);
                self.fail("I10 newer", m);
            }
            if r > lit {
                self.pools[p].rises += 1;
            }
            self.pools[p].snap = snap;
            self.on_moved(p, r);
            mv = true;
        } else if r == lit {
            self.pools[p].snap = snap;
        }
        self.pools[p].applied_snap = self.pools[p].applied_snap.max(snap);
        if !over {
            let n = self.pools[p].n;
            if (!really && n != r) || (really && r > lit && n != r) {
                let m = format!("{} read {} last {}", self.now, r, n);
                self.fail("I10 sets", m);
            }
        }
        if really && self.pools[p].n < lit {
            self.fail("I10 down", String::new());
        }
        mv
    }
    fn n_at(&mut self, snap: u64) -> i64 {
        let hist = if self.c.servers > 1 {
            let k = self.rng.n(self.c.servers as u64) as usize;
            &self.db.replay[k]
        } else {
            &self.db.nhist
        };
        hist.iter().rev().find(|x| x.0 < snap || x.0 == 0).unwrap().1
    }
    fn old(&mut self, p: usize, conn: &Conn, cx: &mut Cx) -> bool {
        let (wd, now) = (self.c.w, self.now);
        let q = &mut self.pools[p];
        let start = match &q.epoch {
            None => true,
            Some(e) => (now >= e.end && self.c.fault != 4) || (e.n != q.n && self.c.fault != 4),
        };
        if start {
            q.epoch_seq += 1;
            q.epoch = Some(Epoch { id: q.epoch_seq, t: now, n: q.n, end: now + wd, susp: false, noextra: false });
            let x = (q.epoch_seq, now, q.snap, q.rises, q.open as u64);
            q.epochs.push(x);
            self.inc("epochs");
        }
        let e = self.pools[p].epoch.as_ref().unwrap();
        let r = conn.opened < e.t && !e.susp;
        let id = e.id;
        cx.discard = Some(id);
        cx.epoch_t = Some(e.t);
        cx.tested = Some(r);
        if self.c.fault == 2 {
            cx.tested = Some(true);
            return true;
        }
        if self.c.fault == 5 && !cx.held {
            return false;
        }
        r
    }
    fn entry(&mut self, p: usize, sp: &Spec, err: Err, marked: bool) {
        let (mut mx, mut e, now) = (self.c.pend_max, self.c.e, self.now);
        // break 124 (twenty-third review, FA2): pending entries off still keeping one, as if the setting were ignored
        if self.c.fault == 124 && (mx == 0 || e == 0) {
            (mx, e) = (64, S);
        }
        if mx == 0 || e == 0 {
            return;
        }
        let xx = err == Err::XX && self.c.fault != 39;
        if let Some(x) = self.pools[p].entries.iter_mut().find(|x| x.stmt == sp.stmt) {
            x.bytes = Some(sp.bytes());
            x.err = err.code();
            x.marked = marked;
            x.xx = xx;
            x.replace = false;
            return;
        }
        if self.landing_at(p, true) {
            self.finish_take(p);
        }
        let id = self.id();
        if self.pools[p].entries.len() >= mx {
            let d = self.pools[p].entries.pop_front().unwrap();
            self.pools[p].removals.entry(d.stmt).or_default().push(now);
        }
        self.pools[p].created.entry(sp.stmt).or_default().push(now);
        self.pools[p].entries.push_back(Entry { stmt: sp.stmt, bytes: Some(sp.bytes()), err: err.code(), marked, xx, replace: false, expiry: now + e, id });
        if self.pools[p].entries.len() > mx {
            self.fail("I6", String::new());
        }
    }
    fn expect_confirm(&mut self, p: usize, sp: &Spec, err: u8, cx: &mut Cx) {
        if let Some(x) = self.pools[p].entries.iter_mut().find(|x| x.stmt == sp.stmt && x.bytes == Some(sp.bytes())) {
            x.replace = true;
        } else {
            cx.mark_next = Some((*sp, err));
        }
    }
    fn suspend(&mut self, p: usize, ep: u64) {
        if let Some(e) = self.pools[p].epoch.as_mut() {
            if e.id == ep {
                e.susp = true;
            }
        }
        self.inc("suspended");
    }
    pub fn check_open(&mut self, p: usize) {
        let (m, b) = (self.c.max, self.c.budget);
        let q = &self.pools[p];
        if q.slots > m || q.open > m + 1 || q.open > b {
            let t = format!("slots {} open {}", q.slots, q.open);
            self.fail("I5", t);
        }
    }
    pub fn class(&self, e: Err, cached: bool, sp: &Spec, out: bool) -> Cls {
        let casts = casts(sp.stmt);
        match e {
            Err::A0 if cached => Cls::Stmt,
            Err::P22 { named, ret, .. } => {
                // as decided (2026-11-26): every cached statement's 22P02 from enum_in takes the second row, known
                // by its SQLSTATE and routine alone; no message text is read. Break 120: only one that casts a
                // parameter takes it. Switch 36: the second row reads the message again, a 22P02 naming a type the
                // statement casts to now being taken for a label's own. Break 123: the procedure at 64820fe, the
                // name test reading a statement that casts one parameter, here the conversion form of statement 10
                // (odd values), and the two-cast form always taken.
                let br = self.c.fault;
                let read = (br == 36 && casts) || (br == 123 && casts && !(sp.stmt == TWO && sp.val % 2 == 0));
                let r2 = cached && (casts || br != 120) && !(read && named == Named::Cur);
                if r2 && self.c.pooled2 {
                    return Cls::Stmt;
                }
                if self.c.fault != 38 && ret {
                    return Cls::Sess;
                }
                if r2 {
                    return Cls::Stmt;
                }
                if ret {
                    return Cls::Sess;
                }
                if cached && casts && out {
                    return Cls::Stmt; // choice I
                }
                if self.c.fault == 33 { Cls::Sess } else { Cls::None }
            }
            Err::XX => {
                if cached { Cls::Stmt } else { Cls::Sess }
            }
            Err::D42804 => Cls::Sess,
            _ => Cls::None,
        }
    }
    fn confirms(&self, r: &Result<u32, Err>, first: u8) -> bool {
        match r {
            Ok(_) => true,
            Err(e) => !e.noconf() && e.code() != first,
        }
    }
}

// ---- I/O ----
pub async fn read(p: usize, cx: &mut Cx, snap_over: Option<u64>) -> bool {
    cx.reads += 1;
    let (lat, off, fail, t, sent) = {
        let s = w();
        s.inc("reads");
        let lat = s.rng.r(MS, 200 * MS);
        let off = s.rng.r(0, lat);
        let rf = s.c.readfail;
        let fail = s.rng.p(rf);
        let t = s.pools[p].tickets;
        s.pools[p].tickets += 1;
        // overlap bookkeeping
        let over = !s.pools[p].inflight.is_empty();
        let fl = s.pools[p].inflight.clone();
        for x in fl {
            s.pools[p].overl.insert(x, true);
        }
        s.pools[p].inflight.push(t);
        s.pools[p].overl.insert(t, over);
        (lat, off, fail, t, s.now)
    };
    let failed_app = cx.held && cx.app && cx.app_failed;
    if failed_app {
        sleep(MS).await;
        let s = w();
        s.begin_step(p);
        s.pools[p].inflight.retain(|x| *x != t);
        return false;
    }
    sleep(off).await;
    let (v, snap) = {
        let s = w();
        let snap = snap_over.unwrap_or(s.now);
        (s.n_at(snap), snap)
    };
    sleep(lat - off).await;
    let s = w();
    s.begin_step(p);
    s.pools[p].inflight.retain(|x| *x != t);
    let over = s.pools[p].overl.remove(&t).unwrap_or(false);
    if fail {
        return false;
    }
    if s.pools[p].starting {
        s.finish_take(p);
    }
    cx.carry = v;
    if s.c.fault == 18 {
        return arrive_split(p, t, v, snap).await;
    }
    s.arrive(p, t, v, snap, sent, over)
}

// ---- switch 18: no stale lock, each step split at every read and write of the state ----
async fn gap() {
    let d = {
        let s = w();
        let g = s.c.splitgap;
        s.rng.r(0, g)
    };
    sleep(d).await;
}
async fn arrive_split(p: usize, t: u64, r: i64, snap: u64) -> bool {
    let h = w().pools[p].horizon;
    gap().await;
    if t < h {
        return false;
    }
    let lit = w().pools[p].n;
    gap().await;
    let tk = w().pools[p].tickets;
    gap().await;
    w().pools[p].horizon = tk;
    gap().await;
    let several = w().c.several;
    if r == lit {
        let s = w();
        s.pools[p].snap = snap;
        s.pools[p].applied_snap = s.pools[p].applied_snap.max(snap);
        return false;
    }
    if r < lit && several {
        return false;
    }
    {
        let s = w();
        if snap <= s.pools[p].applied_snap {
            s.fail("I10 newer", format!("{} older snapshot", s.now));
        }
        if r > s.pools[p].n {
            s.pools[p].rises += 1;
        }
        s.pools[p].n = r;
        s.pools[p].snap = snap;
        s.pools[p].applied_snap = s.pools[p].applied_snap.max(snap);
    }
    gap().await;
    w().clear_tables(p);
    true
}
async fn request_split(p: usize, a: Act) -> bool {
    let win = w().pools[p].win;
    gap().await;
    let (n, sn, ri) = {
        let s = w();
        (s.pools[p].n, s.pools[p].snap, s.pools[p].rises)
    };
    gap().await;
    let open = win.is_none_or(|x| w().now >= x.2 || x.1 != n);
    let due = w().pools[p].due;
    gap().await;
    if !open {
        let s = w();
        s.pools[p].due = due.max(a);
        return false;
    }
    let a = a.max(due);
    {
        let s = w();
        if a == Act::Replace { s.pools[p].cgen += 1 } else { s.pools[p].sgen += 1 }
        s.rec_act(p, a, n, sn, ri);
    }
    gap().await;
    w().pools[p].due = Act::None;
    gap().await;
    {
        let s = w();
        let (now, wd) = (s.now, s.c.w);
        s.pools[p].win = Some((now, n, now + wd));
    }
    gap().await;
    w().clear_tables(p);
    true
}
async fn old_split(p: usize, conn: &Conn, cx: &mut Cx) -> bool {
    let e = w().pools[p].epoch.as_ref().map(|e| (e.id, e.t, e.n, e.end, e.susp));
    gap().await;
    let n = w().pools[p].n;
    gap().await;
    let start = e.is_none_or(|e| w().now >= e.3 || e.2 != n);
    let (id, t, susp) = if start {
        let s = w();
        let (wd, now) = (s.c.w, s.now);
        let q = &mut s.pools[p];
        q.epoch_seq += 1;
        q.epoch = Some(Epoch { id: q.epoch_seq, t: now, n, end: now + wd, susp: false, noextra: false });
        let x = (q.epoch_seq, now, q.snap, q.rises, q.open as u64);
        q.epochs.push(x);
        (x.0, now, false)
    } else {
        let e = e.unwrap();
        (e.0, e.1, e.4)
    };
    gap().await;
    let r = conn.opened < t && !susp;
    cx.discard = Some(id);
    cx.tested = Some(r);
    cx.epoch_t = Some(t);
    r
}

async fn request(p: usize, a: Act, unread: bool, cx: &mut Cx, snap: Option<u64>) -> bool {
    cx.reqs += 1;
    {
        let s = w();
        let now = s.now;
        s.pools[p].req_t.push(now);
    }
    let pooled = {
        let s = w();
        s.inc("requests");
        s.c.pooled
    };
    if unread && !pooled {
        if w().c.fault == 18 {
            let closed = {
                let s = w();
                let x = s.pools[p].win;
                x.is_some_and(|x| s.now < x.2 && x.1 == s.pools[p].n)
            };
            let mv = if closed { read(p, cx, snap).await } else { false };
            request_split(p, a).await;
            return mv;
        }
        let (taken, _) = {
            let s = w();
            (s.req_step(p, a, true), 0)
        };
        if taken {
            return false;
        }
        let mv = read(p, cx, snap).await;
        w().req_step(p, a, false);
        return mv;
    }
    let f = w().c.fault;
    if f == 8 {
        // judged by the <n> the call carried
        let c = cx.carry;
        let s = w();
        let n = s.pools[p].n;
        let open = s.win_open(p) || s.pools[p].win.unwrap().1 != c;
        if open {
            let (d, sn, ri) = (s.pools[p].due, s.pools[p].snap, s.pools[p].rises);
            s.do_take(p, a.max(d), n, sn, ri);
            if let Some(x) = s.pools[p].win.as_mut() {
                x.1 = c;
            }
        } else {
            s.req_step(p, a, false);
        }
        return false;
    }
    if f == 18 {
        request_split(p, a).await;
        return false;
    }
    w().req_step(p, a, false);
    false
}

pub async fn acquire(prefer: usize) -> usize {
    loop {
        {
            let s = w();
            let mut k = prefer;
            if s.rng.p(0.3) {
                k = (prefer + 1 + s.rng.n(2) as usize) % 3;
            }
            for d in 0..3 {
                let j = (k + d) % 3;
                if !s.srv[j].busy {
                    let o = s.srv[j].sess;
                    if (s.now - s.sess[o].opened >= 4 * S && s.c.fault != 106) {
                        s.srv[j].sess = s.new_sess();
                    }
                    s.srv[j].busy = true;
                    return j;
                }
            }
        }
        sleep(500).await;
    }
}
pub fn release_srv(j: usize) {
    let s = w();
    s.srv[j].busy = false;
    let o = s.srv[j].sess;
    if s.srv[j].kill || (s.now - s.sess[o].opened >= 4 * S && s.c.fault != 106) {
        s.srv[j].kill = false;
        s.srv[j].sess = s.new_sess();
    }
}
fn evict(conn: &mut Conn, sp: &Spec) {
    let s = w();
    s.sess[conn.sess].copies.retain(|(t, _)| t.0 != sp.stmt);
}
fn clear_caches(conn: &mut Conn) {
    let s = w();
    if s.c.pooled { conn.cache.clear() } else { s.sess[conn.sess].copies.clear() }
}

pub async fn run_once(p: usize, conn: &mut Conn, cx: &mut Cx, sp: &Spec, mode: Mode) -> Out {
    let (lat, inj) = {
        let s = w();
        s.inc("runs");
        cx.runs += 1;
        let lat = s.rng.r(MS, 10 * MS);
        let (t, se) = (s.c.timeout, s.c.ser);
        let inj = if cx.app && cx.app_failed {
            Some(Err::F25)
        } else if s.rng.p(t) {
            Some(Err::T57)
        } else if !cx.out && s.rng.p(se) {
            Some(Err::S40)
        } else {
            None
        };
        (lat, inj)
    };
    sleep(lat / 2).await;
    let pooled = w().c.pooled;
    let out = {
        if let Some(e) = inj {
            let s = w();
            let oracle = s.oracle(sp);
            Out { r: Err(e), cause: Cause::Inj, oracle, cached: false, copy: None, copy_at: 0, copy_by: p, comp: None, sess_opened: 0, t: s.now, ver: 0 }
        } else {
            let (how, keep) = match mode {
                Mode::Cached(t) => (Some(t), None),
                Mode::Keep(t) => (None, Some(t)),
                Mode::Unnamed => (None, None),
            };
            if pooled {
                match cx.srv {
                    Some(j) => {
                        let s = w();
                        let sid = s.srv[j].sess;
                        s.exec(sid, sp, how, keep, p)
                    }
                    None => {
                        let j = acquire(conn.last_srv).await;
                        conn.last_srv = j;
                        let o = {
                            let s = w();
                            let sid = s.srv[j].sess;
                            s.exec(sid, sp, how, keep, p)
                        };
                        release_srv(j);
                        o
                    }
                }
            } else {
                let s = w();
                s.exec(conn.sess, sp, how, keep, p)
            }
        }
    };
    if (!cx.out || cx.app) && cx.tx_snap.is_none() {
        cx.tx_snap = Some(w().now);
    }
    sleep(lat - lat / 2).await;
    if cx.app && out.r.is_err() {
        cx.app_failed = true;
    }
    if out.oracle.is_ok() && out.r.is_err() {
        // nothing
    }
    out
}

pub async fn run_call(p: usize, conn: &mut Conn, cx: &mut Cx, sp: &Spec) -> Res {
    cx.fresh_call();
    let (mode, ent, tw) = {
        let s = w();
        let f25 = s.c.fault == 25;
        let (sg, cg) = (s.pools[p].sgen, s.pools[p].cgen);
        if conn.cc != (sg, cg) && !f25 {
            clear_caches(conn);
            conn.cc = (sg, cg);
            conn.sgen = sg;
        }
        let now = s.now;
        let ent = s.pools[p].entries.iter().find(|e| e.stmt == sp.stmt && e.expiry > now).map(|e| (e.id, e.bytes, if e.marked { 1 } else if e.xx { 2 } else { 4 }, e.marked, e.replace, e.err));
        let retried = cx.retried && s.c.fault != 28;
        let pend = ent.is_some() && s.c.fault != 107;
        let mode = if pend || retried || !s.c.caches {
            Mode::Unnamed
        } else if s.c.pooled {
            let m = s.pools[p].marker;
            let t = *conn.cache.entry(sp.stmt).or_insert((sp.stmt, m.0, m.1));
            Mode::Cached(t)
        } else {
            Mode::Cached((sp.stmt, 0, 0))
        };
        (mode, ent, now)
    };
    cx.cached_mode = matches!(mode, Mode::Cached(_));
    cx.conn = conn.id;
    // the twenty-fourth review's FB1: with pending entries off no entry is made, so with the caches on every run a
    // call starts, outside .retry(n)'s later attempts, runs the cached copy: a statement under a per-transaction
    // search_path meets whatever path its connection's copy, or behind PgBouncer its server session's, was prepared
    // under, and fails inside a transaction at every switch from it (R-0718's "Pending entries off", R-1271's bullet)
    {
        let s = w();
        if (s.c.pend_max == 0 || s.c.e == 0) && s.c.caches && !cx.retried && !matches!(mode, Mode::Cached(_)) {
            s.fail("I8 entries off: a run with no entry not cached", format!("t={} stmt={}", s.now, sp.stmt));
        }
    }
    let o = run_once(p, conn, cx, sp, mode).await;
    cx.first = Some(o.r);
    {
        let s = w();
        if matches!(mode, Mode::Cached(_)) {
            if cx.retried {
                s.fail("I7 retried attempt runs cached", String::new());
            }
            if let Some(ch) = s.copy_change(sp, &o) {
                if s.pools[p].acts_t.iter().any(|a| *a > ch && *a < tw && o.copy_at < ch) {
                    if o.copy_by == p {
                        s.fail("I8 clearing", format!("{} stmt {}", s.now, sp.stmt));
                    } else {
                        s.inc("clearing clause broken by another process's copy");
                    }
                    s.inc("I8lit clearing");
                }
            }
        }
        if ent.is_some() && o.cached {
            s.fail("I8 pending run cached", String::new());
        }
    }
    let fresh = !matches!(mode, Mode::Cached(_));
    // the confirming run
    if let Some((id, bytes, ferr, marked, repl, terr)) = ent {
        if bytes == Some(sp.bytes()) {
            cx.confirm_run = true;
            w().begin_step(p);
            let conf = w().confirms(&o.r, ferr) && !(w().c.fault == 42 && o.r.is_err_and(|e| matches!(e.code(), 1..=4)));
            if !conf && w().confirms(&o.r, terr) {
                w().fail("I13 a confirming run that ends in another error doesn't confirm", String::new());
            }
            if conf && o.r.is_err_and(|e| e.code() == terr) {
                w().fail("I13 a confirming run that ends in the error that set its bytes confirms", String::new());
            }
            if ferr != terr && w().c.fault != 39 {
                w().fail("I13 an entry's bits don't give its error", format!("{} {}", ferr, terr));
            }
            if cx.held { w().inc("held confirming runs"); if conf && !marked { w().inc("held confirming runs that request"); } }
            if conf {
                let now = w().now;
                w().pools[p].confs.push((now, true));
                if !marked {
                    let (pooled, f15, f17) = {
                        let s = w();
                        (s.c.pooled, s.c.fault == 15, s.c.fault == 17)
                    };
                    let unread = (cx.out || f15) && (!cx.held || f17) && !pooled;
                    let snap = if (!cx.out || (cx.held && cx.app)) && cx.rr { cx.tx_snap } else { None };
                    request(p, if repl { Act::Replace } else { Act::Clear }, unread, cx, snap).await;
                }
            } else {
                let s = w();
                let f27 = s.c.fault == 27;
                if f27 {
                    s.pools[p].entries.retain(|e| e.id != id);
                } else if let Some(e) = s.pools[p].entries.iter_mut().find(|e| e.stmt == sp.stmt) {
                    if e.bytes == bytes {
                        e.bytes = None;
                    }
                }
                if let Err(e) = o.r {
                    if e.noconf() {
                        return Res::Pg(o);
                    }
                }
            }
        }
    }
    // row B3's or B2's mark on the next attempt's run
    if let Some((ms, first)) = cx.due_mark {
        if ms.stmt == sp.stmt && ms.bytes() == sp.bytes() && ent.is_none_or(|e| e.1 != Some(sp.bytes())) {
            cx.due_mark = None;
            fh1_note(sp, &o, false, first);
            if w().confirms(&o.r, first) || fh1_break(&o) {
                w().begin_step(p);
                w().inc("FF1 replace requested: a confirming next attempt, row B3");
                fg1_count(sp, &o, false, first);
                request(p, Act::Replace, false, cx, None).await;
                fg2_record(p, conn);
            }
        }
    }
    if o.r.is_ok() {
        return Res::Ok(o);
    }
    let (r0, q0, e0) = (cx.reqs, o.clone(), cx.made_entry);
    let c0 = cx.cached_mode;
    let res = on_error(p, conn, cx, sp, o, fresh).await;
    if let Err::P22 { named: Named::Kept(_), ret: false, .. } = q0.r.unwrap_err() {
        if matches!(q0.cause, Cause::Kept(_)) && !casts(sp.stmt) && sp.stmt != NOC && w().visible(&q0) {
            // the four answers' loud half: a kept type's stale function's error reached through a cached statement
            // that casts no parameter gets the second row's handling, one fresh run outside a transaction and
            // Error::SchemaChanged and a pending entry inside one, and then returns as it is; nothing session-class
            // is done for it, and nothing is requested unless that fresh run confirms
            // the fresh run's own outcome, a session-class error or a timeout among them, counts as any server
            // error's does
            let ok = match &res {
                Res::Pg(x) => x.r != q0.r || cx.reqs == r0 || cx.fresh_sess || cx.other,
                Res::Ok(_) => true,
                Res::Sc(_) => !cx.out || cx.fresh_sess, // row A4 on a held connection, after the fresh run met a stale session
            };
            let second = c0 && !cx.out;
            let entered = cx.made_entry && !e0;
            let own = w().class(q0.r.unwrap_err(), c0, sp, cx.out) == Cls::Sess;
            if !ok || own || (entered && !second) || (!c0 && (cx.reqs > r0 || entered || !matches!(&res, Res::Pg(x) if x.r == q0.r && x.t == q0.t))) {
                w().fail("I9 loud: more than the second row's, or not as it is after it", format!("{:?} out={} reqs={} entry={} repl={} cool={} other={} fs={} res={:?}", q0, cx.out, cx.reqs - r0, cx.made_entry, cx.replaced, cx.cool, cx.other, cx.fresh_sess, match &res { Res::Ok(x) => format!("ok {:?}", x.r), Res::Pg(x) => format!("pg {:?}", x.r), Res::Sc(x) => format!("sc {:?}", x.r) }));
            }
            if c0 {
                w().inc(if cx.out { "kept function's error, cached, outside a transaction" } else { "kept function's error, cached, in a transaction" });
                if matches!(&res, Res::Sc(_)) {
                    w().inc("kept function's error: SchemaChanged first");
                }
            }
        }
    }
    res
}

async fn on_error(p: usize, conn: &mut Conn, cx: &mut Cx, sp: &Spec, o: Out, fresh: bool) -> Res {
    w().begin_step(p);
    let err = o.r.unwrap_err();
    let holds = matches!(err, Err::P22 { ret: true, .. });
    let ptest = matches!(sp.label, Label::RetX | Label::RetReal);
    if ptest && (holds || w().c.fault == 101) {
        cx.ptest = true;
        return Res::Pg(o);
    }
    let cls = w().class(err, !fresh, sp, cx.out);
    if cls == Cls::Sess {
        return session(p, conn, cx, sp, o).await;
    }
    if cls != Cls::Stmt {
        return Res::Pg(o);
    }
    let (pooled, caches) = {
        let s = w();
        (s.c.pooled, s.c.caches)
    };
    if cx.out {
        {
            let s = w();
            if s.c.fault == 103 {
                return Res::Pg(o);
            }
            if s.c.fault == 7 && !s.pools[p].win.is_none_or(|x| s.now >= x.2) {
                return Res::Pg(o);
            }
        }
        // A1, C1: the fresh run
        let r = if pooled {
            run_once(p, conn, cx, sp, Mode::Unnamed).await
        } else {
            if !(w().c.fault == 34 && cx.app) {
                evict(conn, sp);
            }
            if caches { run_once(p, conn, cx, sp, Mode::Keep((sp.stmt, 0, 0))).await } else { run_once(p, conn, cx, sp, Mode::Unnamed).await }
        };
        w().begin_step(p);
        if w().confirms(&r.r, err.code()) || err == Err::A0 {
            if pooled {
                let mv = read(p, cx, None).await;
                let now = w().now;
                w().pools[p].confs.push((now, true));
                if !mv {
                    w().pools[p].unconfs.push(now);
                    request(p, Act::Clear, false, cx, None).await;
                }
            } else {
                let now = w().now;
                w().pools[p].confs.push((now, true));
                let f17 = w().c.fault == 17;
                let snap = if cx.held && cx.app && cx.rr { cx.tx_snap } else { None };
                request(p, Act::Clear, !cx.held || f17, cx, snap).await;
            }
        }
        if r.r == Err(Err::F25) {
            return Res::Pg(o);
        }
        if let (Err::XX, Err(Err::XX)) = (err, r.r) {
            if w().c.fault == 102 {
                return Res::Pg(r);
            }
            return session(p, conn, cx, sp, r).await;
        }
        if r.r.is_ok() {
            return Res::Ok(r);
        }
        if w().c.fault != 37 {
            let e2 = r.r.unwrap_err();
            let c2 = w().class(e2, false, sp, cx.out);
            let h2 = matches!(e2, Err::P22 { ret: true, .. });
            if ptest && h2 {
                cx.ptest = true;
            }
            if c2 == Cls::Sess && !(ptest && h2) {
                cx.fresh_sess = true;
                return session(p, conn, cx, sp, r).await;
            }
        }
        return Res::Pg(r);
    }
    // B1, B2, D1
    let (f16, f3) = {
        let s = w();
        (s.c.fault == 16, s.c.fault == 3)
    };
    let _ = f3;
    let spsnap = if f16 && cx.sp && cx.rr { cx.tx_snap } else { None };
    cx.rolled = true;
    sleep(MS).await;
    w().begin_step(p);
    if !pooled {
        evict(conn, sp);
    }
    let mut mv = false;
    if pooled || err == Err::XX {
        mv = read(p, cx, spsnap).await;
        w().begin_step(p);
    }
    if err == Err::A0 {
        let now = w().now;
        w().pools[p].confs.push((now, false));
    }
    if !mv && err == Err::A0 {
        if pooled {
            let now = w().now;
            w().pools[p].unconfs.push(now);
        }
        mv = request(p, Act::Clear, !pooled, cx, spsnap).await;
    }
    if !mv {
        w().entry(p, sp, err, err == Err::A0);
        cx.made_entry = w().c.pend_max > 0 && w().c.e > 0;
    }
    cx.discard = None;
    if !pooled && err == Err::XX {
        let old = if w().c.fault == 18 { old_split(p, conn, cx).await } else { w().old(p, conn, cx) };
        if old {
            w().expect_confirm(p, sp, err.code(), cx);
            return Res::Sc(o);
        }
        cx.discard = None;
    }
    Res::Sc(o)
}

async fn session(p: usize, conn: &mut Conn, cx: &mut Cx, sp: &Spec, o: Out) -> Res {
    let err = o.r.unwrap_err();
    let pooled = w().c.pooled;
    cx.discard = None;
    if pooled {
        let (incool, wd, cm) = {
            let s = w();
            let now = s.now;
            let had = s.pools[p].entries.iter().any(|e| e.stmt == sp.stmt);
            if had {
                s.pools[p].entries.retain(|e| e.stmt != sp.stmt);
                s.pools[p].removals.entry(sp.stmt).or_default().push(now);
            }
            (s.pools[p].cool.iter().any(|x| x.0 == sp.stmt && x.1 > now), s.c.w, s.c.cool_max)
        };
        if incool {
            cx.cool = true;
            return Res::Pg(o);
        }
        let add = || {
            let s = w();
            let now = s.now;
            if s.pools[p].cool.iter().any(|x| x.0 == sp.stmt) {
                return;
            }
            if s.pools[p].cool.len() >= cm {
                if let Some(x) = s.pools[p].cool.pop_front() {
                    if x.1 > now {
                        s.pools[p].cool_rm.entry(x.0).or_default().push((now, 3));
                    }
                }
            }
            // break 127: a cool-down entry expiring half of stale_cooldown after it's made
            let life = if s.c.fault == 127 { wd / 2 } else { wd };
            s.pools[p].cool.push_back((sp.stmt, now + life));
        };
        if cx.out {
            cx.other = true;
            // another client connection: a random server session, with nothing cached
            let r = if w().c.fault == 35 {
                // the call keeps its own connection while it waits for another
                let mut c2 = crate::calls::checkout_but(p, conn.id, true).await;
                let r = run_once(p, &mut c2, cx, sp, Mode::Unnamed).await;
                crate::calls::release(p, c2);
                r
            } else {
                // gives its own back, then checks out as any call does, taking another if one is idle
                let own = conn.id;
                let queue = !w().pools[p].waiters.is_empty();
                let gone = Conn { id: 0, opened: 0, sgen: 0, cgen: 0, cc: (0, 0), sess: 0, cache: HashMap::new(), last_srv: 0, mark: None };
                crate::calls::release(p, std::mem::replace(conn, gone));
                if queue {
                    w().inc("C2 waits behind the queue");
                    crate::core::wait(p).await;
                }
                *conn = crate::calls::checkout_but(p, own, false).await;
                w().inc(if conn.id == own { "C2 on the same connection" } else { "C2 on another connection" });
                if w().c.fault == 46 {
                    run_once(p, conn, cx, sp, Mode::Unnamed).await;
                }
                run_once(p, conn, cx, sp, Mode::Unnamed).await
            };
            w().begin_step(p);
            return match r.r {
                Ok(_) => Res::Ok(r),
                Err(e) => {
                    if e.code() == err.code() {
                        add();
                        cx.cool = true;
                    }
                    Res::Pg(r)
                }
            };
        }
        if w().c.fault == 126 {
            // break 126: the statement enters the cool-down a step after its failure, so calls meeting the stale
            // session meanwhile return Error::SchemaChanged too
            sleep(MS).await;
            w().begin_step(p);
        }
        {
            // the twenty-seventh review's FE1: row D2's Error::SchemaChanged comes only from outside the cool-down,
            // so two of one statement in a pool come stale_cooldown apart unless its entry was removed between them
            let s = w();
            let (now, wd) = (s.now, s.c.w);
            if let Some(t0) = s.pools[p].d2_sc.get(&sp.stmt).copied() {
                let rm: Vec<u8> = s.pools[p].cool_rm.get(&sp.stmt).map(|v| v.iter().filter(|x| x.0 >= t0 && x.0 <= now && x.0 < t0 + wd).map(|x| x.1).collect()).unwrap_or_default();
                if now - t0 < wd {
                    s.fail("lit FE1 R-1249 and R-1258, a stale session's SchemaChanged once per stale_cooldown", format!("stmt {} at {} and {}", sp.stmt, t0, now));
                    if rm.is_empty() {
                        s.fail("new I8 FE1 two D2 SchemaChanged of a statement within stale_cooldown, no removal between", format!("stmt {} at {} and {}", sp.stmt, t0, now));
                    }
                }
                s.inc(if rm.contains(&1) {
                    "FE1 D2 SchemaChanged: after a pool-wide action"
                } else if rm.contains(&2) {
                    "FE1 D2 SchemaChanged: after an adoption or a moved <n>"
                } else if rm.contains(&3) {
                    "FE1 D2 SchemaChanged: after a push-out by the cap"
                } else {
                    "FE1 D2 SchemaChanged: after the expiry"
                });
            } else {
                s.inc("FE1 D2 SchemaChanged: the first");
            }
            s.pools[p].d2_sc.insert(sp.stmt, now);
        }
        if w().c.fault != 125 {
            // break 125: row D2 never puts the statement in the cool-down inside a transaction
            add();
        }
        cx.cool = true;
        cx.rolled = true;
        sleep(MS).await;
        return Res::Sc(o);
    }
    let sp_bit = cx.sp && w().c.fault != 31;
    if !cx.out && !sp_bit {
        cx.rolled = true;
        sleep(MS).await;
        w().begin_step(p);
    }
    let (f15, f16) = {
        let s = w();
        (s.c.fault == 15, s.c.fault == 16)
    };
    let _ = f15;
    if !sp_bit || f16 {
        let snap = if f16 && cx.sp && cx.rr { cx.tx_snap } else { None };
        if w().c.fault == 45 {
            read(p, cx, snap).await;
        }
        read(p, cx, snap).await;
    }
    let f = w().c.fault;
    if f == 19 {
        let carry = cx.carry;
        let start = {
            let s = w();
            match &s.pools[p].epoch { None => true, Some(e) => s.now >= e.end || e.n != carry }
        };
        if start {
            let s = w();
            let (wd, now) = (s.c.w, s.now);
            let q = &mut s.pools[p];
            q.epoch_seq += 1;
            q.epoch = Some(Epoch { id: q.epoch_seq, t: now, n: carry, end: now + wd, susp: false, noextra: false });
            let x = (q.epoch_seq, now, q.snap, q.rises, q.open as u64);
            q.epochs.push(x);
        }
        if f == 19 {
            // the machine's own test is left out: the epoch is the one above
            let s = w();
            let n = s.pools[p].epoch.as_ref().unwrap().n;
            let keep = s.pools[p].n;
            s.pools[p].n = n;
            let r = s.old(p, conn, cx);
            s.pools[p].n = keep;
            cx.tested = Some(r);
        }
    }
    let isold = if f == 19 { cx.tested == Some(true) } else if f == 18 { old_split(p, conn, cx).await } else { w().old(p, conn, cx) };
    if !isold {
        if !cx.out && cx.sp && cx.rolled {
            w().fail("I11", "B4 rolled the whole transaction back".into());
        }
        cx.discard = None;
        return Res::Pg(o);
    }
    let ep = cx.discard.unwrap();
    let sure = matches!(err, Err::D42804 | Err::XX);
    if sure {
        if w().c.fault == 3 {
            let s = w();
            let (n, sn, ri) = (s.pools[p].n, s.pools[p].snap, s.pools[p].rises);
            s.do_take(p, Act::Replace, n, sn, ri);
        } else {
            w().inc("FF1 replace requested: a 42804 or XX000 at its failure");
            request(p, Act::Replace, false, cx, None).await;
        }
    }
    if cx.held {
        cx.discard = None;
        conn.mark = Some(ep);
        cx.replaced = true;
        if w().c.fault == 32 {
            return Res::Pg(o);
        }
        w().inc("A4 SchemaChanged");
        return Res::Sc(o);
    }
    if cx.out {
        cx.discard = None;
        cx.replaced = true;
        // A2
        match replace(p, conn, cx, sp, ep).await {
            None => return Res::Pg(o),
            Some(r) => {
                w().begin_step(p);
                fh1_note(sp, &r, true, err.code());
                if !sure && (w().confirms(&r.r, err.code()) || fh1_break(&r)) {
                    w().inc("FF1 replace requested: a confirming run on the new connection, row A2");
                    fg1_count(sp, &r, true, err.code());
                    request(p, Act::Replace, false, cx, None).await;
                    fg2_record(p, conn);
                }
                return if r.r.is_ok() { Res::Ok(r) } else { Res::Pg(r) };
            }
        }
    }
    cx.replaced = true;
    if !sure {
        w().expect_confirm(p, sp, err.code(), cx);
    }
    if cx.sp {
        cx.rolled = true;
        sleep(MS).await;
    }
    Res::Sc(o)
}

// try to open one connection; Ok(sess, opened), Err(is 53300)
pub async fn open_try(p: usize) -> Result<(usize, u64, u64, u64), bool> {
    let (lat, sg, cg) = {
        let s = w();
        s.pools[p].open += 1;
        s.check_open(p);
        (s.rng.r(5 * MS, 25 * MS), s.pools[p].sgen, s.pools[p].cgen)
    };
    sleep(lat).await;
    let s = w();
    if !s.c.pooled {
        if s.now < s.db.refused_until {
            s.pools[p].open -= 1;
            s.inc("refused opens");
            return Err(false);
        }
        if s.pools[p].open > s.c.role_limit {
            s.pools[p].open -= 1;
            s.db.last_refusal_end = s.db.last_refusal_end.max(s.now);
            s.inc("53300");
            return Err(true);
        }
    }
    let sess = if s.c.pooled { 0 } else { s.new_sess() };
    Ok((sess, s.now, sg, cg))
}
pub fn mk_conn(p: usize, o: (usize, u64, u64, u64)) -> Conn {
    let s = w();
    let id = s.id();
    let _ = p;
    let srv = s.rng.n(3) as usize;
    Conn { id, opened: o.1, sgen: o.2, cgen: o.3, cc: (o.2, o.3), sess: o.0, cache: HashMap::new(), last_srv: srv, mark: None }
}
fn count_open(p: usize, ep: u64) {
    let s = w();
    let now = s.now;
    s.pools[p].sopens.push(now);
    *s.pools[p].ep_open.entry(ep).or_insert(0) += 1;
}

// the twenty-ninth review's FG1: what made a stored-text function's confirming run confirm
fn fg1_count(sp: &Spec, o: &Out, a2: bool, first: u8) {
    if sp.stmt != ST {
        return;
    }
    let s = w();
    let k = match (o.r, s.db.stored_alt) {
        (Err(Err::OT), _) => if a2 { "FG1 A2 replace by stored text: the function fails another way" } else { "FG1 B3 replace by stored text: the function fails another way" },
        (Ok(_), 0) => if a2 { "FG1 A2 replace by stored text: a good label" } else { "FG1 B3 replace by stored text: a good label" },
        (Ok(_), 1) => if a2 { "FG1 A2 replace by stored text: NULL" } else { "FG1 B3 replace by stored text: NULL" },
        (Ok(_), _) => if a2 { "FG1 A2 replace by stored text: no row" } else { "FG1 B3 replace by stored text: no row" },
        _ => if a2 { "FG1 A2 replace by stored text: other" } else { "FG1 B3 replace by stored text: other" },
    };
    s.inc(k);
    // the thirtieth review's FH1: after a 22P02, a run that ends in another 22P02, the same error by SQLSTATE, never
    // confirms, so it never brings the *replace* action, whatever routine raised it (R-1245, R-1249, R-1250); after
    // row B2's XX000 a 22P02 is another SQLSTATE, and confirms
    if let Err(e) = o.r {
        if e.code() == 4 && first == 4 {
            let d = format!("t={} {:?}", s.now, e);
            s.fail("FH1 a run ending in another 22P02 brought a replace", d);
        }
    }
    // the older wording, R-0981's "stored text a client changes to a good label", as a check of an older wording
    if !k.ends_with("a good label") {
        let d = format!("t={} {}", s.now, k);
        s.fail("lit FG1 R-0981 a replace from stored text only when it changes to a good label", d);
    }
}
// the thirtieth review's FH1: a stored-text function's run on the new connection, row A2, or the next attempt's, row
// B3, after its stored text changed, that ends in another 22P02, from enum_in (another bad label) or from another
// routine (bad text for a number): it requests nothing. R-1249's and R-1250's older wording took any run that fails
// another way, off the errors that stop it before the value, for a cured stale session ("lit FH1")
fn fh1_note(sp: &Spec, o: &Out, a2: bool, first: u8) {
    if sp.stmt != ST || first != 4 {
        return;
    }
    let s = w();
    let (k, c) = match (o.r, a2) {
        (Err(Err::P22 { ret: false, .. }), true) => ("another bad label, enum_in", "FH1 A2 another 22P02, another bad label (enum_in): nothing requested"),
        (Err(Err::P22 { ret: false, .. }), false) => ("another bad label, enum_in", "FH1 B3 another 22P02, another bad label (enum_in): nothing requested"),
        (Err(Err::N22), true) => ("bad text for a number", "FH1 A2 another 22P02, bad text for a number (another routine): nothing requested"),
        (Err(Err::N22), false) => ("bad text for a number", "FH1 B3 another 22P02, bad text for a number (another routine): nothing requested"),
        _ => return,
    };
    s.inc(c);
    let d = format!("t={} {}", s.now, k);
    s.fail("lit FH1 R-1249 and R-1250, a run failing another way off the stopping errors confirms", d);
}
// break 129: a run ending in another 22P02 taken for a confirmation, as R-1249's older wording reads
fn fh1_break(o: &Out) -> bool {
    w().c.fault == 129 && matches!(o.r, Err(Err::P22 { ret: false, .. }) | Err(Err::N22))
}
// the twenty-ninth review's FG2: the connection whose run requested *replace* opened before the action is taken, so
// it's behind the connection generation once it is, and is replaced at its next checkout too
fn fg2_record(p: usize, conn: &Conn) {
    let s = w();
    if s.c.pooled || conn.id == 0 {
        return;
    }
    let now = s.now;
    s.pools[p].req_conn.insert(conn.id, now);
    s.inc(if conn.cgen < s.pools[p].cgen { "FG2 requester behind at once (action taken)" } else { "FG2 requester not yet behind (action due)" });
}

async fn replace(p: usize, conn: &mut Conn, cx: &mut Cx, sp: &Spec, ep: u64) -> Option<Out> {
    let beside = {
        let s = w();
        let f24 = s.c.fault == 24;
        let no = s.pools[p].epoch.as_ref().is_some_and(|e| e.id == ep && e.noextra);
        let ok = (s.pools[p].extra == 0 || f24) && s.pools[p].open < s.c.budget && !no;
        if ok {
            s.pools[p].extra += 1;
        }
        ok
    };
    if beside {
        count_open(p, ep);
        match open_try(p).await {
            Ok(o) => {
                let mut nc = mk_conn(p, o);
                let mode = if w().c.caches { Mode::Keep((sp.stmt, 0, 0)) } else { Mode::Unnamed };
                let r = run_once(p, &mut nc, cx, sp, mode).await;
                let s = w();
                s.pools[p].extra -= 1;
                s.pools[p].open -= 1; // the old one closes
                *s.pools[p].ep_repl.entry(ep).or_insert(0) += 1; let rt = s.now; s.pools[p].repl_t.push((rt, 0)); s.pools[p].repl_dbg.push((rt, ep, 1));
                *conn = nc;
                return Some(r);
            }
            Err(limit) => {
                let s = w();
                s.begin_step(p);
                s.pools[p].extra -= 1;
                if !limit {
                    s.suspend(p, ep);
                    return None;
                }
                if s.c.fault != 30 {
                    if let Some(e) = s.pools[p].epoch.as_mut() {
                        if e.id == ep {
                            e.noextra = true;
                        }
                    }
                }
            }
        }
    }
    // in the slot
    w().pools[p].open -= 1;
    count_open(p, ep);
    match open_try(p).await {
        Ok(o) => {
            let mut nc = mk_conn(p, o);
            let mode = if w().c.caches { Mode::Keep((sp.stmt, 0, 0)) } else { Mode::Unnamed };
            let r = run_once(p, &mut nc, cx, sp, mode).await;
            { let s = w(); *s.pools[p].ep_repl.entry(ep).or_insert(0) += 1; let rt = s.now; s.pools[p].repl_t.push((rt, 0)); s.pools[p].repl_dbg.push((rt, ep, 2)); }
            *conn = nc;
            Some(r)
        }
        Err(_) => {
            let s = w();
            s.begin_step(p);
            s.suspend(p, ep);
            conn.id = 0; // the slot is empty
            None
        }
    }
}
