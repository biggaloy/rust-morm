//! Proof 5: hooks are `async fn` in the impl, the futures are `Send` under
//! Axum, `#[morm::hooks]` records which hooks exist, and the lazy `db` checks
//! out nothing unless a hook queries.

use app::{Note, Product};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use morm::{Db, Hooks, Overrides, insert};
use tower::ServiceExt;

#[test]
fn the_macro_records_overrides() {
    assert_eq!(
        Note::OVERRIDES,
        Overrides::NONE
            .with(Overrides::VALIDATE)
            .with(Overrides::PRE_INSERT)
    );
    assert!(!Note::OVERRIDES.has(Overrides::POST_INSERT));
    assert_eq!(Product::OVERRIDES, Overrides::NONE);
}

/// Both database tests use `spike_note`, so they take turns.
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> Db {
    let admin = app::admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS spike_note; CREATE TABLE spike_note (id int8, body text)",
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
