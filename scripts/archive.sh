#!/bin/bash

set -u

TARGET="${1}"
VERSION="${2}"
BIN_PATH="${3}"

SUFFIX=".tar.gz"
case "$TARGET" in
  windows) SUFFIX=".zip";;
esac

BASE="surge-${VERSION}-${TARGET}"
ARCHIVE_NAME="${BASE}${SUFFIX}"

REPOSITORY_ROOT="$(git rev-parse --show-cdup)"
[[ -n "$REPOSITORY_ROOT" ]] && cd "$REPOSITORY_ROOT"
REPOSITORY_ROOT="$(pwd)"
cd "$REPOSITORY_ROOT"

ARCHIVE_DIR="${REPOSITORY_ROOT}/archive/${BASE}"
mkdir -p "$ARCHIVE_DIR" || exit 1
mkdir -p "${ARCHIVE_DIR}/completion" || exit 1

cp "${BIN_PATH}" "${ARCHIVE_DIR}" || exit 1

cat << EOF | while read F; do cp "$F" "${ARCHIVE_DIR}/" || exit 1; done
README.md
LICENSE
LICENSE-APACHE
EOF

"$BIN_PATH" --completion zsh - > "${ARCHIVE_DIR}/completion/_surge"
"$BIN_PATH" --completion fish - > "${ARCHIVE_DIR}/completion/surge.fish"
"$BIN_PATH" --completion bash - > "${ARCHIVE_DIR}/completion/surge.bash"
"$BIN_PATH" --completion powershell - > "${ARCHIVE_DIR}/completion/_surge.ps1"

cd "${REPOSITORY_ROOT}/archive"
case "$TARGET" in
  windows) 7z -y a "$ARCHIVE_NAME" "${BASE}"/* | tail -2;;
  *) tar czf "$ARCHIVE_NAME" "${BASE}"/*;;
esac

echo "${PWD}/${ARCHIVE_NAME}"
