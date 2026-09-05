#!/usr/bin/env bash
set -euo pipefail

REPO="${GGOK_REPO:-jjdufu/ggok}"
BINDIR="${GGOK_BINDIR:-$HOME/.local/bin}"
DEST="${BINDIR}/ggok"
UPLOAD_DIR="/tmp/.ggok-uploads"
GGOK_TMP=""

cleanup() {
  if [[ -n "${GGOK_TMP:-}" ]]; then
    rm -rf "$GGOK_TMP"
    GGOK_TMP=""
  fi
}
trap cleanup EXIT

usage() {
  cat <<EOF
usage: $(basename "$0") [version] [--uninstall]

  (default)  download the latest ggok release for this OS/arch into:
             \$HOME/.local/bin/ggok
  version    install that release (with or without a leading v)
             example: $(basename "$0") 0.0.0
             piped:   curl .../install.sh | bash -s -- 0.0.0
  --version <ver>
             same as the positional version
  --uninstall
             stop ggok and delete its binary, config, logs, and cache
             does not delete ~/.grok or workspace files

Environment:
  GGOK_REPO     GitHub repo (default: jjdufu/ggok)
  GGOK_VERSION  release version without v (default: latest);
                overridden by the version argument
  GGOK_BINDIR   install directory (default: \$HOME/.local/bin)
EOF
}

die() { echo "$*" >&2; exit 1; }

need_home() {
  [[ -n "${HOME:-}" ]] || die "HOME is unset"
}

os_arch() {
  local sys mach
  sys="$(uname -s 2>/dev/null || true)"
  mach="$(uname -m 2>/dev/null || true)"
  case "$sys" in
    Darwin) OS="darwin" ;;
    Linux) OS="linux" ;;
    *) die "unsupported OS: ${sys:-unknown} (need macOS or Linux)" ;;
  esac
  case "$mach" in
    x86_64|amd64) ARCH="amd64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) die "unsupported arch: ${mach:-unknown}" ;;
  esac
}

config_dir() {
  if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
    printf '%s\n' "$XDG_CONFIG_HOME/ggok"
  else
    printf '%s\n' "$HOME/.config/ggok"
  fi
}

state_dir() {
  if [[ -n "${XDG_STATE_HOME:-}" ]]; then
    printf '%s\n' "$XDG_STATE_HOME/ggok"
  else
    printf '%s\n' "$HOME/.local/state/ggok"
  fi
}

is_ggok_leaf() {
  local base
  base="$(basename "$1")"
  [[ "$base" == "ggok" || "$base" == ".ggok-uploads" ]]
}

owned_by_me() {
  [[ -O "$1" ]]
}

remove_tree() {
  local path="$1"
  if [[ ! -e "$path" && ! -L "$path" ]]; then
    return 0
  fi
  if ! is_ggok_leaf "$path"; then
    echo "skip $path (unexpected name)" >&2
    LEFTOVER=1
    echo "$path" >> "$LEFTOVER_FILE"
    return 0
  fi
  if ! owned_by_me "$path"; then
    echo "left behind $path (not owned by $(id -un))" >&2
    LEFTOVER=1
    echo "$path" >> "$LEFTOVER_FILE"
    return 0
  fi
  rm -rf "$path"
  echo "removed $path"
}

remove_bin() {
  local path="$1"
  if [[ ! -e "$path" && ! -L "$path" ]]; then
    return 0
  fi
  if [[ "$(basename "$path")" != "ggok" ]]; then
    return 0
  fi
  if ! owned_by_me "$path"; then
    echo "left behind $path (not owned by $(id -un); try: sudo rm -f $(printf '%q' "$path"))" >&2
    LEFTOVER=1
    echo "$path" >> "$LEFTOVER_FILE"
    return 0
  fi
  rm -f "$path"
  echo "removed $path"
}

kill_pidfile() {
  local pidf="$1" pid
  [[ -f "$pidf" ]] || return 0
  pid="$(tr -d ' \t\n' < "$pidf" 2>/dev/null || true)"
  if [[ "$pid" =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null; then
    kill -TERM "$pid" 2>/dev/null || true
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
      kill -KILL "$pid" 2>/dev/null || true
    fi
  fi
}

stop_ggok() {
  if command -v ggok >/dev/null 2>&1; then
    ggok stop >/dev/null 2>&1 || true
  fi
  kill_pidfile "$(state_dir)/ggok.pid"
  kill_pidfile "$(state_dir)/grok-agent.pid"
}

do_uninstall() {
  need_home
  LEFTOVER=0
  LEFTOVER_FILE="$(mktemp)"

  stop_ggok
  remove_tree "$(config_dir)"
  remove_tree "$(state_dir)"
  remove_tree "$UPLOAD_DIR"

  remove_bin "$DEST"
  remove_bin "$HOME/.local/bin/ggok"
  remove_bin "/usr/local/bin/ggok"
  if command -v ggok >/dev/null 2>&1; then
    remove_bin "$(command -v ggok)"
  fi

  if [[ "$LEFTOVER" == 1 ]]; then
    echo "uninstalled with leftovers:" >&2
    sort -u "$LEFTOVER_FILE" >&2
    rm -f "$LEFTOVER_FILE"
    exit 1
  fi
  rm -f "$LEFTOVER_FILE"
  echo "uninstalled"
  echo "Grok CLI data under ~/.grok was not removed"
}

verify_sha256() {
  local sums="$1" file="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    grep -E "([ *])${file}\$" "$sums" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    grep -E "([ *])${file}\$" "$sums" | shasum -a 256 -c -
  else
    die "need sha256sum or shasum to verify the download"
  fi
}

resolve_version() {
  local raw="${CLI_VERSION:-${GGOK_VERSION:-}}"
  if [[ -n "$raw" ]]; then
    VERSION="${raw#v}"
    [[ -n "$VERSION" && "$VERSION" != "latest" ]] || die "invalid version: $raw"
    return 0
  fi
  local url
  url="$(curl -fsSL --retry 3 -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest")"
  VERSION="${url##*/}"
  VERSION="${VERSION#v}"
  [[ -n "$VERSION" && "$VERSION" != "latest" ]] || die "cannot resolve latest release from $url"
}

do_install() {
  need_home
  os_arch
  command -v curl >/dev/null 2>&1 || die "curl not found"
  command -v tar >/dev/null 2>&1 || die "tar not found"

  local VERSION asset base
  resolve_version
  asset="ggok_${VERSION}_${OS}_${ARCH}.tar.gz"
  base="https://github.com/${REPO}/releases/download/v${VERSION}"
  cleanup
  GGOK_TMP="$(mktemp -d)"

  echo "installing ggok ${VERSION} (${OS}/${ARCH})"
  echo "downloading ${base}/${asset}"
  curl -fsSL --retry 3 -o "${GGOK_TMP}/${asset}" "${base}/${asset}"
  curl -fsSL --retry 3 -o "${GGOK_TMP}/SHA256SUMS" "${base}/SHA256SUMS"
  (
    cd "$GGOK_TMP"
    verify_sha256 SHA256SUMS "$asset"
  )
  tar -xzf "${GGOK_TMP}/${asset}" -C "$GGOK_TMP"
  [[ -f "${GGOK_TMP}/ggok" ]] || die "archive missing ggok binary"

  mkdir -p "$BINDIR"
  if command -v install >/dev/null 2>&1; then
    install -m 755 "${GGOK_TMP}/ggok" "$DEST"
  else
    cp "${GGOK_TMP}/ggok" "$DEST"
    chmod 755 "$DEST"
  fi

  local cfgdir cfg
  cfgdir="$(config_dir)"
  cfg="${cfgdir}/config.toml"
  if [[ ! -f "$cfg" && -f "${GGOK_TMP}/config.toml" ]]; then
    mkdir -p "$cfgdir"
    cp "${GGOK_TMP}/config.toml" "$cfg"
    echo "wrote $cfg"
  fi

  echo "installed $DEST"
  case ":$PATH:" in
    *":$BINDIR:"*) ;;
    *) echo "note: $BINDIR is not on PATH; add it or run $DEST" ;;
  esac
  if ! command -v grok >/dev/null 2>&1; then
    echo "note: grok not found on PATH; ggok start needs Grok CLI or GGOK_GROK_BIN"
  fi
  echo "run: ggok start"
  echo "uninstall: ggok uninstall"
}

UNINSTALL=0
CLI_VERSION=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --uninstall|-u) UNINSTALL=1; shift ;;
    -h|--help) usage; exit 0 ;;
    --version)
      [[ $# -ge 2 ]] || die "--version needs an argument"
      [[ -z "$CLI_VERSION" ]] || die "version specified twice"
      CLI_VERSION="$2"
      shift 2
      ;;
    --version=*)
      [[ -z "$CLI_VERSION" ]] || die "version specified twice"
      CLI_VERSION="${1#--version=}"
      shift
      ;;
    -*)
      usage >&2
      die "unknown arg: $1"
      ;;
    *)
      [[ -z "$CLI_VERSION" ]] || die "version specified twice"
      CLI_VERSION="$1"
      shift
      ;;
  esac
done

if [[ "$UNINSTALL" == 1 ]]; then
  do_uninstall
else
  do_install
fi
