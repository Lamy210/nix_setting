#[test]
fn cli_uninstall_delegates_receipt_execution_to_managed_nix_core() {
    let source = include_str!("../src/nix_cmd.rs");

    assert!(
        !source.contains("let uninstall_args = build_uninstall_args(Some(&receipt_path));"),
        "CLI uninstall must not rebuild receipt arguments through the lossy String helper"
    );
    assert!(
        source.contains("mn.run_uninstall(&binary, Some(&receipt_path))"),
        "CLI uninstall must delegate receipt-path execution to ManagedNix Core"
    );
}
