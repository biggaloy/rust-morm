use morm_tests::{Transport, connect, connect_to, simple_rows, target};

// covers: R-0792
#[tokio::test]
async fn queries_pass_through_pgbouncer_in_transaction_mode() {
    assert_eq!(target().transport, Transport::PgBouncer);
    let client = connect().await;
    simple_rows(&client, "select 1").await;

    let admin_dsn = std::env::var("MORM_TEST_PGBOUNCER_ADMIN_DSN")
        .expect("MORM_TEST_PGBOUNCER_ADMIN_DSN must name the admin console");
    let admin = connect_to(&admin_dsn).await;
    let config = simple_rows(&admin, "SHOW CONFIG").await;
    let pool_mode = config
        .iter()
        .find(|row| row.get("key") == Some("pool_mode"))
        .and_then(|row| row.get("value"));
    assert_eq!(pool_mode, Some("transaction"));
    let clients = simple_rows(&admin, "SHOW CLIENTS").await;
    assert!(
        clients
            .iter()
            .any(|c| c.get("database") == Some("postgres")),
        "the test's connection isn't going through this PgBouncer"
    );
}
