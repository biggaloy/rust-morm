# Step 56: Enums, domains and composite types in Rust

- **Phase:** Runtime
- **Depends on:** 13, 28
- **Can run beside:** 10, 11, 12, 29
- **Crates:** `morm`, `morm-macros`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Map Rust enums, domains and composite types to their columns, with binding that a type swap can't break.

## Deliverables

- `TextEnum` with its CHECK in metadata and its `unknown` fallback, `PgEnum` with OID-free binding, `#[pg_type]`, variant labels, `was`, `PgDomain`, `PgComposite`

## End-to-end check

Each of these types round-trips through a real Postgres on tables and types created by hand, and `PgEnum` values still bind and decode after `ADD VALUE` and a hand-run type swap.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 13 gives exact encoding, `SqlType` and `morm::Param` for every other type, and the cast-name function. Step 05 emits each range CHECK in `ModelMeta`.
>
> Step 30 proves the `TextEnum` CHECK on a database. Step 35 adds the attributes that make morm manage a type, and step 41 adds `deprecated`.
>

## Out of scope

- Creating these types or the CHECK on a database (steps 30 and 57)
- `#[pg_enum(name = "...")]`, `#[pg_domain]` and `#[pg_composite]` (step 35)
- JSON formats (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Type inference

**R-0187** `#[derive(TextEnum)]` maps a Rust enum to `text` with a CHECK constraint, the recommended form, because adding or removing a value is an ordinary migration.

**R-0188** `#[derive(PgEnum)]` maps to a Postgres enum, and on its own it only binds to a type that already exists. `#[pg_enum(name = "...")]` turns on full management, in step 35: adding, renaming, removing and reordering values. `TextEnum` stays the preferred form, because removing or reordering a Postgres enum value means replacing the whole type.

**R-0189** Domains and composite types are also supported.

- **R-0961** **Domains and composite types in Rust.** `#[derive(PgDomain)]` goes on a one-field struct, such as `pub struct Email(pub String);`, and `#[derive(PgComposite)]` on a struct with named fields, one field per attribute, in order. Each gives encoding, decoding, `SqlType` and `morm::Param`, and a model column uses the Rust type directly, as it does a `PgEnum`. Each is declared as its carrier, `text` for a composite and the base type's for a domain, and cast to its name, so no OID is looked up or sent. On its own, `PgDomain`, `PgComposite` or `PgEnum` only binds to a type that already exists: morm doesn't create or change that type. Step 35 adds the attributes that make morm manage one.

### Managed enums

- **R-0976** **Type names.** A bare `#[derive(PgEnum)]`, `#[derive(PgDomain)]` or `#[derive(PgComposite)]` binds to the snake_case of the Rust name, so `OrderState` binds to `order_state`. `#[pg_type(name = "...")]` overrides that name, and it doesn't turn on management.
- **R-0977** **Labels.** A variant's Postgres label is the snake_case of its name, so `AwaitingPayment` is `awaiting_payment`. `#[pg_enum(value = "...")]` on the variant overrides it. Attributes on a variant never turn on management: only `#[pg_enum(name = "...")]` on the type does. A label starting `_morm_`, whether given by `value` or made from the variant's name, is a compile error naming the variant, because morm renames a retired enum type's labels to `_morm_retired_<label>`, and a label of yours with that prefix could collide with one. Owner decision SA.
- **R-1055** **`TextEnum` labels.** `#[derive(TextEnum)]` stores each variant's Rust spelling, so `Pending` is stored as `Pending`, and the CHECK constraint lists that same spelling. This differs from `PgEnum`, whose labels are snake_case. `#[model]` emits the CHECK in metadata like a range CHECK, named `__CHK_{table}_{column}_enum__`, reading labels through a trait constant on the type. `ModelMeta` and the manifest also carry these labels on the column, so step 32's with-data proof can read them.
- **R-0978** **Enum binding needs no OID.** morm writes `ToSql` and `FromSql` for `PgEnum` itself and doesn't use `postgres-derive`, whose exact variant check fails on every connection opened after a value is added. `accepts` compares no variants: `ToSql` accepts only the `text` carrier, and `FromSql` accepts only `text`, because R-1241 reads every enum column as text. A `PgEnum` parameter is declared as `text` and cast to the enum's name, so no OID is looked up or sent, and a swap can't leave one stale. A list goes as `text[]`, cast to the enum's array type. Labels are matched as bytes at compile time, with no heap use.
- **R-1241** **Enum and composite result columns are read as text.** `tokio-postgres` looks up any result column type that isn't built in, with named statements of its own, once per connection per type. Behind PgBouncer with no prepared statements, 1.20.1, or a later version with `max_prepared_statements = 0`, those statements reach other server connections and fail with `26000` `prepared statement "sN" does not exist`: 83 of 120 enum decodes failed on 1.20.1 with a pool of 4, and 86 of 120 on 1.22.0 with `max_prepared_statements = 0`. So morm never lets the driver see such a column. Every result column morm writes, in a model's or a view's column list, `filter_cols`, `RETURNING`, related rows and keyset pages, whose type is an enum, a composite, a domain over either, or a type morm doesn't know that a field with a text form sits on, such as `citext` or `ltree`, is read as `"col"::text`, or `"col"::text[]` for an array, which keeps the column's name. `SqlType` says how through `READ_AS`, the cast written after a result column, or nothing: `text` for `PgEnum` and `PgComposite` and for a `PgDomain` over one of them, `text[]` for a list of any of these, and for a list of a `PgDomain` over a built-in type its base type's array, such as `int4[]` for a list of a domain over `integer`, because Postgres sends an array of a domain as the domain's own array type, with an OID of 16384 or more, where it sends a scalar domain as its base type; every other type writes nothing. `#[model]` and `#[view]` also read as text a field with a text form whose `sql` names a type morm doesn't know. The cast is written into the compile-time column list, so it costs nothing at run time. `PgEnum`'s and `PgComposite`'s `FromSql` accept only `text`, so an uncast column can't decode by luck on one connection and fail on another. `PgComposite` decodes Postgres's text form, `(a,b)`, with no lookup: a field with nothing between its delimiters is NULL, `""` is an empty string, a backslash escapes the next character, inside quotes `""` is one `"`, and a nested composite or array field is passed to its own type's decoder. It checks every position and length against the input, and returns an error, never a panic, on a field count that doesn't match, a NULL in a field that isn't an `Option`, a missing delimiter, an unclosed quote or anything after the closing parenthesis. A scalar domain over a built-in type needs no cast, because Postgres sends a domain's base type in the row description: on 18.6 a column of a domain over `text` came back as `text`, and one of a domain over an enum as that enum, while on 18.6 and 17.7 `posint[]`, `dtext[]` and `denum[]` came back as their own array types, and `SELECT d::int4[]` read `{1,2}`. Owner decisions EA and EF2.
- **R-1233** **`SqlType` for these types.** `TextEnum` is cast as `text`, with a `text` carrier, as a `String` is, and carries `text` and `varchar`. `PgEnum` and `PgComposite` are cast to their type's name, as R-0976 or `#[pg_type]` gives it, quoted as morm's DDL quotes it, such as `"billing"."order_state"`, with a `text` carrier, and carry only that type. `PgDomain` over `T` is cast to the domain's name, with `T`'s carrier, and carries only the domain. None of the four has a text form, so none sits on a type morm doesn't know: a `PgEnum` field on another enum or on `ltree` is a compile error naming the field, and a `PgDomain` field sits only on its own domain. Each list is cast to the name with `[]`, with the carrier's array, and carries only that array type. Owner decisions DH2 and DH3.
- **R-0979** **A label the binary doesn't know** returns `Error::UnknownEnumValue`, with the type and the label, for rows holding it and no others. `#[pg_enum(unknown)]` on one unit variant decodes any such label to that variant instead, and writing that variant returns `Error::Validation`. This variant has no label: it isn't in the CHECK list, the managed type or `check_schema`, and it can't take `value`, `was` or `deprecated`.
- **R-1056** **`TextEnum` fallback.** `#[text_enum(unknown)]` on one unit variant works like `#[pg_enum(unknown)]`: any stored value the binary doesn't know decodes to that variant, as one branch on read, instead of returning `Error::UnknownEnumValue`, and writing that variant returns `Error::Validation`. It's opt-in. Without it, an unknown stored value returns `Error::UnknownEnumValue`, with the Rust type's name and the value. This variant has no label: it isn't in the CHECK list, the managed type or `check_schema`, and it can't take `value`, `was` or `deprecated`.
- **R-0980** **Renamed labels.** `#[pg_enum(was = "old")]` on a variant decodes both labels to it and writes the current one, so a new binary reads rows from either side of a rename. A current label beats any `was`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0141 | integration | `#[derive(PgDomain)]` and `#[derive(PgComposite)]` round-tripping through columns and parameters on types created by hand |
| T-0142 | integration | **Plain `String` on a domain:** test whether a `String` field or parameter binds to a column of a text domain, and don't assume it. State the result in the PR. If it doesn't bind, `PgDomain` is the only way to use such a column, and its docs say so. |
| T-0148 | integration | **Enum binding survives type changes:** on an open connection and on one opened afterwards, `PgEnum` values and arrays, declared as `text` and `text[]` and cast to the enum's name, bind, and decode from columns read as `::text` and `::text[]`, through `query_typed` and prepared statements after `ADD VALUE`, and through `query_typed` and a prepared statement on the swapped table after a hand-run type swap; statements prepared before `ADD VALUE`, a free `(($1::st))::text`, an insert binding the new label and a select returning the column, run after it with the new label and give it back; `PgDomain` and `PgComposite` values, cast to their names, do the same after their type is dropped and made again by hand |
| T-0149 | integration | **Names and labels:** a bare derive binds to the snake_case name, `#[pg_type(name = "...")]` and `#[pg_enum(value = "...")]` override it, an unknown label returns `Error::UnknownEnumValue`, `#[pg_enum(unknown)]` decodes it and refuses to write, and `was` decodes both labels. |
| T-0150 | trybuild | a `PgEnum` with duplicate labels, an empty label, a label over 63 bytes, a label starting `_morm_`, whether given by `value` or made from the variant's name, a variant that isn't a unit variant, two `#[pg_enum(unknown)]` variants, or an `unknown` variant that also has `value`, `was` or `deprecated` |
| T-0180 | integration | **`TextEnum` labels:** variants such as `Pending` and `AwaitingPayment` are stored as those exact spellings, not snake_case, and the CHECK in `ModelMeta` lists the same spellings. |
| T-0181 | integration | **`TextEnum` fallback:** `#[text_enum(unknown)]` decodes a stored value the binary doesn't know to that variant, known values still decode to their own variants, writing that variant returns `Error::Validation`, and without the attribute the same value returns `Error::UnknownEnumValue`. |
| T-0182 | trybuild | a `TextEnum` with two `#[text_enum(unknown)]` variants, or a variant that isn't a unit variant |
| T-0301 | suite | **Unit: reading composites and enums as text:** a model's and a view's column list read a `PgEnum`, a `PgComposite`, a `PgDomain` over an enum, a list of each, and a `String` field on `citext` and on `ltree` as `"col"::text` or `"col"::text[]`, a list of a `PgDomain` over `integer` as `"col"::int4[]`, and an `i32`, a `String` on `text` and a `PgDomain` over `integer` with no cast, all in the compile-time constant; `PgComposite`'s decoder reads a NULL field, an empty string, a field with a quote, a comma, a parenthesis, a backslash and a leading space, a nested composite and an array field, as Postgres 15 to 18 print them, and returns an error, never a panic, on a short or long field count, a NULL in a non-`Option` field, a missing delimiter, an unclosed quote, trailing text and every truncation of a valid input; `PgEnum` and `PgComposite` refuse any type but `text` |
