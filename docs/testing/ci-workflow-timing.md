# CI macOS timing baseline (diagnostic only)

Use this procedure for [Issue #252](https://github.com/Lamy210/nix_setting/issues/252). It reads **saved GitHub Actions job JSON** and makes no changes to workflows, caches, trusted inputs, or branch protection.

## Collect

From a workstation with `gh auth login` already configured, retrieve the full job list from each relevant `check` workflow run. Preserve run IDs, commit SHAs, OS/Xcode matrix version, workflow revision, run attempt, and cache context separately so comparisons can be matched.

```bash
gh api "repos/Lamy210/nix_setting/actions/runs/39801714239/jobs?per_page=100" > /tmp/jobs-39801714239.json
gh api "repos/Lamy210/nix_setting/actions/runs/39698132013/jobs?per_page=100" > /tmp/jobs-39698132013.json
python3 scripts/ci/report-workflow-timings.py /tmp/jobs-39801714239.json /tmp/jobs-39698132013.json
python3 -m unittest discover -s tests -p 'test_ci_timings.py'
```

If a workflow grows beyond one API page, save every page (`?per_page=100&page=N`) and pass every downloaded JSON file as an argument. The script deduplicates identical GitHub job IDs across files. If multiple captures of the same ID include both queued and completed snapshots, the completed snapshot takes precedence regardless of argument order. Two completed snapshots with conflicting job conclusion or timestamps are rejected rather than silently skewing measurements.

## Interpretation

- **Created → started:** the interval between GitHub job creation and job start. This includes scheduling, concurrency, and runner allocation. It is **not proof** of hosted-runner queueing alone.
- **Started → completed:** runner job execution elapsed time. It is not a billing figure.
- **Home Manager:** elapsed time of the named `build macos home-manager` step, reported only when that step succeeded.
- Reports group **macos-stable (macos-15)**, **macos-stable (macos-26)**, and **release-artifact-check** by exact job name. Changing the matrix Xcode path results in a separate group to prevent accidental comparison.
- p50 and p95 are nearest-rank percentiles, rounded to seconds. Only **completed successful** jobs contribute. Pending, cancelled, failed, skipped, and other conclusions have separate counters. Importing an identical job twice does not inflate the sample.
- Fewer than ten successful samples are exploratory. Include matched cold/warm-cache coverage, cancellation counts, the actual config and each run attempt. No cache hit/miss rates or cost can be inferred from job timestamps alone.
- Missing or impossible timestamps cause a nonzero exit code: never silently turn malformed API evidence into a reassuring zero-duration statistic.

## Optimization acceptance

The report **does not change** the required `openspec-check`, `flake-check`, `rust-check`, `lint`, `bootstrap-test`, `managed-nix-e2e`, or `release-artifact-check` contexts. Both macOS 15/26 build compatibility lanes, full Home Manager and nix-darwin system realization, and CLI/DMG mounted-app portability are required. Make cache or matrix optimizations in a separate, reviewed OpenSpec change with matched benchmark evidence and rollback criteria.
