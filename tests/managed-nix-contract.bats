#!/usr/bin/env bats

# Managed Nix upstream contract test (PR #13 review 指摘対応)
#
# SchneeForge が pin している nix-installer (bootstrap-manifest.toml) の実 binary を
# download + SHA256 検証し、`install <plan>` の引数形式 (positional plan) が
# upstream に受け入れられることを検証する。
#
# 注意: plan file は存在しない path を渡す。clap の parse は引数形式の検証に
# 先行するため、「unknown argument '--plan'」error ではなく file 読み込み error
# (No such file) になれば positional として受理されている証拠になる。

MANIFEST="$BATS_TEST_DIRNAME/../bootstrap-manifest.toml"

installer_version() {
  awk -F'"' '/^version *= */ {print $2; exit}' "$MANIFEST"
}

expected_sha_for() {
  local arch="$1"
  awk -F'"' -v k="$arch" '/^'"$arch"' *= */ {print $2; exit}' "$MANIFEST"
}

download_installer() {
  local version="$1" dest="$2"
  curl -fsSL -o "$dest" \
    "https://github.com/NixOS/nix-installer/releases/download/${version}/nix-installer-x86_64-linux"
}

@test "bootstrap-manifest.toml pins a version" {
  version="$(installer_version)"
  [ -n "$version" ]
  case "$version" in
    v*) false ;;  # 'v' prefix は付けない (URL は bare version)
    *) true ;;
  esac
}

@test "pinned installer binary downloads and matches manifest sha256" {
  version="$(installer_version)"
  expected="$(expected_sha_for x86_64-linux)"
  [ -n "$expected" ]

  tmp_bin="$(mktemp -t nix-installer-XXXXXX)"
  download_installer "$version" "$tmp_bin"

  actual="$(sha256sum "$tmp_bin" | awk '{print $1}')"
  [ "$actual" = "$expected" ]

  chmod +x "$tmp_bin"
  rm -f "$tmp_bin"
}

@test "upstream install accepts positional plan path (rejects --plan flag)" {
  version="$(installer_version)"
  tmp_bin="$(mktemp -t nix-installer-XXXXXX)"
  download_installer "$version" "$tmp_bin"
  chmod +x "$tmp_bin"

  # positional plan: clap parse を通過する。root 無し / file 無しで error に
  # なるが、「unexpected argument」等の usage error にはならない。
  # CI は非 root かつ terminal 無しなので sudo 昇格が失敗して終わる。
  run "$tmp_bin" install /nonexistent/plan.json --logger json --no-confirm
  positional_output="$output"
  case "$positional_output" in
    *"unexpected argument"*|*"unknown argument"*)
      echo "positional plan was rejected by clap:"
      echo "$positional_output"
      rm -f "$tmp_bin"
      false
      ;;
  esac
  # 実行まで到達した証拠: version banner (INFO log) または sudo escalation のいずれか
  echo "$positional_output" | grep -qE "nix-installer v|needs to run as .root.|Reading plan|No such file"

  # --plan flag: upstream は long flag を持たないので unknown flag で reject される。
  # もし受理されるようになったら (upstream 仕様変更) SchneeForge 側 install_args の
  # 見直しが必要なので、この test が契約変更を検知する。
  run "$tmp_bin" install --plan /nonexistent/plan.json --logger json --no-confirm
  if ! echo "$output" | grep -qF "unexpected argument '--plan'"; then
    echo "--plan flag was ACCEPTED (upstream contract change?):"
    echo "$output"
    rm -f "$tmp_bin"
    false
  fi

  rm -f "$tmp_bin"
}
