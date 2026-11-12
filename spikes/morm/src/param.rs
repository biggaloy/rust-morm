//! Proof 9: `Param` is `ToSql` plus its Postgres type, for `query_typed`.

use std::time::SystemTime;
use tokio_postgres::types::{ToSql, Type};

/// A parameter: `ToSql` plus its Postgres type, because `query_typed` needs it.
pub trait Param: ToSql + Sync {
    fn pg_type(&self) -> Type;
}

/// A type whose Postgres type is fixed: the scalar and its array.
pub trait PgType {
    const TYPE: Type;
    const ARRAY: Type;
}

impl<T: PgType + ToSql + Sync> Param for T {
    #[inline]
    fn pg_type(&self) -> Type {
        T::TYPE
    }
}

macro_rules! pg_type {
    ($($t:ty => $s:ident, $a:ident;)*) => {
        $(impl PgType for $t { const TYPE: Type = Type::$s; const ARRAY: Type = Type::$a; })*
    };
}

pg_type! {
    bool => BOOL, BOOL_ARRAY;
    i16 => INT2, INT2_ARRAY;
    i32 => INT4, INT4_ARRAY;
    i64 => INT8, INT8_ARRAY;
    f32 => FLOAT4, FLOAT4_ARRAY;
    f64 => FLOAT8, FLOAT8_ARRAY;
    String => TEXT, TEXT_ARRAY;
    Vec<u8> => BYTEA, BYTEA_ARRAY;
    uuid::Uuid => UUID, UUID_ARRAY;
    serde_json::Value => JSONB, JSONB_ARRAY;
    rust_decimal::Decimal => NUMERIC, NUMERIC_ARRAY;
    SystemTime => TIMESTAMPTZ, TIMESTAMPTZ_ARRAY;
}

impl PgType for &str {
    const TYPE: Type = Type::TEXT;
    const ARRAY: Type = Type::TEXT_ARRAY;
}
impl PgType for &[u8] {
    const TYPE: Type = Type::BYTEA;
    const ARRAY: Type = Type::BYTEA_ARRAY;
}

/// `None` is a `NULL` of `T`'s type.
impl<T: PgType> PgType for Option<T> {
    const TYPE: Type = T::TYPE;
    const ARRAY: Type = T::ARRAY;
}

/// Arrays: `Vec<T>` and `&[T]`, including `Vec<Option<T>>`.
impl<T: PgType> PgType for Vec<T> {
    const TYPE: Type = T::ARRAY;
    const ARRAY: Type = Type::UNKNOWN;
}
impl<T: PgType> PgType for &[T] {
    const TYPE: Type = T::ARRAY;
    const ARRAY: Type = Type::UNKNOWN;
}

/// Runs `sql` with typed parameters straight from `&[&dyn Param]`: the
/// iterator feeds `query_typed_raw`, so nothing is boxed or collected.
pub(crate) fn typed<'a>(
    params: &'a [&'a dyn Param],
) -> impl ExactSizeIterator<Item = (&'a (dyn ToSql + Sync), Type)> + 'a {
    params.iter().map(|p| (*p as &(dyn ToSql + Sync), p.pg_type()))
}
