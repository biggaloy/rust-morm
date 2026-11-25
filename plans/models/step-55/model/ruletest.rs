mod rule;
fn main() {
    match std::env::args().nth(1) {
        Some(f) => rule::from_file(&f),
        None => rule::exhaustive(),
    }
}
