#!/usr/bin/env bash
# 配布 artifact ごとに cosign attestation bundle (SchneeSoftwareDistribution
# platform の命名規約) を生成する:
#   <asset>.sig.bundle        cosign sign-blob (keyless OIDC)
#   <asset>.provenance.bundle cosign attest-blob + SLSA v0.2 predicate
#   <asset>.spdx.json         syft SPDX SBOM (CLI binary のみ。DMG は対象外)
#
# 生成後、platform と同じ identity pin + OIDC issuer で自己検証する
# (失敗時は exit 1 = release を作らない)。
#
# release.yml の release job (id-token: write 付き) からのみ実行する
# (証明書 identity が workflow に pin されるため。PR CI / local では実行しない)。
#
# usage: attest-release-assets.sh <TAG> <SOURCE_SHA> <ARTIFACT>...
#   TAG        release tag (例: v0.2.0)
#   SOURCE_SHA tag の commit SHA
#   ARTIFACT   配布 artifact の path (複数)。basename が asset 名になる
set -euo pipefail

TAG="${1:?usage: $0 <TAG> <SOURCE_SHA> <ARTIFACT>...}"
SOURCE_SHA="${2:?usage: $0 <TAG> <SOURCE_SHA> <ARTIFACT>...}"
shift 2

REPOSITORY="Lamy210/nix_setting"
WORKFLOW_PATH=".github/workflows/release.yml"
OIDC_ISSUER="https://token.actions.githubusercontent.com"
# platform (scripts/verify-attestations.ts identityRegexpFor) と同じ anchor 付き pin
IDENTITY_REGEXP="^https://github\.com/${REPOSITORY//\//\\/}/${WORKFLOW_PATH//\//\\/}@"
OUT_DIR="attestations"

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
mkdir -p "$OUT_DIR"

cosign() { nix run nixpkgs#cosign -- "$@"; }

for artifact in "$@"; do
	name="$(basename "$artifact")"
	echo "==> attesting ${name}"

	# 1. cosign 署名 bundle (keyless。証明書 SAN = release workflow identity)
	cosign sign-blob --yes --bundle "${OUT_DIR}/${name}.sig.bundle" "$artifact"

	# 2. SLSA v0.2 provenance bundle (subject は cosign が blob から設定)
	python3 "$REPO_ROOT/scripts/ci/slsa_predicate.py" \
		"$TAG" "$SOURCE_SHA" "refs/tags/${TAG}" "${OUT_DIR}/${name}.predicate.json"
	cosign attest-blob \
		--yes \
		--type slsaprovenance \
		--predicate "${OUT_DIR}/${name}.predicate.json" \
		--bundle "${OUT_DIR}/${name}.provenance.bundle" \
		"$artifact"

	# 3. SPDX SBOM (CLI binary のみ。syft は UDIF (DMG) を scan できない)
	case "$name" in
	*.dmg) echo "    (skip SBOM: syft cannot scan DMG)" ;;
	*)
		nix run nixpkgs#syft -- scan "$artifact" -o "spdx-json=${OUT_DIR}/${name}.spdx.json" >/dev/null
		;;
	esac

	# 4. 自己検証 gate (platform と同じ pin。失敗したら release を出さない)
	cosign verify-blob \
		--bundle "${OUT_DIR}/${name}.sig.bundle" \
		--certificate-identity-regexp "$IDENTITY_REGEXP" \
		--certificate-oidc-issuer "$OIDC_ISSUER" \
		"$artifact"
	cosign verify-blob-attestation \
		--bundle "${OUT_DIR}/${name}.provenance.bundle" \
		--certificate-identity-regexp "$IDENTITY_REGEXP" \
		--certificate-oidc-issuer "$OIDC_ISSUER" \
		"$artifact"

	# predicate は attest-blob の入力用なので release asset には含めない
	rm -f "${OUT_DIR}/${name}.predicate.json"
done

echo "generated attestation assets:"
ls -1 "$OUT_DIR"
