//! Proof 7: refusing a second connection on a task that is streaming rows.
//!
//! A stream registers its task on first poll, and again whenever it is polled
//! from another task. A checkout reads one atomic; only when some stream is
//! open anywhere does it look up the current task.

use crate::pool::Object;
use crate::{Db, Error, Result};
use futures_util::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use tokio::task::Id;
use tokio_postgres::Row;

/// Streams registered to some task, anywhere in the process.
static OPEN: AtomicUsize = AtomicUsize::new(0);
/// Which tasks hold an open stream, with a count each.
static TASKS: Mutex<Vec<(Id, usize)>> = Mutex::new(Vec::new());

/// The check every checkout makes. With no stream open it is one relaxed load.
#[inline]
pub fn check() -> Result<()> {
    if OPEN.load(Ordering::Relaxed) == 0 {
        return Ok(());
    }
    check_task()
}

#[cold]
fn check_task() -> Result<()> {
    let Some(id) = tokio::task::try_id() else {
        return Ok(());
    };
    if TASKS.lock().unwrap().iter().any(|(t, _)| *t == id) {
        return Err(Error::NestedConnection);
    }
    Ok(())
}

pub fn open_streams() -> usize {
    OPEN.load(Ordering::Relaxed)
}

fn add(id: Id) {
    let mut t = TASKS.lock().unwrap();
    match t.iter_mut().find(|(x, _)| *x == id) {
        Some((_, n)) => *n += 1,
        None => t.push((id, 1)),
    }
}

fn remove(id: Id) {
    let mut t = TASKS.lock().unwrap();
    if let Some(i) = t.iter().position(|(x, _)| *x == id) {
        t[i].1 -= 1;
        if t[i].1 == 0 {
            t.swap_remove(i);
        }
    }
}

/// One stream's registration.
#[derive(Default)]
struct Reg {
    task: Option<Id>,
}

impl Reg {
    /// Called on every poll: registers on the first, moves on a task change.
    #[inline]
    fn on_poll(&mut self) {
        let now = tokio::task::try_id();
        if now == self.task {
            return;
        }
        match (self.task, now) {
            (None, Some(n)) => {
                add(n);
                OPEN.fetch_add(1, Ordering::Relaxed);
            }
            (Some(o), Some(n)) => {
                remove(o);
                add(n);
            }
            (Some(o), None) => {
                remove(o);
                OPEN.fetch_sub(1, Ordering::Relaxed);
            }
            (None, None) => {}
        }
        self.task = now;
    }
}

impl Drop for Reg {
    fn drop(&mut self) {
        if let Some(o) = self.task.take() {
            remove(o);
            OPEN.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

type Opening = Pin<Box<dyn Future<Output = Result<(Object, tokio_postgres::RowStream)>> + Send>>;

enum State {
    Idle(Db, &'static str),
    Opening(Opening),
    Rows(#[allow(dead_code)] Object, Pin<Box<tokio_postgres::RowStream>>),
    Done,
}

/// `db.stream(sql)`: checks nothing out until first polled.
pub struct RowStream {
    reg: Reg,
    state: State,
}

impl Db {
    pub fn stream(&self, sql: &'static str) -> RowStream {
        RowStream {
            reg: Reg::default(),
            state: State::Idle(self.clone(), sql),
        }
    }
}

impl Stream for RowStream {
    type Item = Result<Row>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                State::Idle(db, sql) => {
                    // A stream inside another stream's loop is a nested checkout too.
                    if let Err(e) = check() {
                        this.state = State::Done;
                        return Poll::Ready(Some(Err(e)));
                    }
                    this.reg.on_poll();
                    let (db, sql) = (db.clone(), *sql);
                    this.state = State::Opening(Box::pin(async move {
                        let c = db.checkout_unchecked().await?;
                        let rows = c.query_raw(sql, std::iter::empty::<i32>()).await?;
                        Ok((c, rows))
                    }));
                }
                State::Opening(f) => {
                    this.reg.on_poll();
                    match f.as_mut().poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => {
                            this.state = State::Done;
                            this.reg = Reg::default();
                            return Poll::Ready(Some(Err(e)));
                        }
                        Poll::Ready(Ok((c, rows))) => this.state = State::Rows(c, Box::pin(rows)),
                    }
                }
                State::Rows(_, rows) => {
                    this.reg.on_poll();
                    return match rows.as_mut().poll_next(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(Some(r)) => Poll::Ready(Some(r.map_err(Error::from))),
                        Poll::Ready(None) => {
                            // Finished: the connection goes back and the task is free.
                            this.state = State::Done;
                            this.reg = Reg::default();
                            Poll::Ready(None)
                        }
                    };
                }
                State::Done => return Poll::Ready(None),
            }
        }
    }
}
