//! Spike macros for step 02: `#[model]` with views, bases, groups, unique keys,
//! borrowed rows and registration; `#[base]`; `#[hooks]`; `groups!`.
//! Throwaway code: it proves the designs compile and work, nothing more.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Ts};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, Field, Fields, Ident, ImplItem, ItemImpl, ItemStruct, Lit, Meta, Path, Token,
    Type, Visibility, braced,
};

fn err(span: Span, msg: impl std::fmt::Display) -> syn::Error {
    syn::Error::new(span, msg)
}

/// `__base_fields { .. }`, which a base's `macro_rules!` passes back, then the
/// ordinary comma-separated arguments.
struct Args {
    base_fields: Vec<Field>,
    metas: Vec<Meta>,
}

fn parse_args(tokens: TokenStream) -> syn::Result<Args> {
    let parser = |input: ParseStream| {
        let mut base_fields = Vec::new();
        if input.peek(Ident) && input.fork().parse::<Ident>()? == "__base_fields" {
            input.parse::<Ident>()?;
            let content;
            braced!(content in input);
            let fields: Punctuated<Field, Token![,]> =
                content.parse_terminated(Field::parse_named, Token![,])?;
            base_fields = fields.into_iter().collect();
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        let metas: Punctuated<Meta, Token![,]> = input.parse_terminated(Meta::parse, Token![,])?;
        Ok(Args {
            base_fields,
            metas: metas.into_iter().collect(),
        })
    };
    parser.parse(tokens)
}

fn path_value(meta: &Meta) -> syn::Result<Path> {
    match &meta.require_name_value()?.value {
        Expr::Path(p) => Ok(p.path.clone()),
        other => Err(err(Span::call_site(), format!("expected a path, found `{}`", quote!(#other)))),
    }
}

fn str_value(meta: &Meta) -> syn::Result<String> {
    match &meta.require_name_value()?.value {
        Expr::Lit(l) => match &l.lit {
            Lit::Str(s) => Ok(s.value()),
            _ => Err(err(Span::call_site(), "expected a string")),
        },
        _ => Err(err(Span::call_site(), "expected a string")),
    }
}

fn ident_list(meta: &Meta) -> syn::Result<Vec<Ident>> {
    let list = meta.require_list()?;
    let ids: Punctuated<Ident, Token![,]> = list.parse_args_with(Punctuated::parse_terminated)?;
    Ok(ids.into_iter().collect())
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn q(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn named_fields(item: &ItemStruct) -> syn::Result<Vec<Field>> {
    match &item.fields {
        Fields::Named(f) => Ok(f.named.iter().cloned().collect()),
        _ => Err(err(Span::call_site(), "morm models need named fields")),
    }
}

fn take_attrs(attrs: &mut Vec<Attribute>, name: &str) -> Vec<Attribute> {
    let (taken, kept) = attrs.drain(..).partition(|a| a.path().is_ident(name));
    *attrs = kept;
    taken
}

// ---------------------------------------------------------------- #[base]

/// `#[base]` / `#[base(extends = path)]`: a set of fields other models copy in.
///
/// It emits a `#[macro_export]` `macro_rules!` that hands the fields back to
/// whichever attribute asked (`model` or `base`), so the fields cross crates as
/// tokens and land in the model as plain fields.
#[proc_macro_attribute]
pub fn base(args: TokenStream, item: TokenStream) -> TokenStream {
    match base_impl(args, item) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn base_impl(args: TokenStream, item: TokenStream) -> syn::Result<Ts> {
    let args = parse_args(args)?;
    let item_ts: Ts = item.clone().into();
    let mut st: ItemStruct = syn::parse(item)?;
    let mut extends = None;
    for m in &args.metas {
        if m.path().is_ident("extends") {
            extends = Some(path_value(m)?);
        } else {
            return Err(err(Span::call_site(), "unknown `base` argument"));
        }
    }
    if let Some(parent) = extends {
        // Ask the parent for its fields; it calls `#[::morm::base(__base_fields ..)]` back.
        return Ok(quote! { #parent! { @base [] #item_ts } });
    }
    let mut fields = args.base_fields;
    fields.extend(named_fields(&st)?);
    let name = &st.ident;
    let mac = format_ident!("__morm_base_{}", name);
    // The plain struct, for docs: field attributes stripped.
    let mut plain = fields.clone();
    for f in &mut plain {
        take_attrs(&mut f.attrs, "col");
    }
    st.fields = Fields::Named(syn::parse_quote!({ #(#plain),* }));
    Ok(quote! {
        #[allow(dead_code)]
        #st
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #mac {
            (@$kind:ident [$($args:tt)*] $($item:tt)*) => {
                #[::morm::$kind(__base_fields { #(#fields),* } , $($args)*)]
                $($item)*
            };
        }
        #[doc(hidden)]
        pub use #mac as #name;
    })
}

// ---------------------------------------------------------------- #[model]

struct Col {
    name: Ident,
    ty: Type,
    vis: Visibility,
    attrs: Vec<Attribute>,
    unique: bool,
    group: Option<Ident>,
}

struct View {
    name: Ident,
    cols: Vec<usize>,
}

struct UniqueDecl {
    name: Ident,
    conflict: String,
}

/// `#[model(table = "..", base = path, global = path, groups(a, b), hooks, borrowed)]`
#[proc_macro_attribute]
pub fn model(args: TokenStream, item: TokenStream) -> TokenStream {
    match model_impl(args, item) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn model_impl(args: TokenStream, item: TokenStream) -> syn::Result<Ts> {
    let args = parse_args(args)?;
    let item_ts: Ts = item.clone().into();
    let mut st: ItemStruct = syn::parse(item)?;

    let (mut table, mut global, mut scoped, mut hooks, mut borrowed) =
        (None, None, Vec::new(), false, false);
    let mut rest = Vec::new();
    for m in &args.metas {
        let p = m.path();
        if p.is_ident("base") {
            let base = path_value(m)?;
            let others: Vec<&Meta> = args.metas.iter().filter(|x| !x.path().is_ident("base")).collect();
            // Ask the base for its fields; it calls `#[::morm::model(__base_fields ..)]` back.
            return Ok(quote! { #base! { @model [#(#others),*] #item_ts } });
        } else if p.is_ident("table") {
            table = Some(str_value(m)?);
        } else if p.is_ident("global") {
            global = Some(path_value(m)?);
        } else if p.is_ident("groups") {
            scoped = ident_list(m)?;
        } else if p.is_ident("hooks") {
            hooks = true;
        } else if p.is_ident("borrowed") {
            borrowed = true;
        } else {
            rest.push(m);
        }
    }
    if let Some(m) = rest.first() {
        return Err(err(Span::call_site(), format!("unknown `model` argument `{}`", quote!(#m))));
    }

    let name = st.ident.clone();
    let vis = st.vis.clone();
    let table = table.unwrap_or_else(|| snake(&name.to_string()));

    // Fields: the bases' first, then the model's own, all plain.
    let mut raw = args.base_fields;
    raw.extend(named_fields(&st)?);
    let mut cols = Vec::new();
    for mut f in raw {
        let mut unique = false;
        let mut group = None;
        for a in take_attrs(&mut f.attrs, "col") {
            a.parse_nested_meta(|m| {
                if m.path.is_ident("unique") {
                    unique = true;
                } else if m.path.is_ident("group") {
                    group = Some(m.value()?.parse::<Ident>()?);
                } else {
                    return Err(m.error("unknown `col` argument"));
                }
                Ok(())
            })?;
        }
        cols.push(Col {
            name: f.ident.clone().unwrap(),
            ty: f.ty.clone(),
            vis: f.vis.clone(),
            attrs: f.attrs.clone(),
            unique,
            group,
        });
    }
    let find = |id: &Ident| -> syn::Result<usize> {
        cols.iter()
            .position(|c| c.name == *id)
            .ok_or_else(|| err(id.span(), format!("no field `{id}` on model `{name}`")))
    };

    let mut views = Vec::new();
    for a in take_attrs(&mut st.attrs, "view") {
        let metas: Punctuated<Meta, Token![,]> = a.parse_args_with(Punctuated::parse_terminated)?;
        let mut it = metas.iter();
        let vname = match it.next() {
            Some(Meta::Path(p)) if p.get_ident().is_some() => p.get_ident().unwrap().clone(),
            _ => return Err(err(Span::call_site(), "a view starts with its struct name")),
        };
        let mut chosen: Vec<usize> = (0..cols.len()).collect();
        for m in it {
            let ids = ident_list(m)?;
            let idx = ids.iter().map(find).collect::<syn::Result<Vec<_>>>()?;
            if m.path().is_ident("hide") {
                chosen.retain(|i| !idx.contains(i));
            } else if m.path().is_ident("only") {
                chosen = idx;
                chosen.sort();
            } else {
                return Err(err(Span::call_site(), "a view takes `hide(..)` or `only(..)`"));
            }
        }
        views.push(View { name: vname, cols: chosen });
    }

    let mut uniques = Vec::new();
    for a in take_attrs(&mut st.attrs, "unique") {
        let (mut uname, mut ucols, mut expr, mut filter) = (None, Vec::new(), None, None);
        a.parse_nested_meta(|m| {
            if m.path.is_ident("name") {
                uname = Some(m.value()?.parse::<Ident>()?);
            } else if m.path.is_ident("cols") {
                let content;
                syn::parenthesized!(content in m.input);
                let ids: Punctuated<Ident, Token![,]> = content.parse_terminated(Ident::parse, Token![,])?;
                ucols = ids.into_iter().collect();
            } else if m.path.is_ident("expr") {
                expr = Some(m.value()?.parse::<syn::LitStr>()?.value());
            } else if m.path.is_ident("filter") {
                filter = Some(m.value()?.parse::<syn::LitStr>()?.value());
            } else {
                return Err(m.error("unknown `unique` argument"));
            }
            Ok(())
        })?;
        let uname = uname.ok_or_else(|| err(Span::call_site(), "a unique key needs `name`"))?;
        let mut target: Vec<String> = Vec::new();
        for c in &ucols {
            target.push(q(&cols[find(c)?].name.to_string()));
        }
        if let Some(e) = expr {
            target.push(e);
        }
        let mut conflict = format!("({})", target.join(", "));
        if let Some(f) = filter {
            conflict = format!("{conflict} WHERE {f}");
        }
        uniques.push(UniqueDecl { name: uname, conflict });
    }

    // Groups: scoped names are this model's, anything else is a global group.
    let scoped_ty = if scoped.is_empty() {
        quote!(())
    } else {
        let n = scoped.len().div_ceil(64);
        quote!(::morm::Bits<#n>)
    };
    let global_ty = match &global {
        Some(g) => quote!(#g::Mask),
        None => quote!(()),
    };
    let mut reqs = Vec::new();
    for c in &cols {
        reqs.push(match &c.group {
            None => quote!(::morm::Req::Any),
            Some(g) => match scoped.iter().position(|s| s == g) {
                Some(i) => {
                    let i = i as u16;
                    quote!(::morm::Req::Scoped(#i))
                }
                None => match &global {
                    Some(gp) => quote!(::morm::Req::Global(#gp::GROUPS.#g.bit())),
                    None => {
                        return Err(err(g.span(), format!("`{g}` isn't a group of `{name}`, and the model names no `global` groups")));
                    }
                },
            },
        });
    }

    let cols_mod = format_ident!("__morm_cols_{}", name);
    let all_idx: Vec<usize> = (0..cols.len()).collect();
    let mut out = Ts::new();

    // The model struct itself, every field plain.
    let fdefs = cols.iter().map(|c| {
        let (a, v, n, t) = (&c.attrs, &c.vis, &c.name, &c.ty);
        quote!(#(#a)* #v #n: #t)
    });
    let sattrs = &st.attrs;
    let generics = &st.generics;
    out.extend(quote! {
        #(#sattrs)*
        #vis struct #name #generics { #(#fdefs),* }
    });

    // One marker type per column, and per declared unique key.
    let markers = cols.iter().map(|c| &c.name);
    let ukm: Vec<Ident> = uniques.iter().map(|u| format_ident!("__uk_{}", u.name)).collect();
    out.extend(quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types, non_snake_case, dead_code)]
        #vis mod #cols_mod {
            #(pub struct #markers;)*
            #(pub struct #ukm;)*
        }
    });

    let model_items = |target: &Ident, idx: &[usize]| -> Ts {
        let names: Vec<&Ident> = idx.iter().map(|&i| &cols[i].name).collect();
        let tys: Vec<&Type> = idx.iter().map(|&i| &cols[i].ty).collect();
        let col_names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let select = format!(
            "SELECT {} FROM {}",
            col_names.iter().map(|c| q(c)).collect::<Vec<_>>().join(", "),
            q(&table)
        );
        let insert = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            q(&table),
            col_names.iter().map(|c| q(c)).collect::<Vec<_>>().join(", "),
            (1..=col_names.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ")
        );
        let reqs_v: Vec<&Ts> = idx.iter().map(|&i| &reqs[i]).collect();
        let pos: Vec<usize> = (0..idx.len()).collect();
        let fstruct = format_ident!("{}Columns", target);
        quote! {
            #[allow(dead_code)]
            #vis struct #fstruct { #(pub #names: ::morm::Column<#target, #tys, #cols_mod::#names>),* }
            impl #target {
                /// This model's columns: `F.<field>`.
                pub const F: #fstruct = #fstruct { #(#names: ::morm::Column::new(#col_names)),* };
            }
            impl ::morm::Model for #target {
                type Global = #global_ty;
                type Scoped = #scoped_ty;
                const TABLE: &'static str = #table;
                const COLUMNS: &'static [&'static str] = &[#(#col_names),*];
                const SELECT: &'static str = #select;
                const GROUPS: &'static [::morm::Req] = &[#(#reqs_v),*];
                const INSERT: &'static str = #insert;
                fn from_row(row: &::morm::Row) -> ::morm::Result<Self> {
                    Ok(Self { #(#names: row.try_get(#pos)?),* })
                }
                fn params(&self) -> ::std::vec::Vec<&dyn ::morm::Param> {
                    ::std::vec![#(&self.#names as &dyn ::morm::Param),*]
                }
            }
        }
    };
    out.extend(model_items(&name, &all_idx));

    if !hooks {
        out.extend(quote! {
            impl ::morm::Hooks for #name { const OVERRIDES: ::morm::Overrides = ::morm::Overrides::NONE; }
        });
    }

    // Views: sibling structs from the same field list.
    for v in &views {
        let vname = &v.name;
        let fd = v.cols.iter().map(|&i| {
            let c = &cols[i];
            let (a, fv, n, t) = (&c.attrs, &c.vis, &c.name, &c.ty);
            quote!(#(#a)* #fv #n: #t)
        });
        out.extend(quote! {
            #(#sattrs)*
            #vis struct #vname { #(#fd),* }
            impl ::morm::Hooks for #vname { const OVERRIDES: ::morm::Overrides = ::morm::Overrides::NONE; }
        });
        out.extend(model_items(vname, &v.cols));
    }

    // Unique keys: a single unique column is its own key, other keys live in `UK`.
    for c in cols.iter().filter(|c| c.unique) {
        let (n, t) = (&c.name, &c.ty);
        let conflict = format!("({})", q(&n.to_string()));
        out.extend(quote! {
            impl ::morm::UniqueKey<#name> for ::morm::Column<#name, #t, #cols_mod::#n> {
                const CONFLICT: &'static str = #conflict;
            }
        });
    }
    let ukstruct = format_ident!("{}Keys", name);
    let uknames: Vec<&Ident> = uniques.iter().map(|u| &u.name).collect();
    let ukconf: Vec<&String> = uniques.iter().map(|u| &u.conflict).collect();
    out.extend(quote! {
        #[allow(dead_code)]
        #vis struct #ukstruct { #(pub #uknames: ::morm::Key<#name, #cols_mod::#ukm>),* }
        impl #name {
            /// Unique keys that aren't a single column: expression, partial and composite.
            pub const UK: #ukstruct = #ukstruct { #(#uknames: ::morm::Key::new()),* };
        }
        #(impl ::morm::UniqueKey<#name> for ::morm::Key<#name, #cols_mod::#ukm> {
            const CONFLICT: &'static str = #ukconf;
        })*
    });

    // Scoped group handles: `Model::G.<group>`.
    let gstruct = format_ident!("{}Groups", name);
    let gbits: Vec<u16> = (0..scoped.len() as u16).collect();
    out.extend(quote! {
        #[allow(dead_code)]
        #vis struct #gstruct { #(pub #scoped: ::morm::ScopedGroup<#name>),* }
        impl #name {
            /// This model's own groups.
            pub const G: #gstruct = #gstruct { #(#scoped: ::morm::ScopedGroup::new(#gbits)),* };
        }
    });

    // Borrowed rows: `<Model>Ref<'r>` reads text and bytes straight from the row buffer.
    if borrowed {
        let rname = format_ident!("{}Ref", name);
        let names: Vec<&Ident> = cols.iter().map(|c| &c.name).collect();
        let strs: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let rtys: Vec<Ts> = cols.iter().map(|c| borrowed_ty(&c.ty)).collect();
        let pos: Vec<usize> = (0..cols.len()).collect();
        let n = cols.len();
        let rstr = rname.to_string();
        out.extend(quote! {
            #vis struct #rname<'r> { #(pub #names: #rtys),* }
            impl<'r> #rname<'r> {
                pub fn from_row(row: &'r ::morm::Row) -> ::morm::Result<Self> {
                    Ok(Self { #(#names: row.try_get(#pos)?),* })
                }
            }
            impl ::morm::__private::serde::Serialize for #rname<'_> {
                fn serialize<S: ::morm::__private::serde::Serializer>(&self, s: S) -> ::core::result::Result<S::Ok, S::Error> {
                    use ::morm::__private::serde::ser::SerializeStruct;
                    let mut st = s.serialize_struct(#rstr, #n)?;
                    #(st.serialize_field(#strs, &self.#names)?;)*
                    st.end()
                }
            }
        });
    }

    // Registration: only `morm-mgr`'s build, with `--cfg morm_mgr`, sees it.
    // The cfg is read inside `morm`, which declares it, not in the user's
    // crate: a `#[cfg(morm_mgr)]` here would warn `unexpected_cfgs` in every
    // crate with a model.
    let meta = format!(
        "morm-meta:{}:{}",
        table,
        cols.iter()
            .map(|c| {
                let t = &c.ty;
                format!("{} {}", c.name, quote!(#t).to_string().replace(' ', ""))
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    out.extend(quote! {
        ::morm::__morm_register! { #meta }
    });
    Ok(out)
}



/// The borrowed form of a field type: text and bytes borrow, the rest stays.
fn borrowed_ty(ty: &Type) -> Ts {
    let s = quote!(#ty).to_string().replace(' ', "");
    match s.as_str() {
        "String" => quote!(&'r str),
        "Vec<u8>" => quote!(&'r [u8]),
        "Option<String>" => quote!(::core::option::Option<&'r str>),
        "Option<Vec<u8>>" => quote!(::core::option::Option<&'r [u8]>),
        _ => quote!(#ty),
    }
}

// ---------------------------------------------------------------- #[hooks]

const HOOKS: &[&str] = &[
    "validate",
    "pre_insert",
    "post_insert",
    "validate_changes",
    "pre_update",
    "post_update",
    "pre_delete",
    "post_delete",
    "after_commit",
];

/// `#[hooks] impl Hooks for M { .. }`: records which hooks the impl overrides.
#[proc_macro_attribute]
pub fn hooks(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut imp: ItemImpl = match syn::parse(item) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error().into(),
    };
    let mut flags = Vec::new();
    for it in &imp.items {
        if let ImplItem::Fn(f) = it {
            let n = f.sig.ident.to_string();
            if HOOKS.contains(&n.as_str()) {
                flags.push(format_ident!("{}", n.to_uppercase()));
            }
        }
    }
    imp.items.push(syn::parse_quote! {
        const OVERRIDES: ::morm::Overrides = ::morm::Overrides::NONE #(.with(::morm::Overrides::#flags))*;
    });
    quote!(#imp).into()
}

// ---------------------------------------------------------------- groups!

/// `groups! { admin, staff }`: the app's global groups, with `sudo` built in at bit 0.
/// The mask's size is counted here, so it is `Bits<1>` up to 64 groups.
#[proc_macro]
pub fn groups(input: TokenStream) -> TokenStream {
    let parsed = Punctuated::<Ident, Token![,]>::parse_terminated.parse(input);
    let names: Vec<Ident> = match parsed {
        Ok(p) => p.into_iter().collect(),
        Err(e) => return e.to_compile_error().into(),
    };
    if let Some(s) = names.iter().find(|n| *n == "sudo") {
        return err(s.span(), "`sudo` is built in").to_compile_error().into();
    }
    let count = names.len() + 1;
    let words = count.div_ceil(64);
    let bits: Vec<u16> = (1..count as u16).collect();
    quote! {
        /// The app's global group mask.
        pub type Mask = ::morm::Bits<#words>;
        pub const COUNT: usize = #count;
        #[allow(dead_code)]
        pub struct Groups { pub sudo: ::morm::GlobalGroup<Mask>, #(pub #names: ::morm::GlobalGroup<Mask>),* }
        pub const GROUPS: Groups = Groups { sudo: ::morm::GlobalGroup::new(0), #(#names: ::morm::GlobalGroup::new(#bits)),* };
    }
    .into()
}
