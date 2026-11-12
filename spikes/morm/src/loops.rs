//! Proof 11: spotting the same statement repeating in a short window on one task.
//!
//! Each worker thread keeps a few slots keyed by (task, statement). A statement
//! is its `&'static str`'s address, so no hashing or string compare. A task that
//! moves thread starts counting again on the new one: that can miss a loop,
//! never invent one.

use std::cell::RefCell;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::task::Id;

const SLOTS: usize = 8;

#[derive(Clone, Copy)]
struct Slot {
    task: Option<Id>,
    stmt: usize,
    since: Instant,
    count: u32,
    warned: bool,
}

thread_local! {
    static TABLE: RefCell<([Option<Slot>; SLOTS], usize)> = const { RefCell::new(([None; SLOTS], 0)) };
}

pub struct Detector {
    window: Duration,
    threshold: u32,
    /// Stands in for the internal warning sink: what was reported.
    pub warnings: Mutex<Vec<&'static str>>,
}

impl Detector {
    pub fn new(window: Duration, threshold: u32) -> Self {
        Self {
            window,
            threshold,
            warnings: Mutex::new(Vec::new()),
        }
    }

    /// Called once per statement run. Returns true the first time a loop is seen.
    #[inline]
    pub fn observe(&self, sql: &'static str) -> bool {
        let task = tokio::task::try_id();
        let stmt = sql.as_ptr() as usize;
        let now = Instant::now();
        let hit = TABLE.with_borrow_mut(|(slots, next)| {
            for s in slots.iter_mut().flatten() {
                if s.stmt == stmt && s.task == task {
                    if now.duration_since(s.since) > self.window {
                        s.since = now;
                        s.count = 1;
                        s.warned = false;
                        return false;
                    }
                    s.count += 1;
                    if s.count >= self.threshold && !s.warned {
                        s.warned = true;
                        return true;
                    }
                    return false;
                }
            }
            slots[*next] = Some(Slot {
                task,
                stmt,
                since: now,
                count: 1,
                warned: false,
            });
            *next = (*next + 1) % SLOTS;
            false
        });
        if hit {
            self.warnings.lock().unwrap().push(sql);
        }
        hit
    }
}
