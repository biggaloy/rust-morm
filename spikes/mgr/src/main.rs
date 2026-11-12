//! Stands in for the generated `morm-mgr`. Built with `--cfg morm_mgr`, it
//! prints every registered model, one per line.

// Each crate with models is named once, so the linker keeps it even when no
// code calls into it.
use models_a as _;
use models_b as _;
use models_unref as _;

fn main() {
    let mut all: Vec<&str> = morm::__private::registered().collect();
    all.sort();
    for m in all {
        println!("{m}");
    }
}
