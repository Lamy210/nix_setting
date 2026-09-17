#![allow(dead_code)]

// Contract coverage for the Windows doctor state matrix.
//
// The backend module is included into this test crate so private runtime seams
// can be exercised hermetically without requiring WSL on the CI host.
include!("../src/windows_backend.rs");

#[cfg(test)]
mod doctor_matrix {
    use std::collections::VecDeque;

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn output(code: i32, stdout: &str) -> ProcessOutput {
        ProcessOutput {
            code: Some(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    fn output_err(code: i32, stderr: &str) -> ProcessOutput {
        ProcessOutput {
            code: Some(code),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[derive(Default)]
    struct FakeRuntime {
        captures: VecDeque<(Vec<String>, Result<ProcessOutput, String>)>,
        seen: Vec<Vec<String>>,
    }

    impl FakeRuntime {
        fn with_capture(mut self, expected: &[&str], result: ProcessOutput) -> Self {
            self.captures.push_back((args(expected), Ok(result)));
            self
        }

        fn with_capture_error(mut self, expected: &[&str], error: &str) -> Self {
            self.captures
                .push_back((args(expected), Err(error.to_owned())));
            self
        }
    }

    impl WslRuntime for FakeRuntime {
        fn capture(&mut self, argv: &[String]) -> Result<ProcessOutput, String> {
            self.seen.push(argv.to_vec());
            let (expected, result) = self.captures.pop_front().expect("unexpected capture call");
            assert_eq!(argv, expected);
            result
        }

        fn status(&mut self, argv: &[String]) -> Result<Option<i32>, String> {
            panic!("doctor must not delegate an operational command: {argv:?}");
        }
    }

    fn compatible_backend_json(version: &str) -> String {
        format!(
            r#"{{"protocol_version":1,"app_version":"{version}","os":"linux","arch":"x86_64"}}"#
        )
    }

    fn doctor_report(runtime: &mut FakeRuntime) -> String {
        let outcome = dispatch_with_runtime(
            runtime,
            HostPlatform::Windows,
            &args(&["doctor"]),
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .expect("Windows doctor should return diagnostics instead of a dispatch error");

        let EarlyDispatch::Doctor(report) = outcome else {
            panic!("expected Windows doctor diagnostics");
        };
        report
    }

    #[test]
    fn doctor_reports_wsl_unavailable_without_attempting_helper_probe() {
        let mut runtime = FakeRuntime::default().with_capture_error(
            &["--list", "--quiet"],
            "failed to execute wsl.exe: executable not found",
        );

        let report = doctor_report(&mut runtime);

        assert!(report.contains("host: windows"));
        assert!(report.contains("available: no"));
        assert!(report.contains("failed to execute wsl.exe"));
        assert!(report.contains("ready: no"));
        assert_eq!(runtime.seen, vec![args(&["--list", "--quiet"])]);
    }

    #[test]
    fn doctor_reports_no_registered_distribution() {
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, ""))
            .with_capture(&["--list", "--verbose"], output(0, ""));

        let report = doctor_report(&mut runtime);

        assert!(report.contains("no WSL distributions are registered"));
        assert!(report.contains("ready: no"));
        assert_eq!(runtime.seen.len(), 2);
    }

    #[test]
    fn doctor_reports_selected_wsl1_as_not_ready() {
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Legacy\n"))
            .with_capture(&["--list", "--verbose"], output(0, "* Legacy Running 1\n"));

        let report = doctor_report(&mut runtime);

        assert!(report.contains("Legacy: WSL1"));
        assert!(report.contains("WSL2 is required"));
        assert!(report.contains("ready: no"));
        assert_eq!(runtime.seen.len(), 2);
    }

    #[test]
    fn doctor_reports_missing_helper_without_losing_host_state() {
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu\n"))
            .with_capture(&["--list", "--verbose"], output(0, "* Ubuntu Running 2\n"))
            .with_capture(
                &["-d", "Ubuntu", "--exec", "schneeforge", "__backend-info"],
                output_err(127, "schneeforge: not found"),
            );

        let report = doctor_report(&mut runtime);

        assert!(report.contains("host: windows"));
        assert!(report.contains("Ubuntu: WSL2"));
        assert!(report.contains("selected: Ubuntu (default)"));
        assert!(report.contains("compatible: no"));
        assert!(report.contains("schneeforge: not found"));
        assert!(report.contains("ready: no"));
        assert_eq!(runtime.seen.len(), 3);
    }

    #[test]
    fn doctor_reports_helper_version_mismatch_as_not_ready() {
        let helper = compatible_backend_json("9.9.9");
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu\n"))
            .with_capture(&["--list", "--verbose"], output(0, "* Ubuntu Running 2\n"))
            .with_capture(
                &["-d", "Ubuntu", "--exec", "schneeforge", "__backend-info"],
                output(0, &helper),
            );

        let report = doctor_report(&mut runtime);

        assert!(report.contains("compatible: no"));
        assert!(report.contains("WSL helper version mismatch"));
        assert!(report.contains("install the matching SchneeForge version in WSL"));
        assert!(report.contains("ready: no"));
        assert_eq!(runtime.seen.len(), 3);
    }

    #[test]
    fn doctor_reports_compatible_wsl2_backend_as_ready() {
        let version = env!("CARGO_PKG_VERSION");
        let helper = compatible_backend_json(version);
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu\n"))
            .with_capture(&["--list", "--verbose"], output(0, "* Ubuntu Running 2\n"))
            .with_capture(
                &["-d", "Ubuntu", "--exec", "schneeforge", "__backend-info"],
                output(0, &helper),
            );

        let report = doctor_report(&mut runtime);

        assert!(report.contains("available: yes"));
        assert!(report.contains("Ubuntu: WSL2"));
        assert!(report.contains("selected: Ubuntu (default)"));
        assert!(report.contains("compatible: yes"));
        assert!(report.contains(&format!("version: {version}")));
        assert!(report.contains("os: linux"));
        assert!(report.contains("arch: x86_64"));
        assert!(report.contains("ready: yes"));
        assert_eq!(runtime.seen.len(), 3);
    }
}
