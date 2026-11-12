//! Proof 5: hooks are `async fn` in the impl, the futures are `Send` under
//! Axum, `#[morm::hooks]` records which hooks exist, and the lazy `db` checks
//! out nothing unless a hook queries.

use app::{Note, Product};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use morm::{Changes, Db, Hook, Hooks, Target, delete, insert, update};
use tower::ServiceExt;

#[test]
fn the_macro_records_overrides() {
    let all = [
        Hook::Validate,
        Hook::PreInsert,
        Hook::PostInsert,
        Hook::ValidateChanges,
        Hook::PreUpdate,
        Hook::PostUpdate,
        Hook::PreDelete,
        Hook::PostDelete,
        Hook::AfterCommit,
    ];
    let note: Vec<Hook> = all
        .into_iter()
        .filter(|h| Note::OVERRIDES.has(*h))
        .collect();
    assert_eq!(
        note,
        all.into_iter()
            .filter(|h| *h != Hook::PostInsert)
            .collect::<Vec<_>>()
    );
    assert!(all.iter().all(|h| !Product::OVERRIDES.has(*h)));
}

/// Both database tests use `spike_note`, so they take turns.
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> Db {
    let admin = app::admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS spike_note; CREATE TABLE spike_note (id int8, body text, edited bool NOT NULL DEFAULT false)",
        )
        .await
        .unwrap();
    Db::connect(&app::dsn(), 2).unwrap()
}

/// One test, so the checkout counts aren't shared with another test.
#[tokio::test]
async fn hooks_check_out_only_when_they_query() {
    let _turn = LOCK.lock().await;
    let db = setup().await;
    let note = |id, body: &str| Note {
        id,
        body: body.into(),
    };

    // `validate` fails before anything runs.
    assert!(insert(&db, &mut [note(1, "")]).await.is_err());
    assert_eq!(db.checkouts(), 0);

    // `pre_insert` refuses without a query: nothing was checked out.
    assert!(insert(&db, &mut [note(2, " reject ")]).await.is_err());
    assert_eq!(db.checkouts(), 0);

    // A hook with no query, then the insert: the insert's one checkout.
    let mut rows = [note(3, "  padded  ")];
    assert_eq!(insert(&db, &mut rows).await.unwrap(), 1);
    assert_eq!(rows[0].body, "padded");
    assert_eq!(db.checkouts(), 1);

    // A hook that queries: it checks out, and the insert reuses that connection.
    assert_eq!(insert(&db, &mut [note(4, "lookup")]).await.unwrap(), 1);
    assert_eq!(db.checkouts(), 2);

    // A model with no hooks: only the insert's checkout.
    let admin = app::admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS spike_customer; CREATE TABLE spike_customer (id int8, name text)",
        )
        .await
        .unwrap();
    let mut c = [models_b::Customer {
        id: 1,
        name: "a".into(),
    }];
    assert_eq!(insert(&db, &mut c).await.unwrap(), 1);
    assert_eq!(db.checkouts(), 3);
}

async fn create(State(db): State<Db>, Json(body): Json<String>) -> Json<u64> {
    let mut rows = [Note { id: 10, body }];
    Json(insert(&db, &mut rows).await.unwrap())
}

/// Axum needs every handler future to be `Send`; this handler awaits a
/// generic insert that awaits the user's `async fn` hooks.
#[tokio::test]
async fn hook_futures_are_send_under_axum() {
    let _turn = LOCK.lock().await;
    let db = setup().await;
    let app = Router::new()
        .route("/notes", post(create))
        .with_state(db.clone());
    let req = axum::http::Request::post("/notes")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("\" from axum \""))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let rows = morm::Conn::query(&db, "SELECT body FROM spike_note WHERE id = 10", &[])
        .await
        .unwrap();
    assert_eq!(rows[0].get::<_, &str>(0), "from axum");
}

/// The contract's update and delete hooks, written as `async fn`, run through
/// a generic update and delete, then `after_commit` gets the event.
#[tokio::test]
async fn update_and_delete_hooks_and_after_commit_run() {
    let _turn = LOCK.lock().await;
    let db = setup().await;
    app::HOOK_LOG.lock().unwrap().clear();
    let mut rows = [
        Note {
            id: 1,
            body: "a".into(),
        },
        Note {
            id: 2,
            body: "b".into(),
        },
    ];
    insert(&db, &mut rows).await.unwrap();

    // `validate_changes` refuses before anything is checked out.
    let before = db.checkouts();
    let bad = Changes::new().set("id", 9i64);
    assert!(update::<Note>(&db, bad, Target::Id(1)).await.is_err());
    // `pre_update` refuses an update of every row.
    let all = Changes::new().set("body", String::from("x"));
    assert!(update::<Note>(&db, all, Target::All).await.is_err());
    assert_eq!(db.checkouts(), before);

    // `pre_update` adds a change; the update and both hooks share one checkout.
    let ch = Changes::new().set("body", String::from("new"));
    assert_eq!(update::<Note>(&db, ch, Target::Id(1)).await.unwrap(), 1);
    assert_eq!(db.checkouts(), before + 1);
    let r = morm::Conn::query(&db, "SELECT body, edited FROM spike_note WHERE id = 1", &[])
        .await
        .unwrap();
    assert_eq!(
        (r[0].get::<_, &str>(0), r[0].get::<_, bool>(1)),
        ("new", true)
    );

    assert!(delete::<Note>(&db, Target::All).await.is_err());
    assert_eq!(
        delete::<Note>(&db, Target::Ids(vec![1, 2])).await.unwrap(),
        2
    );
    assert_eq!(
        *app::HOOK_LOG.lock().unwrap(),
        [
            "post_update body,edited 1",
            "committed update 1",
            "post_delete 2",
            "committed delete [1, 2]"
        ]
    );
}

async fn edit(State(db): State<Db>, Json(body): Json<String>) -> Json<(u64, u64)> {
    let ch = Changes::new().set("body", body);
    let n = update::<Note>(&db, ch, Target::Id(20)).await.unwrap();
    let d = delete::<Note>(&db, Target::Id(20)).await.unwrap();
    Json((n, d))
}

/// An Axum handler awaiting a generic update and delete, which await the
/// user's update and delete hooks: their futures are `Send`.
#[tokio::test]
async fn update_and_delete_hook_futures_are_send_under_axum() {
    let _turn = LOCK.lock().await;
    let db = setup().await;
    insert(
        &db,
        &mut [Note {
            id: 20,
            body: "old".into(),
        }],
    )
    .await
    .unwrap();
    let app = Router::new()
        .route("/edit", post(edit))
        .with_state(db.clone());
    let req = axum::http::Request::post("/edit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("\"edited\""))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let body = axum::body::to_bytes(res.into_body(), 64).await.unwrap();
    assert_eq!(&body[..], b"[1,1]");
}
