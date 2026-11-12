//! Proof 9: `Param` is `ToSql` plus its Postgres type, for `query_typed`.
//!
//! DF1: where morm knows the column a parameter goes to, the declared type is
//! the column's; a raw `String` or `&str` goes as `Type::UNKNOWN`, so Postgres
//! infers the type from where it's used. An `UNKNOWN` text value is sent in
//! text format, so Postgres reads it the way it reads a quoted literal.

use bytes::BytesMut;
use std::error::Error as StdError;
use std::net::IpAddr;
use std::time::SystemTime;
use tokio_postgres::types::{Format, IsNull, ToSql, Type, to_sql_checked};

/// A parameter: `ToSql` plus its Postgres type, because `query_typed` needs it.
pub trait Param: ToSql + Sync {
    /// The type declared for a raw parameter, where no column is known.
    fn pg_type(&self) -> Type;
    /// The value's own type, which the spike first declared everywhere:
    /// `String` is `text`. Kept to compare against.
    fn value_type(&self) -> Type {
        self.pg_type()
    }
    /// Whether the value can be written in text format.
    fn has_text(&self) -> bool {
        false
    }
    /// Writes the value in text format. Only called when `has_text`.
    fn encode_text(&self, out: &mut BytesMut) {
        let _ = out;
    }
    fn is_null(&self) -> bool {
        false
    }
}

/// A type whose Postgres type is fixed: the scalar and its array.
pub trait PgType {
    const TYPE: Type;
    const ARRAY: Type;
    /// Sent as `UNKNOWN` when raw: `String` and `&str`.
    const RAW_UNKNOWN: bool = false;
    /// Has a text form the spike writes: text values and arrays of them.
    const HAS_TEXT: bool = false;
    fn text(&self, out: &mut BytesMut) {
        let _ = out;
    }
    fn null(&self) -> bool {
        false
    }
}

impl<T: PgType + ToSql + Sync> Param for T {
    #[inline]
    fn pg_type(&self) -> Type {
        if T::RAW_UNKNOWN {
            Type::UNKNOWN
        } else {
            T::TYPE
        }
    }
    #[inline]
    fn value_type(&self) -> Type {
        T::TYPE
    }
    #[inline]
    fn has_text(&self) -> bool {
        T::HAS_TEXT
    }
    #[inline]
    fn encode_text(&self, out: &mut BytesMut) {
        self.text(out)
    }
    #[inline]
    fn is_null(&self) -> bool {
        self.null()
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
    Vec<u8> => BYTEA, BYTEA_ARRAY;
    uuid::Uuid => UUID, UUID_ARRAY;
    serde_json::Value => JSONB, JSONB_ARRAY;
    rust_decimal::Decimal => NUMERIC, NUMERIC_ARRAY;
    SystemTime => TIMESTAMPTZ, TIMESTAMPTZ_ARRAY;
    chrono::DateTime<chrono::Utc> => TIMESTAMPTZ, TIMESTAMPTZ_ARRAY;
    chrono::NaiveDateTime => TIMESTAMP, TIMESTAMP_ARRAY;
    chrono::NaiveDate => DATE, DATE_ARRAY;
    chrono::NaiveTime => TIME, TIME_ARRAY;
    IpAddr => INET, INET_ARRAY;
}

impl PgType for String {
    const TYPE: Type = Type::TEXT;
    const ARRAY: Type = Type::TEXT_ARRAY;
    const RAW_UNKNOWN: bool = true;
    const HAS_TEXT: bool = true;
    fn text(&self, out: &mut BytesMut) {
        out.extend_from_slice(self.as_bytes());
    }
}
impl PgType for &str {
    const TYPE: Type = Type::TEXT;
    const ARRAY: Type = Type::TEXT_ARRAY;
    const RAW_UNKNOWN: bool = true;
    const HAS_TEXT: bool = true;
    fn text(&self, out: &mut BytesMut) {
        out.extend_from_slice(self.as_bytes());
    }
}
impl PgType for &[u8] {
    const TYPE: Type = Type::BYTEA;
    const ARRAY: Type = Type::BYTEA_ARRAY;
}

/// `None` is a `NULL` of `T`'s type.
impl<T: PgType> PgType for Option<T> {
    const TYPE: Type = T::TYPE;
    const ARRAY: Type = T::ARRAY;
    const RAW_UNKNOWN: bool = T::RAW_UNKNOWN;
    const HAS_TEXT: bool = T::HAS_TEXT;
    fn text(&self, out: &mut BytesMut) {
        if let Some(v) = self {
            v.text(out)
        }
    }
    fn null(&self) -> bool {
        self.is_none()
    }
}

/// An array literal, `{"a","b",NULL}`, from elements with a text form.
fn array_text<T: PgType>(items: &[T], out: &mut BytesMut) {
    out.extend_from_slice(b"{");
    let mut el = BytesMut::new();
    for (i, v) in items.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b",");
        }
        if v.null() {
            out.extend_from_slice(b"NULL");
            continue;
        }
        el.clear();
        v.text(&mut el);
        out.extend_from_slice(b"\"");
        for &b in el.iter() {
            if b == b'"' || b == b'\\' {
                out.extend_from_slice(b"\\");
            }
            out.extend_from_slice(&[b]);
        }
        out.extend_from_slice(b"\"");
    }
    out.extend_from_slice(b"}");
}

/// Arrays: `Vec<T>` and `&[T]`, including `Vec<Option<T>>`. An array keeps
/// its element type even when raw.
impl<T: PgType> PgType for Vec<T> {
    const TYPE: Type = T::ARRAY;
    const ARRAY: Type = Type::UNKNOWN;
    const HAS_TEXT: bool = T::HAS_TEXT;
    fn text(&self, out: &mut BytesMut) {
        array_text(self, out)
    }
}
impl<T: PgType> PgType for &[T] {
    const TYPE: Type = T::ARRAY;
    const ARRAY: Type = Type::UNKNOWN;
    const HAS_TEXT: bool = T::HAS_TEXT;
    fn text(&self, out: &mut BytesMut) {
        array_text(self, out)
    }
}

/// Where a parameter goes, as far as morm knows (DF1).
#[derive(Clone, Copy, Debug)]
pub enum Slot {
    /// Raw SQL: the parameter's own `pg_type`.
    Raw,
    /// A column whose type is built in, with a fixed OID.
    Builtin(&'static Type),
    /// A column whose type has no fixed OID (`citext`, an enum): `UNKNOWN`.
    NoOid,
    /// The value's own type, as the spike first did everywhere.
    Value,
}

/// One parameter as sent: the value, written in text format when it's sent
/// as `UNKNOWN` and has a text form. Implements `ToSql`, so a slice of
/// `&dyn Param` feeds `query_typed_raw` through an iterator, unboxed.
#[derive(Debug)]
pub struct Bound<'a> {
    p: &'a dyn Param,
    text: bool,
}

impl ToSql for Bound<'_> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        if self.p.is_null() {
            return Ok(IsNull::Yes);
        }
        if self.text {
            self.p.encode_text(out);
            return Ok(IsNull::No);
        }
        self.p.to_sql_checked(ty, out)
    }
    fn accepts(_: &Type) -> bool {
        true
    }
    fn encode_format(&self, _: &Type) -> Format {
        if self.text {
            Format::Text
        } else {
            Format::Binary
        }
    }
    to_sql_checked!();
}

/// The declared type and wire form for `p` going to `slot`.
pub fn bind(p: &dyn Param, slot: Slot) -> (Bound<'_>, Type) {
    let ty = match slot {
        Slot::Raw => p.pg_type(),
        Slot::Builtin(t) => t.clone(),
        Slot::NoOid => Type::UNKNOWN,
        Slot::Value => p.value_type(),
    };
    let text = ty == Type::UNKNOWN && p.has_text();
    (Bound { p, text }, ty)
}

/// `p` declared as `ty`, in text or binary format: for measuring the forms
/// `bind` doesn't choose, such as an `UNKNOWN` text value sent as binary.
#[doc(hidden)]
pub fn bind_with(p: &dyn Param, ty: Type, text: bool) -> (Bound<'_>, Type) {
    (Bound { p, text }, ty)
}

/// Runs `sql` with raw parameters straight from `&[&dyn Param]`: the
/// iterator feeds `query_typed_raw`, so nothing is boxed or collected.
pub(crate) fn typed<'a>(
    params: &'a [&'a dyn Param],
) -> impl ExactSizeIterator<Item = (Bound<'a>, Type)> + 'a {
    params.iter().map(|p| bind(*p, Slot::Raw))
}
