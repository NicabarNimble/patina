#!/usr/bin/env bash
set -euo pipefail

REPO="${PATINA_REPO:-NicabarNimble/patina}"
CHANNEL="stable"
VERSION=""
BIN_DIR=""
NO_VERIFY=0

usage() {
  cat <<'EOF'
Patina installer (GitHub Releases)

Usage:
  install.sh [options]

Options:
  --channel <stable|beta|nightly>  Release channel (default: stable)
  --version <vX.Y.Z|X.Y.Z>         Pin explicit version (overrides channel)
  --bin-dir <path>                 Install directory for `patina`
  --no-verify                      Skip checksum verification
  --repo <owner/repo>              Override release repo (default: NicabarNimble/patina)
  -h, --help                       Show this help

Examples:
  curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash -s -- --channel beta
  curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash -s -- --version 0.62.1
EOF
}

fail() {
  echo "[install] error: $*" >&2
  exit 1
}

log() {
  echo "[install] $*"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

resolve_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${os}:${arch}" in
    darwin:arm64|darwin:aarch64)
      echo "aarch64-apple-darwin"
      ;;
    darwin:x86_64)
      echo "x86_64-apple-darwin"
      ;;
    linux:x86_64)
      echo "x86_64-unknown-linux-gnu"
      ;;
    *)
      fail "unsupported platform ${os}/${arch}; please install from source"
      ;;
  esac
}

resolve_default_bin_dir() {
  if [ -n "${BIN_DIR}" ]; then
    echo "${BIN_DIR}"
    return
  fi

  if [ "$(id -u)" -eq 0 ]; then
    echo "/usr/local/bin"
  elif [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
  else
    echo "${HOME}/.local/bin"
  fi
}

latest_stable_tag() {
  require_cmd curl
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' \
    | head -n 1
}

resolve_tag() {
  if [ -n "${VERSION}" ]; then
    if [[ "${VERSION}" == v* ]]; then
      echo "${VERSION}"
    else
      echo "v${VERSION}"
    fi
    return
  fi

  case "${CHANNEL}" in
    stable)
      latest_stable_tag
      ;;
    beta)
      # Expect a moving pre-release tag named `beta`.
      echo "beta"
      ;;
    nightly)
      # Expect a moving pre-release tag named `nightly`.
      echo "nightly"
      ;;
    *)
      fail "unknown channel '${CHANNEL}'"
      ;;
  esac
}

verify_checksum() {
  local checksums_file asset_name
  checksums_file="$1"
  asset_name="$2"

  local line
  line="$(grep " ${asset_name}$" "${checksums_file}" || true)"
  [ -n "${line}" ] || fail "checksum for ${asset_name} not found"

  if command -v sha256sum >/dev/null 2>&1; then
    echo "${line}" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    local expected actual
    expected="$(echo "${line}" | awk '{print $1}')"
    actual="$(shasum -a 256 "${asset_name}" | awk '{print $1}')"
    [ "${expected}" = "${actual}" ] || fail "checksum mismatch for ${asset_name}"
  else
    fail "neither sha256sum nor shasum found for checksum verification"
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --channel)
      [ "$#" -ge 2 ] || fail "--channel requires a value"
      CHANNEL="$2"
      shift 2
      ;;
    --version)
      [ "$#" -ge 2 ] || fail "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --bin-dir)
      [ "$#" -ge 2 ] || fail "--bin-dir requires a value"
      BIN_DIR="$2"
      shift 2
      ;;
    --repo)
      [ "$#" -ge 2 ] || fail "--repo requires a value"
      REPO="$2"
      shift 2
      ;;
    --no-verify)
      NO_VERIFY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

require_cmd curl
require_cmd tar
require_cmd mktemp

TARGET="$(resolve_target)"
TAG="$(resolve_tag)"
[ -n "${TAG}" ] || fail "failed to resolve release tag"

ASSET="patina-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
BIN_DIR="$(resolve_default_bin_dir)"

log "repo: ${REPO}"
log "channel: ${CHANNEL}"
log "tag: ${TAG}"
log "target: ${TARGET}"
log "asset: ${ASSET}"
log "bin dir: ${BIN_DIR}"

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

cd "${TMP_DIR}"

log "downloading ${ASSET}"
curl -fL "${BASE_URL}/${ASSET}" -o "${ASSET}" \
  || fail "failed to download ${ASSET} from ${BASE_URL}"

if [ "${NO_VERIFY}" -eq 0 ]; then
  log "downloading checksums"
  curl -fL "${BASE_URL}/checksums.txt" -o checksums.txt \
    || fail "failed to download checksums.txt from ${BASE_URL}"
  log "verifying checksum"
  verify_checksum checksums.txt "${ASSET}"
else
  log "checksum verification skipped (--no-verify)"
fi

log "extracting"
tar -xzf "${ASSET}" || fail "failed to extract ${ASSET}"
[ -f "patina" ] || fail "archive does not contain 'patina' binary"

mkdir -p "${BIN_DIR}"
install -m 0755 "patina" "${BIN_DIR}/patina"

log "installed: ${BIN_DIR}/patina"
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
  echo "[install] warning: ${BIN_DIR} is not on PATH"
fi

"${BIN_DIR}/patina" --version || true
log "done"
