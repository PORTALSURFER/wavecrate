//! Source-quality guardrails for the Wavecrate/Radiant GUI boundary.

use std::fs;

#[test]
fn gui_module_stays_a_pure_radiant_reexport_boundary() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/src/gui/mod.rs"))
        .expect("src/gui/mod.rs should be readable");

    for forbidden in ["pub trait ", "impl ", "fn ", "struct ", "enum ", "const "] {
        assert!(
            !source.contains(forbidden),
            "src/gui should stay a pure Radiant re-export boundary; found `{forbidden}`"
        );
    }
}
