use std::path::PathBuf;

use ts_rs::TS;

/// Run with:
///   cargo test --test generate_bindings
///
/// Writes `frontend/src/bindings/RenderParams.ts` (relative to the workspace root)
/// from the Rust struct definition, so the TypeScript interface never drifts.
///
/// The output directory is resolved at runtime from CARGO_MANIFEST_DIR so that
/// the test works correctly regardless of which directory `cargo test` is invoked from.
#[test]
fn export_render_params() {
    // CARGO_MANIFEST_DIR is set by Cargo to the directory containing Cargo.toml,
    // i.e. the `backend/` directory.  We step one level up to the workspace root,
    // then descend into the frontend bindings directory.
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set by Cargo when running tests"),
    );
    let bindings_dir = manifest_dir
        .parent()
        .expect("backend/ must have a parent directory (the workspace root)")
        .join("frontend/src/bindings");

    std::fs::create_dir_all(&bindings_dir)
        .expect("failed to create frontend/src/bindings directory");

    ascii_renderer::renderer::RenderParams::export_all_to(&bindings_dir)
        .expect("ts-rs failed to export RenderParams bindings");
}
