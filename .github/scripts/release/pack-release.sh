#!/usr/bin/env bash
set -euo pipefail

NAME=""
VERSION=""
OS=""
ARCH=""
OUTDIR="dist/release"
BIN=""
CONFIG=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --name) NAME="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    --os) OS="$2"; shift 2 ;;
    --arch) ARCH="$2"; shift 2 ;;
    --outdir) OUTDIR="$2"; shift 2 ;;
    --bin) BIN="$2"; shift 2 ;;
    --config) CONFIG="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$NAME" || -z "$VERSION" || -z "$OS" || -z "$ARCH" || -z "$BIN" ]]; then
  echo "missing required args" >&2
  exit 1
fi
if [[ ! -f "$BIN" ]]; then
  echo "binary not found: $BIN" >&2
  exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

DEST_BIN="$NAME"
cp "$BIN" "${STAGE}/${DEST_BIN}"
chmod +x "${STAGE}/${DEST_BIN}"

if [[ -n "$CONFIG" ]]; then
  if [[ ! -f "$CONFIG" ]]; then
    echo "config not found: $CONFIG" >&2
    exit 1
  fi
  cp "$CONFIG" "${STAGE}/$(basename "$CONFIG")"
fi

mkdir -p "$OUTDIR"
ARCHIVE="${OUTDIR}/${NAME}_${VERSION}_${OS}_${ARCH}.tar.gz"
tar -C "$STAGE" -czf "$ARCHIVE" .
echo "wrote ${ARCHIVE}"
ls -lh "$ARCHIVE"
