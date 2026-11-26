// R-1245's second row's test, as decided (2026-11-25): it reads only what stands before the message's value.
// `br` is a deliberate break: 0 as decided, 36 the whole message, 110 the rule it replaces, 111.. others.

// enum_in's message in English and the 16 languages Postgres 15 to 18 ship, every text of 58 releases (probes/po2.py)
pub const LANGS: [(&str, &str); 18] = [
    ("en", "invalid input value for enum %s: \"%s\""),
    ("de", "ungültiger Eingabewert für Enum %s: »%s«"),
    ("es", "la sintaxis de entrada no es válida para el enum %s: «%s»"),
    ("fr", "valeur en entrée invalide pour le enum %s : « %s »"),
    ("id", "nilai masukan tidak valid untuk enum %s : « %s »"),
    ("it", "la sintassi per l'enumerazione %s non è valida: \"%s\""),
    ("ja", "列挙型%sの不正な入力構文: \"%s\""),
    ("ka", "არასწორი შეყვანილი მნიშვნელობა ჩამონათვალისთვის %s: \"%s\""),
    ("ko", "%s 열거형의 입력 값이 잘못됨: \"%s\""),
    ("pl", "nieprawidłowa wartość wejścia dla enumeracji %s: \"%s\""),
    ("pt_BR", "valor de entrada é inválido para enum %s: \"%s\""),
    ("ru", "неверное значение для перечисления %s: \"%s\""),
    ("sv", "ogiltigt indata-värde för enum %s: \"%s\""),
    ("tr", "%s enum tipi için geçersiz değer: \"%s\""),
    ("uk", "неприпустиме вхідне значення для переліку %s: \"%s\""),
    ("zh_CN", "枚举 %s 的输入值无效: \"%s\""),
    ("zh_CN before 18.7, 17.12, 16.16 and 15.20", "对于枚举%s的输入值无效: \"%s\""),
    ("zh_TW", "列舉 %s 的輸入值無效: \"%s\""),
];
// messages the test can't read: French without its quote marks, and an English text it doesn't know
pub const UNREADABLE: [&str; 3] = ["valeur en entrée invalide pour le enum %s : << %s >>", "invalid input value for enum %s (%s)", "invalid input value for enum type %s: \"%s\""];

#[derive(Clone, Debug, PartialEq)]
pub struct Cast {
    pub schema: Option<String>,
    pub name: String,
}

pub fn fill(t: &str, ty: &str, val: &str) -> String {
    let mut it = t.splitn(3, "%s");
    let (a, b, c) = (it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
    format!("{a}{ty}{b}{val}{c}")
}
// quote_identifier, with quote_all_identifiers; keywords other than unreserved ones are always quoted
pub fn qi(x: &str, all: bool) -> String {
    let bare = x.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c == '_') && x.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    let kw = ["values", "user", "order", "for", "end", "char"].contains(&x);
    if bare && !kw && !all { x.to_string() } else { format!("\"{}\"", x.replace('"', "\"\"")) }
}

// where the value's colon is: the first colon outside double quotes, if a quote mark follows it
pub fn colon(m: &str, br: u32) -> Option<usize> {
    let mut dq = false;
    let mut last = None;
    let cs: Vec<(usize, char)> = m.char_indices().collect();
    for (k, &(i, c)) in cs.iter().enumerate() {
        if c == '"' && br != 112 {
            dq = !dq;
        }
        if c == ':' && !dq {
            let mut j = k + 1;
            if j < cs.len() && (cs[j].1 == ' ' || cs[j].1 == '\u{a0}') {
                j += 1;
            }
            let mark = j < cs.len() && if br == 117 { cs[j].1 == '"' } else { matches!(cs[j].1, '"' | '»' | '«') };
            if br == 111 {
                if mark {
                    last = Some(i);
                }
                continue;
            }
            if br == 118 && !mark {
                continue; // any later colon a quote mark follows
            }
            return if mark { Some(i) } else { None };
        }
    }
    last
}
fn cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff)
}
fn word(c: char, br: u32) -> bool {
    c == '_' || if br == 119 { c.is_alphanumeric() } else { c.is_alphanumeric() && !cjk(c) }
}
// one name's part at `i`: (text, whether quoted, end)
fn part(cs: &[char], i: usize, br: u32) -> Option<(String, usize)> {
    if i >= cs.len() {
        return None;
    }
    if cs[i] == '"' {
        let mut t = String::new();
        let mut j = i + 1;
        while j < cs.len() {
            if cs[j] == '"' {
                if j + 1 < cs.len() && cs[j + 1] == '"' {
                    t.push('"');
                    j += 2;
                    continue;
                }
                return Some((if br == 116 { format!("\"{t}\"") } else { t }, j + 1));
            }
            t.push(cs[j]);
            j += 1;
        }
        return None;
    }
    let mut j = i;
    while j < cs.len() && word(cs[j], br) {
        j += 1;
    }
    let t: String = cs[i..j].iter().collect();
    let ok = !t.is_empty() && !t.starts_with(|c: char| c.is_ascii_digit()) && t.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if ok { Some((t, j)) } else { None }
}
// a name, with its schema or without
fn name(cs: &[char], i: usize, br: u32) -> Option<(Vec<String>, usize)> {
    let (a, j) = part(cs, i, br)?;
    if j < cs.len() && cs[j] == '.' {
        if let Some((b, k)) = part(cs, j + 1, br) {
            return Some((vec![a, b], k));
        }
    }
    Some((vec![a], j))
}
fn is(n: &[String], casts: &[Cast], br: u32) -> bool {
    casts.iter().any(|c| match n {
        // the four answers: a name printed with no schema names only a cast written with none (break 121: any)
        [a] => *a == c.name && (br == 121 || c.schema.is_none()),
        [s, a] => *a == c.name && (br == 115 || c.schema.as_deref() == Some(s)),
        _ => false,
    })
}
fn anywhere(t: &str, casts: &[Cast], br: u32) -> bool {
    let cs: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if let Some((n, j)) = name(&cs, i, br) {
            if is(&n, casts, br) {
                return true;
            }
            i = j.max(i + 1);
        } else if word(cs[i], br) {
            while i < cs.len() && word(cs[i], br) {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    false
}
pub const EN: &str = "invalid input value for enum ";

/// None: the test can't read the message. Some(true): it names a type the statement casts a parameter to.
pub fn read(m: &str, casts: &[Cast], br: u32) -> Option<bool> {
    if br == 36 {
        return Some(anywhere(m, casts, 0));
    }
    let at = colon(m, br)?;
    let before = m[..at].trim_end_matches([' ', '\u{a0}']);
    if let Some(r) = before.strip_prefix(EN) {
        if br != 113 {
            let cs: Vec<char> = r.chars().collect();
            let (n, j) = name(&cs, 0, br)?;
            return if j == cs.len() { Some(is(&n, casts, br)) } else { None };
        }
    }
    Some(anywhere(before, casts, br))
}
/// the rule this one replaces: the call's own bound texts taken out of the message, the rest searched
pub fn old_rule(m: &str, casts: &[Cast], bound: &[String]) -> bool {
    if let Some(at) = colon(m, 0) {
        let v = m[at + 1..].trim_start_matches([' ', '\u{a0}']);
        let mut c = v.chars();
        c.next();
        c.next_back();
        let v = c.as_str().trim_matches('\u{a0}').trim_matches(' ');
        if bound.iter().any(|b| b == v) {
            return anywhere(&m[..at], casts, 0);
        }
    }
    anywhere(m, casts, 0)
}
/// whether the second row takes it
pub fn second(m: &str, casts: &[Cast], br: u32, bound: &[String]) -> bool {
    if br == 110 {
        return !old_rule(m, casts, bound);
    }
    match read(m, casts, br) {
        Some(named) => !named,
        None => br != 114, // the safe side: a fresh run decides
    }
}
/// the words of a language's message that a type named so would be printed as
pub fn words(t: &str) -> Vec<String> {
    let head = fill(t, "\u{1}", "");
    let at = colon(&head, 0).unwrap_or(head.len());
    let cs: Vec<char> = head[..at].chars().collect();
    let mut v = vec![];
    let mut i = 0;
    while i < cs.len() {
        if word(cs[i], 0) {
            let mut j = i;
            while j < cs.len() && word(cs[j], 0) {
                j += 1;
            }
            if let Some((t, k)) = part(&cs, i, 0) {
                if k == j {
                    v.push(t);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    v
}

// ---- every language, name and value ----
pub fn exhaustive() {
    let mut n = 0u64;
    let casts = [
        (None, "st"), (None, "St"), (None, "st old"), (None, "a: \"b"), (None, "»x: «y"), (None, "x\"\"y"), (None, "value"), (None, "input"), (None, "for"),
        (None, "入力"), (None, "a.b"), (None, "_x1"), (None, "1x"), (None, ": \""), (None, "\""), (None, "st:"), (None, "l"), (None, "enum"), (None, "entrada"),
        (Some("billing"), "order_state"), (Some("My: \"S"), "st"), (Some("a.b"), "c.d"),
    ];
    let values = |t: &str| -> Vec<String> {
        vec!["c".into(), String::new(), format!("in {t}"), format!("x \"{t}\""), format!("{t}: \"bad"), format!("»{t}«"), format!(": \"{t}\""), format!(": «{t} "), format!("invalid input value for enum {t}: \"z"), t.into(), "é".into()]
    };
    let (mut own, mut stale, mut kept_word) = (0u64, 0u64, 0u64);
    for (lang, t) in LANGS {
        let ws = words(t);
        for (sch, nm) in casts {
            let cast = [Cast { schema: sch.map(String::from), name: nm.to_string() }];
            for all in [false, true] {
                // a cast written with its schema is named only by a name printed with it: printed with none, as
                // its own type prints with its schema on the search_path or a kept one moved where the path finds
                // it, the message can mean either, so it's the second row's
                let own_prints: Vec<String> = match sch {
                    Some(s) => vec![format!("{}.{}", qi(s, all), qi(nm, all))],
                    None => vec![qi(nm, all)],
                };
                let mut kept: Vec<String> = [format!("{nm}_old"), format!("{nm} old"), format!("my {nm}"), format!("x_{nm}"), format!("{nm}2"), format!("{nm}: \"{nm}"), format!("{nm}.{nm}"), format!("«{nm}»")].iter().map(|k| qi(k, all)).collect();
                if sch.is_none() {
                    kept.push(format!("{}.{}", qi("old", all), qi(nm, all)));
                    kept.push(format!("{}.{}", qi(nm, all), qi("other", all)));
                } else {
                    kept.push(format!("{}.{}", qi("old", all), qi(nm, all)));
                    kept.push(format!("{}.{}", qi(sch.unwrap(), all), qi("other", all)));
                    kept.push(qi(nm, all));
                }
                for v in values(nm) {
                    for p in &own_prints {
                        let m = fill(t, p, &v);
                        n += 1;
                        own += 1;
                        assert_eq!(read(&m, &cast, 0), Some(true), "own error: {lang} {m}");
                    }
                    for p in &kept {
                        let m = fill(t, p, &v);
                        n += 1;
                        stale += 1;
                        let is_word = sch.is_none() && lang != "en" && ws.iter().any(|w| w == nm);
                        if is_word {
                            kept_word += 1;
                            assert_eq!(read(&m, &cast, 0), Some(true), "a word of its message: {lang} {m}");
                        } else {
                            assert_eq!(read(&m, &cast, 0), Some(false), "a stale cast's: {lang} {m}");
                        }
                    }
                }
            }
        }
        println!("{lang}: words a type could be named as: {ws:?}");
    }
    for t in UNREADABLE {
        for v in values("st") {
            let m = fill(t, "st", &v);
            let c = [Cast { schema: None, name: "st".into() }];
            assert_eq!(read(&m, &c, 0), None, "unreadable: {m}");
            assert!(second(&m, &c, 0, &[]));
        }
    }
    println!("messages read: {n}, {own} of the cast's own type, {stale} of a kept one, {kept_word} of those with the type named as a word of its message");
}
/// lines of: kind<TAB>schema or empty<TAB>name<TAB>message with \n and \t escaped; own or kept
pub fn from_file(f: &str) {
    let (mut own, mut kept, mut words_, mut bad, mut second_, mut left) = (0, 0, 0, 0, 0, 0);
    for l in std::fs::read_to_string(f).unwrap().lines() {
        let x: Vec<&str> = l.split('\t').collect();
        let c = [Cast { schema: if x[1].is_empty() { None } else { Some(x[1].into()) }, name: x[2].into() }];
        let r = read(x[3], &c, 0);
        match x[0] {
            "own" => own += (r == Some(true)) as u32,
            "kept" => kept += (r == Some(false)) as u32,
            "word" => words_ += (r == Some(true)) as u32,
            // the four answers: a cast written with its schema, its name printed with none
            "second" => second_ += (r != Some(true)) as u32,
            // what the four answers leave: a stale conversion of the user's own SQL naming the cast's type
            "left" => left += (r == Some(true)) as u32,
            _ => panic!("{l}"),
        }
        let want = match x[0] { "kept" => r == Some(false), "second" => r != Some(true), _ => r == Some(true) };
        if !want {
            bad += 1;
            println!("NOT AS EXPECTED {r:?} {l}");
        }
    }
    println!("from Postgres: {own} own, {kept} kept, {words_} kept with the type named as a word of its message, {second_} the second row's always, {left} the case left; {bad} not as expected");
}
