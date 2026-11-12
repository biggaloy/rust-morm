//! Proof 9: `morm::Param` with `query_typed`, for every type the spike maps,
//! `Option<T>`, arrays and `NULL`, all through one `&[&dyn Param]` with no
//! boxing. An enum label goes as `Type::UNKNOWN`, so no type OID is sent.

use bytes::BytesMut;
use morm::{Conn, Db, Param};
use rust_decimal::Decimal;
use std::error::Error as StdError;
use std::time::{Duration, SystemTime};
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};
use uuid::Uuid;

/// What the plan's `PgEnum` derive would write: the label as text, any type.
#[derive(Debug)]
struct Mood(&'static str);

impl ToSql for Mood {
    fn to_sql(
        &self,
        _: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(IsNull::No)
    }
    fn accepts(_: &Type) -> bool {
        true
    }
    to_sql_checked!();
}

impl Param for Mood {
    fn pg_type(&self) -> Type {
        Type::UNKNOWN
    }
}

/// Sends every value with its declared type and reads each one back as the
/// same Rust type.
#[tokio::test]
async fn every_type_round_trips_through_dyn_param() {
    let db = Db::connect(&app::dsn(), 1).unwrap();
    let when = SystemTime::UNIX_EPOCH + Duration::from_micros(1_700_000_000_123_456);
    let id = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
    let json = serde_json::json!({"a": [1, 2, {"b": null}]});
    let dec: Decimal = "12345678901234567890.123456789".parse().unwrap();
    let text = String::from("text");
    let bytes: Vec<u8> = vec![0, 1, 255];

    let (b, i2, i4, i8_, f4, f8) = (true, -2i16, 4i32, -8i64, 1.5f32, -2.25f64);
    let (s, sr, by, byr) = (&text, "borrowed", &bytes, &bytes[..]);
    let (none_i, none_s, none_u, none_j, none_d, none_t): (
        Option<i64>,
        Option<String>,
        Option<Uuid>,
        Option<serde_json::Value>,
        Option<Decimal>,
        Option<SystemTime>,
    ) = (None, None, None, None, None, None);
    let some = Some(7i32);
    let arr = vec![1i64, 2, 3];
    let arr_null = vec![Some(1i32), None, Some(3)];
    let arr_text = vec![String::from("a"), String::from("b")];
    let arr_slice: &[i16] = &[5, 6];
    let empty: Vec<Uuid> = Vec::new();

    let params: [&dyn Param; 27] = [
        &b,
        &i2,
        &i4,
        &i8_,
        &f4,
        &f8,
        s,
        &sr,
        by,
        &byr,
        &id,
        &json,
        &dec,
        &when, // 1..14
        &none_i,
        &none_s,
        &none_u,
        &none_j,
        &none_d,
        &none_t,
        &some, // 15..21
        &arr,
        &arr_null,
        &arr_text,
        &arr_slice,
        &empty,         // 22..26
        &Mood("happy"), // 27, untyped: Postgres resolves it from the cast
    ];
    let sql = "SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, \
               $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27::text";
    let rows = db.query(sql, &params).await.unwrap();
    let r = &rows[0];
    let cols: Vec<Type> = r.columns().iter().map(|c| c.type_().clone()).collect();
    // Raw `String` and `&str` go as `UNKNOWN` (DF1), and Postgres resolves an
    // unknown in a select list to `text`; every other type is as declared.
    let declared: Vec<Type> = params.iter().map(|p| p.pg_type()).collect();
    assert_eq!(
        (&declared[6], &declared[7], &declared[15]),
        (&Type::UNKNOWN, &Type::UNKNOWN, &Type::UNKNOWN)
    );
    let want: Vec<Type> = params.iter().map(|p| p.value_type()).collect();
    assert_eq!(
        cols[..26],
        want[..26],
        "the declared types are what Postgres used"
    );

    assert_eq!(
        (
            r.get::<_, bool>(0),
            r.get::<_, i16>(1),
            r.get::<_, i32>(2),
            r.get::<_, i64>(3)
        ),
        (b, i2, i4, i8_)
    );
    assert_eq!((r.get::<_, f32>(4), r.get::<_, f64>(5)), (f4, f8));
    assert_eq!(
        (r.get::<_, &str>(6), r.get::<_, &str>(7)),
        ("text", "borrowed")
    );
    assert_eq!(
        (r.get::<_, Vec<u8>>(8), r.get::<_, &[u8]>(9)),
        (bytes.clone(), &bytes[..])
    );
    assert_eq!(r.get::<_, Uuid>(10), id);
    assert_eq!(r.get::<_, serde_json::Value>(11), json);
    assert_eq!(r.get::<_, Decimal>(12), dec);
    assert_eq!(r.get::<_, SystemTime>(13), when);
    assert_eq!(r.get::<_, Option<i64>>(14), None);
    assert_eq!(r.get::<_, Option<String>>(15), None);
    assert_eq!(r.get::<_, Option<Uuid>>(16), None);
    assert_eq!(r.get::<_, Option<serde_json::Value>>(17), None);
    assert_eq!(r.get::<_, Option<Decimal>>(18), None);
    assert_eq!(r.get::<_, Option<SystemTime>>(19), None);
    assert_eq!(r.get::<_, Option<i32>>(20), Some(7));
    assert_eq!(r.get::<_, Vec<i64>>(21), arr);
    assert_eq!(r.get::<_, Vec<Option<i32>>>(22), arr_null);
    assert_eq!(r.get::<_, Vec<String>>(23), arr_text);
    assert_eq!(r.get::<_, Vec<i16>>(24), [5, 6]);
    assert_eq!(r.get::<_, Vec<Uuid>>(25), empty);
    assert_eq!(r.get::<_, &str>(26), "happy");
}

/// Typed parameters bind to real columns, `NULL` included, and an enum label
/// sent as `UNKNOWN` binds to an enum column and an enum comparison.
#[tokio::test]
async fn params_bind_to_columns_and_enum_labels_need_no_oid() {
    let admin = app::admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS spike_param; DROP TYPE IF EXISTS spike_mood;
             CREATE TYPE spike_mood AS ENUM ('happy', 'sad');
             CREATE TABLE spike_param (n int8, t text, j jsonb, d numeric, a int4[], m spike_mood)",
        )
        .await
        .unwrap();
    let db = Db::connect(&app::dsn(), 1).unwrap();
    let (n, t): (Option<i64>, Option<&str>) = (Some(1), None);
    let j = serde_json::json!([1]);
    let d: Option<Decimal> = None;
    let a = vec![Some(1i32), None];
    let p: [&dyn Param; 6] = [&n, &t, &j, &d, &a, &Mood("sad")];
    db.query(
        "INSERT INTO spike_param (n, t, j, d, a, m) VALUES ($1, $2, $3, $4, $5, $6)",
        &p,
    )
    .await
    .unwrap();
    let p: [&dyn Param; 1] = [&Mood("sad")];
    let rows = db
        .query("SELECT n, t, a, m::text FROM spike_param WHERE m = $1", &p)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, Option<i64>>(0), Some(1));
    assert_eq!(rows[0].get::<_, Option<&str>>(1), None);
    assert_eq!(rows[0].get::<_, Vec<Option<i32>>>(2), a);
    assert_eq!(rows[0].get::<_, &str>(3), "sad");
    // A label the type doesn't have is the database's error, as with any value.
    let p: [&dyn Param; 1] = [&Mood("angry")];
    let err = db
        .query("SELECT 1 FROM spike_param WHERE m = $1", &p)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Some("22P02"));
}
