//! The app binary proof 10 inspects: it uses models from two crates, and its
//! build must carry no registration metadata.

use morm::Model;

fn main() {
    println!("{}", models_a::Order::SELECT);
    println!("{}", models_b::Customer::SELECT);
    println!("{}", app::Product::SELECT);
}
