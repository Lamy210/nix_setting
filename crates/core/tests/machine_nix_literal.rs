use std::path::PathBuf;

use schneeforge_core::{Architecture, MachineFacts, OperatingSystem};

#[test]
fn machine_facts_escape_nix_interpolation_metacharacters() {
    let facts = MachineFacts {
        username: "user${builtins.abort \"username\"}".to_string(),
        home_directory: PathBuf::from("/home/${builtins.abort \"home\"}"),
        os: OperatingSystem::Linux,
        architecture: Architecture::X86_64,
        hostname: "host${builtins.abort \"hostname\"}".to_string(),
    };

    let nix = facts.to_machine_nix();

    assert!(
        nix.contains("username = \"user\\${builtins.abort \\\"username\\\"}\";"),
        "generated machine.nix must keep username interpolation literal: {nix}"
    );
    assert!(
        nix.contains("homeDirectory = \"/home/\\${builtins.abort \\\"home\\\"}\";"),
        "generated machine.nix must keep home interpolation literal: {nix}"
    );
    assert!(
        nix.contains("hostname = \"host\\${builtins.abort \\\"hostname\\\"}\";"),
        "generated machine.nix must keep hostname interpolation literal: {nix}"
    );
}
