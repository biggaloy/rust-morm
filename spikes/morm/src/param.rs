//! Proof 9, rerun under DH2: every parameter has a built-in *carrier*, the
//! type it's declared as in Parse and encoded as, and a *cast*, written into
//! the SQL as `($n::cast)`, so Postgres never infers a parameter's type.
//!
//! - Bound through a field, the cast is the column's type with no length,
//!   precision or scale, and the carrier is that type's built-in carrier
//!   (`text` for `citext`, an enum or an extension type, a domain's base
//!   type's). The conversion layer encodes the value as the carrier.
//! - Free, the cast and carrier are the Rust type's own, from `SqlType`.
//!
//! Every carrier is built in, so every parameter goes in binary format, and
//! nothing is boxed: `&[&dyn Param]` feeds `query_typed_raw` through an
//! iterator of `Bound`.

use bytes::{BufMut, BytesMut};
use std::error::Error as StdError;
use std::net::IpAddr;
use std::time::SystemTime;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};

type BoxErr = Box<dyn StdError + Sync + Send>;

/// A Rust type's own SQL type: the cast written for a free parameter and the
/// built-in carrier it's declared and encoded as. The spike adds the
/// conversion layer as `encode_as`.
pub trait SqlType {
    const CAST: &'static str;
    const CAST_LIST: &'static str;
    const CARRIER: Type;
    const LIST_CARRIER: Type;
    fn null(&self) -> bool {
        false
    }
    /// The conversion layer: this value encoded as a field's carrier `ty`,
    /// where its own encoder doesn't take `ty`.
    fn encode_as(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr>
    where
        Self: ToSql,
    {
        self.to_sql_checked(ty, out)
    }
}

/// A parameter, object safe for `&dyn Param`, with a blanket impl from
/// `SqlType`.
pub trait Param: ToSql + Sync {
    fn cast(&self) -> &'static str;
    fn carrier(&self) -> Type;
    fn is_null(&self) -> bool;
    fn encode_field(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr>;
}

impl<T: SqlType + ToSql + Sync> Param for T {
    #[inline]
    fn cast(&self) -> &'static str {
        T::CAST
    }
    #[inline]
    fn carrier(&self) -> Type {
        T::CARRIER
    }
    #[inline]
    fn is_null(&self) -> bool {
        self.null()
    }
    #[inline]
    fn encode_field(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr> {
        self.encode_as(ty, out)
    }
}

macro_rules! sql_type {
    ($($t:ty => $c:literal, $s:ident, $cl:literal, $a:ident;)*) => {
        $(impl SqlType for $t {
            const CAST: &'static str = $c;
            const CAST_LIST: &'static str = $cl;
            const CARRIER: Type = Type::$s;
            const LIST_CARRIER: Type = Type::$a;
        })*
    };
}

sql_type! {
    bool => "bool", BOOL, "bool[]", BOOL_ARRAY;
    i64 => "int8", INT8, "int8[]", INT8_ARRAY;
    f64 => "float8", FLOAT8, "float8[]", FLOAT8_ARRAY;
    String => "text", TEXT, "text[]", TEXT_ARRAY;
    &str => "text", TEXT, "text[]", TEXT_ARRAY;
    Vec<u8> => "bytea", BYTEA, "bytea[]", BYTEA_ARRAY;
    &[u8] => "bytea", BYTEA, "bytea[]", BYTEA_ARRAY;
    uuid::Uuid => "uuid", UUID, "uuid[]", UUID_ARRAY;
    rust_decimal::Decimal => "numeric", NUMERIC, "numeric[]", NUMERIC_ARRAY;
    chrono::NaiveDate => "date", DATE, "date[]", DATE_ARRAY;
    chrono::NaiveTime => "time", TIME, "time[]", TIME_ARRAY;
}

/// Types the conversion layer converts: each gives its own `encode_as`.
macro_rules! sql_type_conv {
    ($($t:ty => $c:literal, $s:ident, $cl:literal, $a:ident, |$v:ident, $ty:ident, $out:ident| $body:expr;)*) => {
        $(impl SqlType for $t {
            const CAST: &'static str = $c;
            const CAST_LIST: &'static str = $cl;
            const CARRIER: Type = Type::$s;
            const LIST_CARRIER: Type = Type::$a;
            fn encode_as(&self, $ty: &Type, $out: &mut BytesMut) -> Result<IsNull, BoxErr> {
                let $v = self;
                $body
            }
        })*
    };
}

sql_type_conv! {
    i16 => "int2", INT2, "int2[]", INT2_ARRAY, |v, ty, out| match *ty {
        Type::INT4 => i32::from(*v).to_sql(ty, out),
        Type::INT8 => i64::from(*v).to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    i32 => "int4", INT4, "int4[]", INT4_ARRAY, |v, ty, out| match *ty {
        Type::INT8 => i64::from(*v).to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    f32 => "float4", FLOAT4, "float4[]", FLOAT4_ARRAY, |v, ty, out| match *ty {
        Type::FLOAT8 => f64::from(*v).to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    // An instant on `timestamp` is its UTC wall time (DG1).
    chrono::DateTime<chrono::Utc> => "timestamptz", TIMESTAMPTZ, "timestamptz[]", TIMESTAMPTZ_ARRAY, |v, ty, out| match *ty {
        Type::TIMESTAMP => v.naive_utc().to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    SystemTime => "timestamptz", TIMESTAMPTZ, "timestamptz[]", TIMESTAMPTZ_ARRAY, |v, ty, out| match *ty {
        Type::TIMESTAMP => chrono::DateTime::<chrono::Utc>::from(*v).naive_utc().to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    // A naive time on `timestamptz` is that wall time in UTC (DG1).
    chrono::NaiveDateTime => "timestamp", TIMESTAMP, "timestamp[]", TIMESTAMP_ARRAY, |v, ty, out| match *ty {
        Type::TIMESTAMPTZ => v.and_utc().to_sql(ty, out),
        _ => v.to_sql_checked(ty, out),
    };
    // `jsonb` stays `jsonb`; on a `json` column it's sent as `json`, which
    // its encoder takes, so the text is kept as serialized.
    serde_json::Value => "jsonb", JSONB, "jsonb[]", JSONB_ARRAY, |v, ty, out| v.to_sql_checked(ty, out);
    // `IpAddr` on `cidr`: the same wire form as `inet`, with the cidr flag.
    IpAddr => "inet", INET, "inet[]", INET_ARRAY, |v, ty, out| match *ty {
        Type::CIDR => {
            match v {
                IpAddr::V4(a) => {
                    out.put_slice(&[2, 32, 1, 4]);
                    out.put_slice(&a.octets());
                }
                IpAddr::V6(a) => {
                    out.put_slice(&[3, 128, 1, 16]);
                    out.put_slice(&a.octets());
                }
            }
            Ok(IsNull::No)
        }
        _ => v.to_sql_checked(ty, out),
    };
}

/// `None` is a `NULL` of `T`'s type, cast as `T`.
impl<T: SqlType + ToSql> SqlType for Option<T> {
    const CAST: &'static str = T::CAST;
    const CAST_LIST: &'static str = T::CAST_LIST;
    const CARRIER: Type = T::CARRIER;
    const LIST_CARRIER: Type = T::LIST_CARRIER;
    fn null(&self) -> bool {
        self.is_none()
    }
    fn encode_as(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr> {
        match self {
            Some(v) => v.encode_as(ty, out),
            None => Ok(IsNull::Yes),
        }
    }
}

/// A one-dimensional binary array of `items`, each encoded as `member`
/// through the conversion layer.
fn array_as<T: SqlType + ToSql>(
    items: &[T],
    ty: &Type,
    out: &mut BytesMut,
) -> Result<IsNull, BoxErr> {
    let member = match ty.kind() {
        tokio_postgres::types::Kind::Array(m) => m.clone(),
        _ => return Err(format!("{ty} isn't an array type").into()),
    };
    out.put_i32(1);
    let nulls = items.iter().any(|v| v.null());
    out.put_i32(i32::from(nulls));
    out.put_u32(member.oid());
    out.put_i32(i32::try_from(items.len())?);
    out.put_i32(1);
    for v in items {
        let at = out.len();
        out.put_i32(0);
        match v.encode_as(&member, out)? {
            IsNull::Yes => {
                out.truncate(at);
                out.put_i32(-1);
            }
            IsNull::No => {
                let n = i32::try_from(out.len() - at - 4)?;
                out[at..at + 4].copy_from_slice(&n.to_be_bytes());
            }
        }
    }
    Ok(IsNull::No)
}

/// A list: `Vec<T>` and `&[T]`, cast to `T`'s list cast, with `T`'s
/// carrier's array.
impl<T: SqlType + ToSql> SqlType for Vec<T> {
    const CAST: &'static str = T::CAST_LIST;
    const CAST_LIST: &'static str = "";
    const CARRIER: Type = T::LIST_CARRIER;
    const LIST_CARRIER: Type = Type::UNKNOWN;
    fn encode_as(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr> {
        array_as(self, ty, out)
    }
}
impl<T: SqlType + ToSql> SqlType for &[T] {
    const CAST: &'static str = T::CAST_LIST;
    const CAST_LIST: &'static str = "";
    const CARRIER: Type = T::LIST_CARRIER;
    const LIST_CARRIER: Type = Type::UNKNOWN;
    fn encode_as(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr> {
        array_as(self, ty, out)
    }
}

/// A field's typing, as `#[model]` writes it into `Column`: the cast with no
/// modifier, and the built-in carrier.
#[derive(Clone, Debug)]
pub struct Field {
    pub cast: &'static str,
    pub carrier: Type,
}

/// One parameter as sent, always in binary format. Bound through a field it
/// goes through the conversion layer; free, through its own encoder.
#[derive(Debug)]
pub struct Bound<'a> {
    p: &'a dyn Param,
    field: bool,
}

impl ToSql for Bound<'_> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxErr> {
        if self.p.is_null() {
            return Ok(IsNull::Yes);
        }
        if self.field {
            self.p.encode_field(ty, out)
        } else {
            self.p.to_sql_checked(ty, out)
        }
    }
    fn accepts(_: &Type) -> bool {
        true
    }
    to_sql_checked!();
}

/// `p` bound through a field: its cast, and the declared type and wire form.
pub fn field<'a>(p: &'a dyn Param, f: &Field) -> (&'static str, Bound<'a>, Type) {
    (f.cast, Bound { p, field: true }, f.carrier.clone())
}

/// `p` free: its own cast and carrier.
pub fn free(p: &dyn Param) -> (&'static str, Bound<'_>, Type) {
    (p.cast(), Bound { p, field: false }, p.carrier())
}

/// Parameters straight from `&[&dyn Param]`, each declared as its carrier:
/// the iterator feeds `query_typed_raw`, so nothing is boxed or collected.
/// The spike's other proofs write their own SQL, so no cast is added here.
pub(crate) fn typed<'a>(
    params: &'a [&'a dyn Param],
) -> impl ExactSizeIterator<Item = (Bound<'a>, Type)> + 'a {
    params.iter().map(|p| {
        let (_, b, t) = free(*p);
        (b, t)
    })
}
