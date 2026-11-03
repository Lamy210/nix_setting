# Implementation tasks (do not mark complete based on source-only review)

## 1. Research and approval
- [ ] 1.1 Confirm upstream stable vs staging revisions and exact Node/Brotli/c-ares patch status; record immutable provenance and hashes.
- [ ] 1.2 Compare alternatives A/B/C against ABI, closure size, security coverage and maintenance cost; approve implementation design.
- [ ] 1.3 Validate this OpenSpec change strictly and obtain proposal approval.

## 2. Security remediation
- [ ] 2.1 Implement a narrowly pinned Node 24 package >=24.22.2, with patched Brotli/c-ares and audited build input.
- [ ] 2.2 Apply to the root, standalone Node template and devenv template independently without changing Node major or unrelated runtimes.
- [ ] 2.3 Reconcile Draft #257 root lock to prevent downgrade.

## 3. Verification
- [ ] 3.1 Add deterministic actual Node execution/version assertions in all three shells.
- [ ] 3.2 Add transitive Brotli/c-ares patched-closure evidence and negative regression checks; fail closed on unknown.
- [ ] 3.3 Validate Linux/macOS compatibility, pnpm/Bun operation, Nix eval/build, required Rust/OpenSpec and release artifact CI.
- [ ] 3.4 Inventory host Node/mise/running daemons and document opt-in activation / rollback separately.

## 4. Release
- [ ] 4.1 Link precise commits, logs, release input SHAs and host caveats to #275, #265, #271.
- [ ] 4.2 Merge implementation only after CI and acceptance; archive the OpenSpec change in a separate PR.
