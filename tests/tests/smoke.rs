use morm_tests::{assert_on_target, connect, simple_rows};

// covers: R-0789
#[tokio::test]
async fn a_query_runs_on_the_target() {
    let client = connect().await;
    assert_on_target(&client).await;
    let rows = simple_rows(&client, "select 1").await;
    assert_eq!(rows[0].get(0), Some("1"));
}
