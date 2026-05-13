#!/usr/bin/env bash
#
# Functional test suite for fnpm supply-chain protections.
#
# Covers, for each available package manager (npm / yarn / pnpm / bun):
#   1. Audit of the malicious test fixtures in ../test-malicious-packages
#   2. `fnpm install` runs with `--ignore-scripts` (no postinstall side-effects)
#   3. `block_exotic_subdeps` rejects git/url/file/github specifiers
#   4. `minimum_release_age` blocks recent versions and allows old ones
#   5. `allow_builds` allow-list triggers manual rebuild
#   6. Fresh-clone fallback works without `.fnpm/config.json`
#
# Each case prints PASS/FAIL and the suite exits non-zero on any failure.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MALICIOUS_DIR="$(cd "$REPO_ROOT/../test-malicious-packages" 2>/dev/null && pwd || echo "")"
FNPM_BIN="$REPO_ROOT/target/debug/fnpm"

PASS=0
FAIL=0
FAILED_CASES=()

# ----------------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------------

c_red()    { printf '\033[31m%s\033[0m' "$*"; }
c_green()  { printf '\033[32m%s\033[0m' "$*"; }
c_yellow() { printf '\033[33m%s\033[0m' "$*"; }
c_bold()   { printf '\033[1m%s\033[0m' "$*"; }

log_header() {
  echo
  echo "============================================================"
  c_bold "$1"; echo
  echo "============================================================"
}

log_case() {
  printf '  [%s] %s\n' "$(c_yellow RUN)" "$1"
}

pass() {
  PASS=$((PASS + 1))
  printf '  [%s] %s\n' "$(c_green PASS)" "$1"
}

fail() {
  FAIL=$((FAIL + 1))
  FAILED_CASES+=("$1")
  printf '  [%s] %s\n' "$(c_red FAIL)" "$1"
  if [ -n "${2-}" ]; then
    echo "       reason: $2"
  fi
}

assert_contains() {
  # assert_contains <label> <haystack> <needle>
  if echo "$2" | grep -qF "$3"; then
    pass "$1"
  else
    fail "$1" "expected to find '$3'"
  fi
}

assert_not_contains() {
  if echo "$2" | grep -qF "$3"; then
    fail "$1" "did NOT expect to find '$3'"
  else
    pass "$1"
  fi
}

assert_exit_nonzero() {
  # assert_exit_nonzero <label> <exit_code>
  if [ "$2" -ne 0 ]; then
    pass "$1"
  else
    fail "$1" "expected non-zero exit, got 0"
  fi
}

assert_exit_zero() {
  if [ "$2" -eq 0 ]; then
    pass "$1"
  else
    fail "$1" "expected exit 0, got $2"
  fi
}

have_cmd() { command -v "$1" >/dev/null 2>&1; }

# ----------------------------------------------------------------------------
# Setup
# ----------------------------------------------------------------------------

if [ ! -x "$FNPM_BIN" ]; then
  echo "fnpm binary not found at $FNPM_BIN — building..."
  (cd "$REPO_ROOT" && cargo build --bin fnpm) || {
    echo "Build failed"; exit 2;
  }
fi

if [ -z "$MALICIOUS_DIR" ] || [ ! -d "$MALICIOUS_DIR" ]; then
  echo "$(c_red "ERROR"): ../test-malicious-packages not found relative to $REPO_ROOT"
  exit 2
fi

WORK_ROOT="$(mktemp -d -t fnpm-functional.XXXXXX)"
trap 'rm -rf "$WORK_ROOT"' EXIT
echo "Sandbox: $WORK_ROOT"

# Detect which package managers are available locally.
AVAILABLE_PMS=()
for pm in npm yarn pnpm bun; do
  if have_cmd "$pm"; then
    AVAILABLE_PMS+=("$pm")
  fi
done
echo "Detected package managers: ${AVAILABLE_PMS[*]:-none}"

write_fnpm_config() {
  # write_fnpm_config <dir> <pm> [min_age] [block_exotic] [allow_builds_json]
  local dir="$1" pm="$2"
  local min_age="${3-0}"
  local block_exotic="${4-true}"
  local allow_builds="${5-[]}"
  mkdir -p "$dir/.fnpm"
  cat > "$dir/.fnpm/config.json" <<EOF
{
  "package_manager": "$pm",
  "global_cache_path": "$HOME/.local/share/.fnpm/cache",
  "security_audit": true,
  "transitive_scan_depth": 0,
  "minimum_release_age_minutes": $min_age,
  "block_exotic_subdeps": $block_exotic,
  "allow_builds": $allow_builds
}
EOF
}

# ----------------------------------------------------------------------------
# 1. Audit the malicious fixtures via `fnpm scan`
# ----------------------------------------------------------------------------

log_header "1. Audit malicious fixtures (fnpm scan against local packages)"

# `fnpm scan` reads installed deps from cwd's node_modules. We synthesise a fake
# project that "installs" each fixture by symlinking it into node_modules, then
# scan. Each fixture is expected to surface in the scan output.

audit_fixture() {
  local fixture="$1" expected_signal="$2"
  local sandbox="$WORK_ROOT/audit-$(basename "$fixture")"
  mkdir -p "$sandbox/node_modules"

  # Build a minimal package.json that references the fixture by name.
  local fixture_name
  fixture_name=$(node -p "require('$fixture/package.json').name" 2>/dev/null || basename "$fixture")
  cat > "$sandbox/package.json" <<EOF
{
  "name": "audit-host",
  "version": "0.0.1",
  "dependencies": { "$fixture_name": "*" }
}
EOF
  cp -R "$fixture" "$sandbox/node_modules/$fixture_name"
  write_fnpm_config "$sandbox" "npm"

  log_case "audit fixture $(basename "$fixture")"
  local out
  out=$(cd "$sandbox" && "$FNPM_BIN" scan 2>&1 || true)
  assert_contains "scan flags $(basename "$fixture")" "$out" "$expected_signal"
}

audit_fixture "$MALICIOUS_DIR/critical-package"          "CRITICAL"
audit_fixture "$MALICIOUS_DIR/high-risk-package"         "curl"
audit_fixture "$MALICIOUS_DIR/medium-risk-package"       "curl"
audit_fixture "$MALICIOUS_DIR/obfuscated-malware-package" "curl"

# ----------------------------------------------------------------------------
# 2. `fnpm install` adds --ignore-scripts (no malicious script ever runs)
# ----------------------------------------------------------------------------

log_header "2. fnpm install applies --ignore-scripts per package manager"

run_install_no_scripts() {
  local pm="$1"
  local sandbox="$WORK_ROOT/install-$pm"
  mkdir -p "$sandbox"

  # Sentinel file that a postinstall would touch; if it appears, scripts ran.
  cat > "$sandbox/package.json" <<EOF
{
  "name": "scripts-canary",
  "version": "0.0.1",
  "scripts": {
    "preinstall":  "touch $sandbox/PRE_RAN",
    "install":     "touch $sandbox/INS_RAN",
    "postinstall": "touch $sandbox/POST_RAN"
  }
}
EOF
  write_fnpm_config "$sandbox" "$pm"

  log_case "[$pm] fnpm install does not run lifecycle scripts"
  local out
  out=$(cd "$sandbox" && "$FNPM_BIN" install 2>&1 || true)

  local canary_hit=0
  for f in PRE_RAN INS_RAN POST_RAN; do
    [ -e "$sandbox/$f" ] && canary_hit=1
  done

  if [ "$canary_hit" -eq 0 ]; then
    pass "[$pm] lifecycle scripts blocked"
  else
    fail "[$pm] lifecycle scripts blocked" "sentinel file created"
  fi
  assert_contains "[$pm] warning printed" "$out" "lifecycle scripts"
}

for pm in "${AVAILABLE_PMS[@]}"; do
  run_install_no_scripts "$pm"
done

# ----------------------------------------------------------------------------
# 3. block_exotic_subdeps rejects exotic specifiers
# ----------------------------------------------------------------------------

log_header "3. block_exotic_subdeps rejects git/url/file/github specs"

run_block_exotic() {
  local pm="$1" spec="$2" label="$3"
  local sandbox="$WORK_ROOT/exotic-$pm-$label"
  mkdir -p "$sandbox"
  cat > "$sandbox/package.json" <<EOF
{
  "name": "exotic-host",
  "version": "0.0.1",
  "dependencies": { "evil": "$spec" }
}
EOF
  write_fnpm_config "$sandbox" "$pm"

  log_case "[$pm] block exotic spec '$spec'"
  local out rc
  out=$(cd "$sandbox" && "$FNPM_BIN" install 2>&1)
  rc=$?
  assert_exit_nonzero "[$pm] install rejected ($label)" "$rc"
  assert_contains    "[$pm] reports exotic ($label)"   "$out" "block_exotic_subdeps"
}

for pm in "${AVAILABLE_PMS[@]}"; do
  run_block_exotic "$pm" "git+https://github.com/foo/bar.git" "git"
  run_block_exotic "$pm" "https://example.com/foo.tgz"        "https"
  run_block_exotic "$pm" "github:foo/bar"                     "github"
  run_block_exotic "$pm" "file:../local"                      "file"
done

# Sanity: a normal semver spec must NOT be flagged as exotic. We avoid actually
# hitting the network by setting an impossibly high min_release_age so the
# install bails out *after* the exotic check passes. Network may still be
# touched for the age check; we accept either outcome here as long as the
# failure isn't the exotic gate.

run_semver_not_exotic() {
  local pm="$1"
  local sandbox="$WORK_ROOT/exotic-clean-$pm"
  mkdir -p "$sandbox"
  cat > "$sandbox/package.json" <<EOF
{
  "name": "clean-host",
  "version": "0.0.1",
  "dependencies": { "lodash": "^4.17.21" }
}
EOF
  write_fnpm_config "$sandbox" "$pm"

  log_case "[$pm] semver spec is not flagged as exotic"
  local out
  out=$(cd "$sandbox" && "$FNPM_BIN" install 2>&1 || true)
  assert_not_contains "[$pm] semver passes exotic gate" "$out" "block_exotic_subdeps. Pin"
}

for pm in "${AVAILABLE_PMS[@]}"; do
  run_semver_not_exotic "$pm"
done

# ----------------------------------------------------------------------------
# 4. minimum_release_age behaviour (network-dependent, npm registry)
# ----------------------------------------------------------------------------

log_header "4. minimum_release_age age-gate for explicit add"

if ! have_cmd curl; then
  echo "$(c_yellow SKIP): curl not available; skipping registry-backed age tests"
else
  # We test the `add` flow: add lodash@4.17.21 (a release that is years old —
  # always older than 99999999 minutes is impossible, but it's older than 1 min).
  # Use min_age = 99999999999 to force a violation regardless of the version's age.
  for pm in "${AVAILABLE_PMS[@]}"; do
    sandbox="$WORK_ROOT/age-block-$pm"
    mkdir -p "$sandbox"
    cat > "$sandbox/package.json" <<EOF
{ "name": "age-host", "version": "0.0.1" }
EOF
    write_fnpm_config "$sandbox" "$pm" 99999999999

    log_case "[$pm] add lodash@4.17.21 is blocked by huge min_age"
    out=$(cd "$sandbox" && "$FNPM_BIN" add --no-audit lodash@4.17.21 2>&1)
    rc=$?
    assert_exit_nonzero "[$pm] add rejected by age gate" "$rc"
    assert_contains    "[$pm] error mentions minimum_release_age" "$out" "minimum_release_age"
  done

  # Also verify min_age = 0 disables the gate (no age error appears).
  for pm in "${AVAILABLE_PMS[@]}"; do
    sandbox="$WORK_ROOT/age-allow-$pm"
    mkdir -p "$sandbox"
    cat > "$sandbox/package.json" <<EOF
{ "name": "age-host", "version": "0.0.1" }
EOF
    write_fnpm_config "$sandbox" "$pm"  # default min_age=0

    log_case "[$pm] min_age=0 disables age gate"
    out=$(cd "$sandbox" && "$FNPM_BIN" add --no-audit lodash@4.17.21 2>&1 || true)
    assert_not_contains "[$pm] no age violation message" "$out" "minimum_release_age = "
  done
fi

# ----------------------------------------------------------------------------
# 5. allow_builds allow-list triggers rebuild
# ----------------------------------------------------------------------------

log_header "5. allow_builds triggers a manual rebuild step"

run_allow_builds_banner() {
  local pm="$1"
  local sandbox="$WORK_ROOT/allow-$pm"
  mkdir -p "$sandbox"
  cat > "$sandbox/package.json" <<EOF
{ "name": "allow-host", "version": "0.0.1" }
EOF
  write_fnpm_config "$sandbox" "$pm" 0 true '["sharp"]'

  log_case "[$pm] allow_builds entry surfaces in banner"
  local out
  out=$(cd "$sandbox" && "$FNPM_BIN" install 2>&1 || true)
  assert_contains "[$pm] banner lists allow_builds" "$out" "allow_builds"
  assert_contains "[$pm] running build scripts message" "$out" "running build scripts"
}

for pm in "${AVAILABLE_PMS[@]}"; do
  run_allow_builds_banner "$pm"
done

# ----------------------------------------------------------------------------
# 6. Fresh-clone fallback: no .fnpm/config.json → defaults still apply
# ----------------------------------------------------------------------------

log_header "6. Fresh-clone fallback applies defaults"

run_fresh_clone() {
  local pm="$1"
  local sandbox="$WORK_ROOT/fresh-$pm"
  mkdir -p "$sandbox"
  cat > "$sandbox/package.json" <<EOF
{
  "name": "fresh-canary",
  "version": "0.0.1",
  "scripts": { "postinstall": "touch $sandbox/RAN" }
}
EOF
  # NOTE: deliberately no .fnpm/config.json

  log_case "[$pm] fnpm install on fresh clone uses defaults"
  local out
  out=$(cd "$sandbox" && PM_OVERRIDE=$pm "$FNPM_BIN" install 2>&1 || true)
  assert_contains "[$pm] fresh-clone notice printed" "$out" "no .fnpm/config.json found"

  if [ ! -e "$sandbox/RAN" ]; then
    pass "[$pm] fresh-clone install did not run postinstall"
  else
    fail "[$pm] fresh-clone install did not run postinstall" "RAN file created"
  fi
}

# Only run once (fnpm detects the manager from lockfile / falls back to npm).
run_fresh_clone "npm"

# ----------------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------------

echo
echo "============================================================"
c_bold "Summary: "; printf '%s passed, %s failed\n' "$(c_green "$PASS")" "$(c_red "$FAIL")"
echo "============================================================"

if [ "$FAIL" -ne 0 ]; then
  echo "Failed cases:"
  for c in "${FAILED_CASES[@]}"; do
    echo "  - $c"
  done
  exit 1
fi
exit 0
