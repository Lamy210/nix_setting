#!/usr/bin/env bats

setup() {
  APT_TEST_TEMP="$(mktemp -d)"
  export APT_TEST_LOG="$APT_TEST_TEMP/calls"
  cat > "$APT_TEST_TEMP/timeout" <<'MOCK'
#!/usr/bin/env bash
# Drop the three timeout options, then invoke the mocked sudo.
shift 3
exec "$@"
MOCK
  cat > "$APT_TEST_TEMP/sudo" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$APT_TEST_LOG"
case "${APT_TEST_FAILURE:-}" in
  always)
    exit 100
    ;;
  update_once)
    if [[ "$*" == *" update -qq" ]]; then
      count="$(grep -c ' update -qq' "$APT_TEST_LOG")"
      if ((count == 1)); then
        exit 100
      fi
    fi
    ;;
  install_once)
    if [[ "$*" == *" install -y -qq -- "* ]]; then
      count="$(grep -c ' install -y -qq -- ' "$APT_TEST_LOG")"
      if ((count == 1)); then
        exit 100
      fi
    fi
    ;;
esac
exit 0
MOCK
  cat > "$APT_TEST_TEMP/sleep" <<'MOCK'
#!/usr/bin/env bash
printf 'backoff %s\n' "$*" >> "$APT_TEST_LOG"
MOCK
  chmod +x "$APT_TEST_TEMP/timeout" "$APT_TEST_TEMP/sudo" "$APT_TEST_TEMP/sleep"
}

teardown() {
  rm -rf -- "$APT_TEST_TEMP"
}

@test "mandatory APT dependencies recover from one failed index update" {
  run env PATH="$APT_TEST_TEMP:$PATH" APT_TEST_FAILURE=update_once \
    bash scripts/ci/install-apt-packages.sh musl-tools
  [ "$status" -eq 0 ]
  [ "$(grep -c ' update -qq' "$APT_TEST_LOG")" -eq 2 ]
  [ "$(grep -c ' install -y -qq -- ' "$APT_TEST_LOG")" -eq 1 ]
  grep -Fq 'PASS: mandatory APT dependencies installed' <<< "$output"
}

@test "mandatory APT dependencies recover from one failed package fetch" {
  run env PATH="$APT_TEST_TEMP:$PATH" APT_TEST_FAILURE=install_once \
    bash scripts/ci/install-apt-packages.sh bats
  [ "$status" -eq 0 ]
  [ "$(grep -c ' update -qq' "$APT_TEST_LOG")" -eq 2 ]
  [ "$(grep -c ' install -y -qq -- ' "$APT_TEST_LOG")" -eq 2 ]
}

@test "permanent APT failures remain fatal after three attempts" {
  run env PATH="$APT_TEST_TEMP:$PATH" APT_TEST_FAILURE=always \
    bash scripts/ci/install-apt-packages.sh musl-tools
  [ "$status" -ne 0 ]
  [ "$(grep -c ' update -qq' "$APT_TEST_LOG")" -eq 3 ]
  ! grep -Fq ' install -y -qq -- ' "$APT_TEST_LOG"
  [[ "$output" == *"ERROR: mandatory APT dependencies unavailable"* ]]
}

@test "APT installer requires at least one package" {
  run env PATH="$APT_TEST_TEMP:$PATH" bash scripts/ci/install-apt-packages.sh
  [ "$status" -eq 2 ]
  [[ "$output" == *"Usage: install-apt-packages.sh"* ]]
  [ ! -f "$APT_TEST_LOG" ]
}
