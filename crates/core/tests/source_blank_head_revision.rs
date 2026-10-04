#![cfg(unix)]

use schneeforge_core::error::Error;
use schneeforge_core::source::SourceResolver;
use schneeforge_core::tool::{ResolvedTool, ToolSource};
use std::os::unix::fs::PermissionsExt;

#[test]
fn detached_source_rejects_blank_head_revision() {
    let dir = std::env::temp_dir().join(format!(
        "sf-source-blank-head-revision-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let fake_git = dir.join("fake-git");
    std::fs::write(
        &fake_git,
        r#"#!/bin/sh
if [ "$3" = "rev-parse" ] && [ "$4" = "--abbrev-ref" ]; then
  printf 'HEAD\n'
  exit 0
fi
if [ "$3" = "tag" ]; then
  exit 0
fi
if [ "$3" = "rev-parse" ] && [ "$4" = "HEAD" ]; then
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_git).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_git, permissions).unwrap();

    let git = ResolvedTool {
        path: fake_git,
        source: ToolSource::Path,
        version: None,
    };

    let result = SourceResolver::new().detect(dir.to_str().unwrap(), &git);
    assert!(
        result.is_err(),
        "blank detached HEAD revision must fail closed; got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(matches!(err, Error::Command { .. }), "{err}");
    assert!(
        err.to_string().contains("rev-parse HEAD returned empty output"),
        "{err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
