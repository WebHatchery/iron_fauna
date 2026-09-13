// The shared file-size gate from CODE_STANDARDS §2.2 enforces the 800-line
// hard limit for every Rust source file, including integration tests.

#[test]
fn source_files_stay_under_the_limit() {
    macroquad_toolkit::source_gate::assert_source_files_within_limit(
        env!("CARGO_MANIFEST_DIR"),
        &[],
    );
}
