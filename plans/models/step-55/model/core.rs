// Throwaway model for step 55's EZ round: R-1270's procedure on a discrete-event scheduler.
// One async fn per call; the code between two awaits is one step.
use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

pub const MS: u64 = 1000;
pub const S: u64 = 1_000_000;

thread_local! { static SIM: Cell<*mut Sim> = const { Cell::new(std::ptr::null_mut()) }; }
pub fn w() -> &'static mut Sim {
    SIM.with(|s| unsafe { &mut *s.get() })
}

pub struct Sleep {
    at: u64,
    armed: bool,
}
impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        let s = w();
        if !self.armed {
            self.armed = true;
            s.seq += 1;
            let (q, c) = (s.seq, s.cur);
            s.heap.push(Reverse((self.at, q, c)));
            Poll::Pending
        } else if s.now >= self.at {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
pub fn sleep(d: u64) -> Sleep {
    Sleep { at: w().now + d, armed: false }
}
// wait in the pool's queue until a slot or a connection comes back
pub struct Wait {
    p: usize,
    armed: bool,
}
impl Future for Wait {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        if self.armed {
            return Poll::Ready(());
        }
        self.armed = true;
        let s = w();
        let c = s.cur;
        s.pools[self.p].waiters.push_back(c);
        Poll::Pending
    }
}
pub fn wait(p: usize) -> Wait {
    Wait { p, armed: false }
}
type Task = Pin<Box<dyn Future<Output = ()>>>;

#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn p(&mut self, x: f64) -> bool {
        self.f() < x
    }
    pub fn r(&mut self, a: u64, b: u64) -> u64 {
        if b <= a { a } else { a + self.next() % (b - a + 1) }
    }
    pub fn n(&mut self, k: u64) -> u64 {
        self.next() % k
    }
    pub fn exp(&mut self, mean: f64) -> u64 {
        (-(1.0 - self.f()).ln() * mean) as u64
    }
}

#[derive(Clone, Debug)]
pub struct Cfg {
    pub name: &'static str,
    pub pooled: bool,
    pub procs: usize,
    pub servers: usize,
    pub several: bool,
    pub w: u64,
    pub e: u64,
    pub pend_max: usize,
    pub cool_max: usize,
    pub caches: bool,
    pub max: usize,
    pub budget: usize,
    pub role_limit: usize,
    pub events: usize,
    pub gap: f64,
    pub burst: f64,
    pub burst_lo: u64,
    pub burst_hi: u64,
    pub world: f64,
    // mig, lower, drop, keep, col, nothing, stored, refuse, relabel-by-hand
    pub mix: [f64; 9],
    pub mig_refresh: f64,
    pub mig_col: f64,
    pub tx: f64,
    pub held: f64,
    pub two: f64,
    pub gap2: u64,
    pub rr: f64,
    pub sp: f64,
    pub retry0: bool,
    pub retry2: bool,
    pub ser: f64,
    pub held_two: f64,
    pub held_app: f64,
    pub held_rerun: f64,
    pub long: f64,
    pub held_gap: u64,
    pub timeout: f64,
    pub land: f64,
    pub lockwait: u64,
    pub one_value: bool,
    pub only: Option<u8>,
    pub client: bool,
    pub co_gap: u64,
    pub retired_real: bool,
    pub both_stmt: bool,
    pub trig: bool,
    pub noreset: bool,
    pub splitgap: u64,
    pub pooled2: bool,
    pub wait_at: u64,
    pub fault: u32,
    pub readfail: f64,
    pub proc_lag: bool,
    pub path_out: bool,
}
impl Cfg {
    pub fn base(pooled: bool) -> Cfg {
        Cfg {
            name: "defaults",
            pooled,
            procs: 1,
            servers: 1,
            several: false,
            w: S,
            e: S,
            pend_max: 64,
            cool_max: 64,
            caches: true,
            max: 4,
            budget: 5,
            role_limit: 1000,
            events: 2000,
            gap: 20.0 * 1000.0,
            burst: 0.2,
            burst_lo: 2,
            burst_hi: 8,
            world: 0.055,
            mix: [4., 2., 2., 2., 2., 1., 2., 1., 0.],
            mig_refresh: 0.5,
            mig_col: 0.5,
            tx: 0.35,
            held: if pooled { 0.0 } else { 0.10 },
            two: 0.3,
            gap2: 10 * MS,
            rr: 0.5,
            sp: 0.3,
            retry0: false,
            retry2: false,
            ser: 0.02,
            held_two: 0.3,
            held_app: 0.5,
            held_rerun: 0.5,
            long: 0.0,
            held_gap: 10 * MS,
            timeout: 0.01,
            land: 0.5,
            lockwait: 500,
            one_value: false,
            only: None,
            client: true,
            co_gap: 0,
            retired_real: false,
            both_stmt: true,
            trig: true,
            noreset: false,
            splitgap: 500,
            pooled2: false,
            wait_at: 0,
            fault: 0,
            readfail: 0.01,
            proc_lag: false,
            path_out: false,
        }
    }
}

// ---- world ----
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TS {
    Cur,
    Dropped,
    Kept,
    Retired,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Label {
    Shared,
    New(u32),
    Newest,
    NewName, // as Newest, its text holding the type's name
    Bad,
    RetX,
    RetReal,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Named {
    Cur,
    Kept(u32),
    Ret(u32),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Err {
    A0,
    XX,
    D42804,
    P22 { named: Named, ret: bool, nm: bool },
    T57,
    S40,
    F25,
}
impl Err {
    pub fn code(&self) -> u8 {
        match self {
            Err::A0 => 1,
            Err::XX => 2,
            Err::D42804 => 3,
            Err::P22 { .. } => 4,
            Err::T57 => 5,
            Err::S40 => 6,
            Err::F25 => 7,
        }
    }
    pub fn noconf(&self) -> bool {
        matches!(self, Err::T57 | Err::S40 | Err::F25)
    }
}
pub const CAST: u8 = 0;
pub const COL: u8 = 1;
pub const PATH: u8 = 2;
pub const F: u8 = 3;
pub const ST: u8 = 4;
pub const H: u8 = 5;
pub const BOTH: u8 = 6;
pub const TRIG: u8 = 7;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    None,
    Stmt(u64),
    Sess(u64),
    Kept(u32),
    Path,
    Data,
    Inj,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Spec {
    pub stmt: u8,
    pub val: u8,
    pub label: Label,
    pub path: u8,
    pub shape: u8,
}
impl Spec {
    pub fn bytes(&self) -> (u8, Label) {
        (self.val, self.label)
    }
}
pub type Text = (u8, i64, u64);
#[derive(Clone, Debug)]
pub struct Copy {
    pub id: u64,
    pub at: u64,
    pub ty: u32,
    pub resets: u32,
    pub planned: u64,
    pub col: u32,
    pub path: u8,
    pub by: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Comp {
    pub ver: u32,
    pub refresh: u32,
    pub at: u64,
}
pub struct Sess {
    pub opened: u64,
    pub f: Option<Comp>,
    pub s: Option<Comp>,
    pub h: Option<(u8, u32, u32, u64)>,
    pub b: Option<(u32, u32)>,
    pub copies: Vec<(Text, Copy)>,
}
#[derive(Clone, Debug)]
pub struct Out {
    pub r: Result<u32, Err>,
    pub cause: Cause,
    pub oracle: Result<u32, u8>,
    pub cached: bool,
    pub copy: Option<u64>,
    pub copy_at: u64,
    pub copy_by: usize,
    pub comp: Option<Comp>,
    pub sess_opened: u64,
    pub t: u64,
    pub ver: u32,
}

pub struct Db {
    pub n: i64,
    pub nhist: Vec<(u64, i64)>,
    pub replay: Vec<Vec<(u64, i64)>>,
    // state, since when in it, when it left current
    pub types: Vec<(TS, u64, u64)>,
    pub col: u32,
    pub colhist: Vec<u64>,
    pub resets: u32,
    pub refresh: u32,
    pub stored_bad: bool,
    pub last_change: u64,
    pub refused_until: u64,
    pub last_refusal_end: u64,
    pub last_reconnect: u64,
}
// ---- pool ----
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Act {
    None,
    Clear, // salt behind PgBouncer
    Replace,
}
pub struct Entry {
    pub stmt: u8,
    pub bytes: Option<(u8, Label)>,
    pub err: u8,
    pub marked: bool,
    pub xx: bool,
    pub replace: bool,
    pub expiry: u64,
    pub id: u64,
}
pub struct Epoch {
    pub id: u64,
    pub t: u64,
    pub n: i64,
    pub end: u64,
    pub susp: bool,
    pub noextra: bool,
}
pub struct Conn {
    pub id: u64,
    pub opened: u64,
    pub sgen: u64,
    pub cgen: u64,
    pub cc: (u64, u64),
    pub sess: usize,
    pub cache: HashMap<u8, Text>,
    pub last_srv: usize,
    pub mark: Option<u64>,
}
pub struct ActRec {
    pub t: u64,
    pub n: i64,
    pub snap: u64,
    pub snap_lit: u64,
    pub lr_lit: i64,
    pub rise: u64, // how many times the last-read <n> had risen when the <n> it was taken under was current
    pub rise_lit: u64,
    pub kind: Act,
    pub cgen: u64,
}
#[derive(Default)]
pub struct Pool {
    pub sgen: u64,
    pub cgen: u64,
    pub due: Act,
    pub starting: bool,
    pub win: Option<(u64, i64, u64)>,
    pub epoch: Option<Epoch>,
    pub epoch_seq: u64,
    pub extra: u32,
    pub entries: VecDeque<Entry>,
    pub cool: VecDeque<(u8, u64)>,
    pub n: i64,
    pub snap: u64,
    pub rises: u64,
    pub tickets: u64,
    pub horizon: u64,
    pub highest: i64,
    pub marker: (i64, u64),
    pub step_n0: i64,
    pub step_snap: u64,
    pub step_rise: u64,
    pub idle: Vec<Conn>,
    pub slots: usize,
    pub open: usize,
    pub waiters: VecDeque<usize>,
    // records
    pub acts: Vec<ActRec>,
    pub adopts: Vec<u64>,
    pub epochs: Vec<(u64, u64, u64, u64, u64)>, // id, t, snap, rises, open at start
    pub ep_repl: HashMap<u64, u32>,
    pub ep_open: HashMap<u64, u32>,
    pub cg_repl: HashMap<u64, u32>,
    pub sopens: Vec<u64>,
    pub owed: Vec<(u64, u64, bool)>,
    pub applied_snap: u64,
    pub inflight: Vec<u64>,
    pub overl: HashMap<u64, bool>,
    pub confs: Vec<(u64, bool)>,
    pub unconfs: Vec<u64>,
    pub req_t: Vec<u64>,
    pub co_t: Vec<u64>,
    pub acts_t: Vec<u64>,
    pub fails_tx: HashMap<(u8, bool), Vec<(u64, u64)>>,
    pub removals: HashMap<u8, Vec<u64>>,
    pub created: HashMap<u8, Vec<u64>>,
    pub moved: Vec<(u64, i64, u64)>,
}
impl Default for Act {
    fn default() -> Self {
        Act::None
    }
}
pub struct Srv {
    pub sess: usize,
    pub busy: bool,
    pub kill: bool,
}

pub struct Sim {
    pub now: u64,
    pub seq: u64,
    pub cur: usize,
    pub heap: BinaryHeap<Reverse<(u64, u64, usize)>>,
    pub tasks: Vec<Option<Task>>,
    pub rng: Rng,
    pub c: Cfg,
    pub db: Db,
    pub sess: Vec<Sess>,
    pub pools: Vec<Pool>,
    pub srv: Vec<Srv>,
    pub ids: u64,
    pub hand: bool,
    pub fails: BTreeMap<&'static str, (u32, String)>,
    pub cnt: BTreeMap<&'static str, u64>,
    pub seen_copy: HashSet<u64>,
    pub sc_copy: HashSet<u64>,
    pub app_copy: HashSet<u64>,
    pub noreset_at: HashMap<u32, u64>,
    pub connfail: HashMap<u64, u32>,
}

impl Sim {
    pub fn new(c: Cfg, seed: u64) -> Box<Sim> {
        let mut s = Box::new(Sim {
            now: 0,
            seq: 0,
            cur: 0,
            heap: BinaryHeap::new(),
            tasks: vec![],
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x55AA_1234_77),
            c: c.clone(),
            db: Db {
                n: 1,
                nhist: vec![(0, 1)],
                replay: vec![],
                types: vec![(TS::Cur, 0, u64::MAX)],
                col: 0,
                colhist: vec![0],
                resets: 0,
                refresh: 0,
                stored_bad: false,
                last_change: 0,
                refused_until: 0,
                last_refusal_end: 0,
                last_reconnect: 0,
            },
            sess: vec![],
            pools: vec![],
            srv: vec![],
            ids: 0,
            hand: false,
            fails: BTreeMap::new(),
            cnt: BTreeMap::new(),
            seen_copy: HashSet::new(),
            sc_copy: HashSet::new(),
            app_copy: HashSet::new(),
            noreset_at: HashMap::new(),
            connfail: HashMap::new(),
        });
        s.db.replay = (0..c.servers).map(|_| vec![(0, 1)]).collect();
        for _ in 0..c.procs {
            s.pools.push(Pool { n: 1, tickets: 1, horizon: 1, highest: 1, marker: (1, 0), step_n0: 1, ..Pool::default() });
        }
        if c.pooled {
            for _ in 0..3 {
                let id = s.new_sess();
                s.srv.push(Srv { sess: id, busy: false, kill: false });
            }
        }
        s
    }
    pub fn id(&mut self) -> u64 {
        self.ids += 1;
        self.ids
    }
    pub fn new_sess(&mut self) -> usize {
        let o = self.now;
        self.sess.push(Sess { opened: o, f: None, s: None, h: None, b: None, copies: vec![] });
        self.sess.len() - 1
    }
    pub fn fail(&mut self, k: &'static str, m: String) {
        let e = self.fails.entry(k).or_insert((0, m));
        e.0 += 1;
    }
    pub fn wake(&mut self, p: usize) {
        if let Some(t) = self.pools[p].waiters.pop_front() {
            self.seq += 1;
            let (n, q) = (self.now, self.seq);
            self.heap.push(Reverse((n, q, t)));
        }
    }
    pub fn inc(&mut self, k: &'static str) {
        *self.cnt.entry(k).or_insert(0) += 1;
    }
    pub fn cur_ver(&self) -> u32 {
        (self.db.types.len() - 1) as u32
    }
    pub fn spawn(&mut self, t: Task, at: u64) {
        let id = self.tasks.len();
        self.tasks.push(Some(t));
        self.seq += 1;
        let q = self.seq;
        self.heap.push(Reverse((at, q, id)));
    }

    // ---- Postgres ----
    pub fn enum_in(&self, v: u32, l: Label) -> Result<u32, Err> {
        let (st, _, _) = self.db.types[v as usize];
        let ret = matches!(l, Label::RetX | Label::RetReal);
        let named = match st {
            TS::Cur => Named::Cur,
            TS::Kept => Named::Kept(v),
            TS::Retired => Named::Ret(v),
            TS::Dropped => return Err(Err::XX),
        };
        let has = match (st, l) {
            (TS::Retired, Label::RetReal) => true,
            (TS::Retired, _) => false,
            (_, Label::Shared) => true,
            (_, Label::New(x)) => x <= v,
            (_, Label::Newest | Label::NewName) => v == self.cur_ver(),
            _ => false,
        };
        if has { Ok(v) } else { Err(Err::P22 { named, ret: ret || matches!(named, Named::Ret(_)), nm: l == Label::NewName }) }
    }
    pub fn stored(&self) -> Label {
        if self.db.stored_bad { Label::RetX } else { Label::Shared }
    }
    pub fn oracle(&self, sp: &Spec) -> Result<u32, u8> {
        let cur = self.cur_ver();
        match sp.stmt {
            CAST | F | TRIG => self.enum_in(cur, sp.label).map_err(|e| e.code()),
            ST => self.enum_in(cur, self.stored()).map_err(|e| e.code()),
            _ => Ok(0),
        }
    }
    fn comp(&mut self, sid: usize, f: bool) -> Comp {
        let (cur, rf, now) = (self.cur_ver(), self.db.refresh, self.now);
        let slot = if f { &mut self.sess[sid].f } else { &mut self.sess[sid].s };
        match slot {
            Some(c) if c.refresh == rf => *c,
            _ => {
                let v = if self.c.fault == 41 { self.db.types.iter().rposition(|t| t.0 == TS::Retired).map_or(cur, |x| x as u32) } else { cur };
                let c = Comp { ver: v, refresh: rf, at: now };
                *slot = Some(c);
                c
            }
        }
    }
    // how: Some(text) runs a cached copy of that text, preparing it on a miss; keep: Some(text) drops any copy of
    // that text and prepares it again; neither: unnamed
    pub fn exec(&mut self, sid: usize, sp: &Spec, how: Option<Text>, keep: Option<Text>, by: usize) -> Out {
        let now = self.now;
        let cur = self.cur_ver();
        let mut copy: Option<Copy> = None;
        let mut was = false;
        if let Some(t) = how.or(keep) {
            if how.is_none() {
                self.sess[sid].copies.retain(|(x, _)| *x != t);
            }
            let pos = self.sess[sid].copies.iter().position(|(x, _)| *x == t);
            if let Some(i) = pos {
                let x = self.sess[sid].copies.remove(i);
                self.sess[sid].copies.push(x);
                was = true;
            } else {
                let id = self.id();
                let c = Copy { id, at: now, ty: cur, resets: self.db.resets, planned: now, col: self.db.col, path: sp.path, by };
                self.sess[sid].copies.push((t, c));
                if self.c.pooled && self.sess[sid].copies.len() > 16 {
                    self.sess[sid].copies.remove(0);
                }
            }
            let rs = self.db.resets;
            let l = self.sess[sid].copies.last_mut().unwrap();
            if l.1.resets != rs {
                l.1.resets = rs;
                l.1.ty = cur;
                l.1.planned = now;
            }
            copy = Some(l.1.clone());
        }
        let oracle = self.oracle(sp);
        let mut comp = None;
        let mut ver = cur;
        let (r, cause) = match sp.stmt {
            CAST => {
                let v = copy.as_ref().map_or(cur, |c| c.ty);
                ver = v;
                let r = self.enum_in(v, sp.label);
                let cause = match r {
                    Err(_) if v != cur => Cause::Stmt(self.db.types[v as usize].1),
                    _ => Cause::None,
                };
                (r, cause)
            }
            TRIG => {
                let v = copy.as_ref().map_or(cur, |c| c.ty);
                ver = v;
                match self.enum_in(v, sp.label) {
                    Err(e) => (Err(e), if v != cur { Cause::Stmt(self.db.types[v as usize].1) } else { Cause::None }),
                    Ok(_) => {
                        let c = self.comp(sid, true);
                        comp = Some(c);
                        ver = c.ver;
                        let r = self.enum_in(c.ver, sp.label);
                        let cause = match r {
                            Err(Err::P22 { named: Named::Kept(x), .. }) => Cause::Kept(x),
                            Err(_) if c.ver != cur => Cause::Sess(self.db.types[c.ver as usize].1),
                            _ => Cause::None,
                        };
                        (r, cause)
                    }
                }
            }
            COL | PATH | BOTH => {
                let a0 = match &copy {
                    Some(c) if sp.stmt == PATH && c.path != sp.path => Some(Cause::Path),
                    Some(c) if c.col != self.db.col && (sp.stmt != PATH || sp.path == 0) => Some(Cause::Stmt(self.db.colhist[(c.col + 1) as usize])),
                    _ => None,
                };
                match a0 {
                    Some(x) => (Err(Err::A0), x),
                    None if sp.stmt == BOTH => {
                        let (cv, rf) = (self.db.col, self.db.refresh);
                        match self.sess[sid].b {
                            Some((c, r)) if r == rf && c != cv => (Err(Err::D42804), Cause::Sess(self.db.colhist[(c + 1) as usize])),
                            Some((_, r)) if r == rf => (Ok(0), Cause::None),
                            _ => {
                                self.sess[sid].b = Some((cv, rf));
                                (Ok(0), Cause::None)
                            }
                        }
                    }
                    None => (Ok(0), Cause::None),
                }
            }
            F | ST => {
                let c = self.comp(sid, sp.stmt == F);
                comp = Some(c);
                ver = c.ver;
                let l = if sp.stmt == F { sp.label } else { self.stored() };
                let r = self.enum_in(c.ver, l);
                let cause = match r {
                    Err(Err::P22 { named: Named::Kept(v), .. }) => Cause::Kept(v),
                    Err(_) if c.ver != cur => Cause::Sess(self.db.types[c.ver as usize].1),
                    _ => Cause::None,
                };
                (r, cause)
            }
            _ => {
                let cv = if sp.shape == 0 { self.db.col } else { 0 };
                let rf = self.db.refresh;
                match self.sess[sid].h {
                    Some((sh, c, r, at)) if r == rf => {
                        comp = Some(Comp { ver: c, refresh: r, at });
                        if sh != sp.shape {
                            (Err(Err::D42804), Cause::Data)
                        } else if c != cv {
                            (Err(Err::D42804), Cause::Sess(self.db.colhist[(c + 1) as usize]))
                        } else {
                            (Ok(0), Cause::None)
                        }
                    }
                    _ => {
                        self.sess[sid].h = Some((sp.shape, cv, rf, now));
                        (Ok(0), Cause::None)
                    }
                }
            }
        };
        if !matches!(sp.stmt, CAST | F | ST | TRIG) {
            ver = 0;
        }
        Out {
            r,
            cause,
            oracle,
            cached: was,
            copy: copy.as_ref().map(|c| c.id),
            copy_at: copy.as_ref().map_or(0, |c| c.planned),
            copy_by: copy.as_ref().map_or(by, |c| c.by),
            comp,
            sess_opened: self.sess[sid].opened,
            t: now,
            ver,
        }
    }
    pub fn visible(&self, o: &Out) -> bool {
        match (o.r, o.oracle) {
            (Err(e), Err(x)) => e.code() != x,
            (Err(_), Ok(_)) => true,
            _ => false,
        }
    }
    // the change that left the copy a run used stale, if any
    pub fn copy_change(&self, sp: &Spec, o: &Out) -> Option<u64> {
        match sp.stmt {
            CAST if o.cached && o.ver != self.cur_ver() => Some(self.db.types[o.ver as usize].2),
            COL | PATH | BOTH => match o.cause {
                Cause::Stmt(t) if o.r == Err(Err::A0) => Some(t),
                _ => None,
            },
            _ => None,
        }
    }
}

pub fn run_loop(s: &mut Box<Sim>) {
    SIM.with(|c| c.set(&mut **s as *mut Sim));
    let waker = Waker::noop();
    loop {
        let next = {
            let m = w();
            match m.heap.pop() {
                None => break,
                Some(Reverse((t, _, id))) => {
                    m.now = t;
                    m.cur = id;
                    let k = m.tasks[id].take();
                    (id, k)
                }
            }
        };
        if let (id, Some(mut t)) = next {
            let mut cx = Context::from_waker(waker);
            if t.as_mut().poll(&mut cx).is_pending() {
                w().tasks[id] = Some(t);
            }
        }
    }
}
