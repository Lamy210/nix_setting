#![allow(dead_code)]

// Contract coverage for launcher option termination on Windows.
//
// Include the backend module so dispatch classification can be exercised
// hermetically on non-Windows CI hosts without invoking WSL.
include!("../src/windows_backend.rs");

#[cfg(test)]
mod passthrough_dispatch {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn help_and_version_after_double_dash_remain_delegated_arguments() {
        for literal in ["--help", "-h", "--version", "-V"] {
            assert_eq!(
                classify_dispatch(HostPlatform::Windows, &args(&["apply", "--", literal])),
                DispatchKind::Delegate,
                "literal {literal} after -- must be delegated with the operational command"
            );
        }
    }
}
