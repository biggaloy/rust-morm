//! The app side of the spikes: models, groups and hooks that the tests use.

use morm::{Changes, Committed, Conn, Error, Result, Target};
// Proof 3: models find the app's global groups through a plain `use`.
use groups::G;

/// Proof 3: the app's global groups. With `sudo` that's 4, so `Bits<1>`.
pub mod groups {
    morm::groups! { admin, staff, support }
}

/// Proof 3: 64 named groups plus `sudo` is 65, so the macro picks `Bits<2>`.
pub mod big {
    morm::groups! {
        g01, g02, g03, g04, g05, g06, g07, g08, g09, g10, g11, g12, g13, g14, g15, g16,
        g17, g18, g19, g20, g21, g22, g23, g24, g25, g26, g27, g28, g29, g30, g31, g32,
        g33, g34, g35, g36, g37, g38, g39, g40, g41, g42, g43, g44, g45, g46, g47, g48,
        g49, g50, g51, g52, g53, g54, g55, g56, g57, g58, g59, g60, g61, g62, g63, g64,
    }
}

/// Proofs 1, 3 and 6: two views, a global and a scoped group, a borrowed form.
#[morm::model(table = "spike_product", groups(editor, reviewer), borrowed)]
#[view(ProductPublic, hide(cost, supplier))]
#[view(ProductCard, only(id, name))]
#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub id: i64,
    pub name: String,
    #[col(group = admin)]
    pub cost: i64,
    #[col(group = editor)]
    pub supplier: String,
    pub photo: Option<Vec<u8>>,
    pub tags: Option<String>,
}

/// Another model with its own scoped group, for proof 3's compile errors.
#[morm::model(table = "spike_thing", groups(owner))]
pub struct Thing {
    pub id: i64,
    #[col(group = owner)]
    pub secret: String,
}

/// Proof 2: a base used in its own crate, from the root and from a module.
#[morm::base]
pub struct Owned {
    pub owner: i64,
}

pub mod inner {
    #[morm::base(extends = crate::Owned)]
    pub struct Flagged {
        pub flag: bool,
    }
}

/// Two bases with the same name in one crate: each macro's name carries
/// where its base is written, so they don't collide at the crate root.
pub mod left {
    #[morm::base]
    pub struct Tag {
        pub left: i64,
    }
}
pub mod right {
    #[morm::base]
    pub struct Tag {
        pub right: i64,
    }
}

#[morm::model(table = "spike_left", base = crate::left::Tag)]
pub struct LeftTagged {
    pub id: i64,
}

#[morm::model(table = "spike_right", base = crate::right::Tag)]
pub struct RightTagged {
    pub id: i64,
}

#[morm::model(table = "spike_doc", base = crate::inner::Flagged)]
pub struct Doc {
    pub id: i64,
}

/// Proof 5: hooks written as `async fn`, recorded by `#[morm::hooks]`.
#[morm::model(table = "spike_note", hooks)]
#[derive(Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub body: String,
}

#[morm::hooks]
impl morm::Hooks for Note {
    fn validate(&self) -> Result<()> {
        if self.body.is_empty() {
            return Err(Error::Validation("body is empty"));
        }
        Ok(())
    }

    /// Trims every body. Runs a query only for a body of `lookup`, and
    /// refuses `reject` before any query.
    async fn pre_insert(rows: &mut [Self], db: &impl Conn) -> Result<()> {
        for r in rows.iter_mut() {
            r.body = r.body.trim().to_owned();
        }
        if rows.iter().any(|r| r.body == "reject") {
            return Err(Error::Validation("rejected"));
        }
        if rows.iter().any(|r| r.body == "lookup") {
            db.query("SELECT 1", &[]).await?;
        }
        Ok(())
    }

    fn validate_changes(changes: &Changes<Self>) -> Result<()> {
        if changes.columns().any(|c| c == "id") {
            return Err(Error::Validation("id can't change"));
        }
        Ok(())
    }

    /// Refuses an update of every note, and stamps the rest as edited.
    async fn pre_update(
        changes: &mut Changes<Self>,
        target: &Target<Self>,
        db: &impl Conn,
    ) -> Result<()> {
        if matches!(target, Target::All) {
            return Err(Error::Validation("update of every note"));
        }
        db.query("SELECT 1", &[]).await?;
        changes.put("edited", true);
        Ok(())
    }

    async fn post_update(
        changes: &Changes<Self>,
        _target: &Target<Self>,
        affected: u64,
        db: &impl Conn,
    ) -> Result<()> {
        db.query("SELECT 1", &[]).await?;
        let cols: Vec<_> = changes.columns().collect();
        log(format!("post_update {} {affected}", cols.join(",")));
        Ok(())
    }

    async fn pre_delete(target: &Target<Self>, _db: &impl Conn) -> Result<()> {
        if matches!(target, Target::All) {
            return Err(Error::Validation("delete of every note"));
        }
        Ok(())
    }

    async fn post_delete(_target: &Target<Self>, affected: u64, _db: &impl Conn) -> Result<()> {
        log(format!("post_delete {affected}"));
        Ok(())
    }

    async fn after_commit(event: Committed<Self>) -> Result<()> {
        log(match event {
            Committed::Updated { target, .. } => format!("committed update {}", ids(&target)),
            Committed::Deleted { target } => format!("committed delete {}", ids(&target)),
            _ => "committed other".into(),
        });
        Ok(())
    }
}

fn ids(t: &Target<Note>) -> String {
    match t {
        Target::Id(id) => id.to_string(),
        Target::Ids(ids) => format!("{ids:?}"),
        _ => "other".into(),
    }
}

/// What `Note`'s update, delete and commit hooks saw, in order.
pub static HOOK_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn log(s: String) {
    HOOK_LOG.lock().unwrap().push(s);
}

/// The DSN the spikes run against. Required: a proof never skips silently.
pub fn dsn() -> String {
    std::env::var("SPIKE_DSN").expect("set SPIKE_DSN to a Postgres 15 or later DSN")
}

pub async fn admin() -> tokio_postgres::Client {
    let (c, conn) = tokio_postgres::connect(&dsn(), tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    c
}
