//! Proof 8: conflict targets are typed markers. A unique column that arrives
//! through two base expansions in two crates is a key, `UK` constants cover
//! expression, partial and composite keys, and Postgres accepts each target.
//! A column with no unique key is a compile error: see `ui/`.

use models_a::Order;
use morm::{Conn, Db, Model, Param, UniqueKey, upsert_sql};

#[test]
fn conflict_targets() {
    fn target<K: UniqueKey<Order>>(_: K) -> &'static str {
        K::CONFLICT
    }
    assert_eq!(target(Order::F.external_id), r#"("external_id")"#);
    assert_eq!(target(Order::UK.lower_email), r#"(lower("email"))"#);
    assert_eq!(
        target(Order::UK.live_code),
        r#"("code") WHERE "deleted" = false"#
    );
    assert_eq!(target(Order::UK.tenant_code), r#"("tenant", "code")"#);
}

fn order(ext: &str, email: &str, code: &str, tenant: i64, deleted: bool, note: &str) -> Order {
    Order {
        created_at: 1,
        updated_at: 1,
        created_by: 1,
        external_id: ext.into(),
        id: 1,
        tenant,
        code: code.into(),
        email: email.into(),
        deleted,
        note: Some(note.into()),
    }
}

async fn upsert<K: UniqueKey<Order>>(db: &Db, key: K, row: &Order) -> u64 {
    let sql: &'static str = upsert_sql::<Order, K>(key, &["note"]).leak();
    db.query(sql, &row.params()).await.unwrap();
    let rows = db
        .query("SELECT count(*) FROM spike_order", &[])
        .await
        .unwrap();
    rows[0].get::<_, i64>(0) as u64
}

#[tokio::test]
async fn postgres_accepts_every_target() {
    let admin = app::admin().await;
    admin
        .batch_execute(
            r#"DROP TABLE IF EXISTS spike_order;
               CREATE TABLE spike_order (created_at int8, updated_at int8, created_by int8,
                 external_id text UNIQUE, id int8, tenant int8, code text, email text,
                 deleted bool, note text);
               CREATE UNIQUE INDEX ON spike_order (lower(email));
               CREATE UNIQUE INDEX ON spike_order (code) WHERE deleted = false;
               CREATE UNIQUE INDEX ON spike_order (tenant, code);"#,
        )
        .await
        .unwrap();
    let db = Db::connect(&app::dsn(), 2).unwrap();
    let base = order("e1", "a@x", "c1", 1, false, "first");
    let p: Vec<&dyn Param> = base.params();
    db.query(Order::INSERT, &p).await.unwrap();

    // Each row conflicts with `base` on one key only, so each target must be
    // the arbiter Postgres picks; any other unique violation would error.
    assert_eq!(
        upsert(
            &db,
            Order::F.external_id,
            &order("e1", "b@x", "c2", 1, false, "by ext")
        )
        .await,
        1
    );
    assert_eq!(
        upsert(
            &db,
            Order::UK.lower_email,
            &order("e2", "A@X", "c3", 1, false, "by email")
        )
        .await,
        1
    );
    assert_eq!(
        upsert(
            &db,
            Order::UK.live_code,
            &order("e3", "c@x", "c1", 2, false, "by live code")
        )
        .await,
        1
    );
    assert_eq!(
        upsert(
            &db,
            Order::UK.tenant_code,
            &order("e4", "d@x", "c1", 1, true, "by tenant code")
        )
        .await,
        1
    );
    let rows = db.query("SELECT note FROM spike_order", &[]).await.unwrap();
    assert_eq!(rows[0].get::<_, &str>(0), "by tenant code");

    // And a row that conflicts on nothing is inserted.
    assert_eq!(
        upsert(
            &db,
            Order::UK.tenant_code,
            &order("e5", "e@x", "c9", 9, false, "new")
        )
        .await,
        2
    );
}
