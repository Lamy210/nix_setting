# Design: signed-version binding for GUI updater

## D1. Threat model

Updater artifact signature alone authenticates the downloaded bytes, but endpoint metadata is not itself signed.
Without signed-version binding, a response can announce version B while pointing to a valid signed artifact from
version A. Version comparison sees B while signature verification authenticates only A.

Therefore the update artifact signature MUST bind the app version and runtime MUST compare that signed version
with the endpoint-announced version.

## D2. Tauri version floor

Release artifact generation MUST use Tauri CLI >= 2.11.5 because that release records the app version in the
updater signature trusted comment when `tauri build` produces updater artifacts.

SchneeForge pins the prebuilt CLI binary for reproducibility, so this change updates:
- `TAURI_CLI_VERSION` to `2.11.5`
- SHA256 for `cargo-tauri-aarch64-apple-darwin.zip` to the GitHub release digest

The Rust lockfile already resolves:
- `tauri = 2.11.5`
- `tauri-plugin-updater = 2.12.0`

No dependency bump is required for runtime support.

## D3. Runtime enforcement

`tauri-plugin-updater 2.12.0` exposes config `requireSignedVersion` with default false.

SchneeForge SHALL set:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "",
      "requireSignedVersion": true
    }
  }
}
```

The empty static pubkey is not a trust root. Activated builds continue to register the plugin with the
compile-time production public key through the existing Rust builder, which overrides the config pubkey.

Endpoints continue to be supplied at runtime by the existing backend builder. Staged activation semantics
are unchanged.

## D4. Fail-closed compatibility

Production activation starts only after this change. Therefore SchneeForge does not need compatibility with
pre-2.11.5 updater signatures.

If a signature has no signed version, or the signed and announced versions differ, update check/install MUST
fail and the Releases-link fallback remains available.

## D5. Regression protection

CI SHALL verify:
- pinned Tauri CLI version is exactly 2.11.5 or later policy-approved value
- pinned SHA256 matches the reviewed 2.11.5 aarch64 macOS release asset
- updater config has `requireSignedVersion: true`
- updater config does not contain a production/test public key
- production activation remains gated and secret-free in PR CI

## D6. Activation boundary

This change does not satisfy the activation tasks:
- Final Acceptance
- production key provision
- signed N→N+1 E2E
- tampered artifact E2E

Those remain explicit release gates.
