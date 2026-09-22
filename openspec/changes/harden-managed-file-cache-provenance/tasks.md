## 1. Cache provenance

- [x] 1.1 Add provenance sidecar schema with repository identity and content SHA-256.
- [x] 1.2 Verify provenance schema / repository / digest before returning a cache hit.
- [x] 1.3 Treat legacy cache without provenance as an untrusted miss.
- [x] 1.4 Write replacement content before provenance so interrupted writes fail closed.

## 2. Input boundary hardening

- [x] 2.1 Require managed repo-file refs to be valid release SemVer tags.
- [x] 2.2 Reject unsafe cache/URL path components.
- [x] 2.3 Reserve the provenance sidecar suffix from content file names.
- [x] 2.4 Keep raw cache path construction module-private.

## 3. Regression coverage

- [x] 3.1 Cover same-tag fork collision.
- [x] 3.2 Cover tampered content digest mismatch.
- [x] 3.3 Cover legacy cache migration behavior and verified offline reuse.
- [x] 3.4 Cover mutable refs, traversal/separator input, and provenance-aware cache reporting.

## 4. Verification and lifecycle

- [ ] 4.1 Pass `openspec validate harden-managed-file-cache-provenance --strict`.
- [ ] 4.2 Pass repository-wide OpenSpec strict validation and required CI.
- [ ] 4.3 Squash merge implementation PR to `develop`.
- [ ] 4.4 Archive + canonical spec sync in a separate `chore/archive-harden-managed-file-cache-provenance` PR.
