#!/bin/bash

set -u

REPOSITORY_ROOT="$(git rev-parse --show-cdup)"
[[ -n "$REPOSITORY_ROOT" ]] && cd "$REPOSITORY_ROOT"
REPOSITORY_ROOT="$(pwd)"
cd "$REPOSITORY_ROOT"

TARGET="${1}"

SUFFIX=""
case "$TARGET" in
  *-pc-windows-*) SUFFIX=".exe";;
esac

BIN_DIR="${REPOSITORY_ROOT}/release/"
mkdir -p "$BIN_DIR" || exit 1
BIN_NAME="surge${SUFFIX}"
BIN_PATH="${BIN_DIR}${BIN_NAME}"

cp "${REPOSITORY_ROOT}/target/${TARGET}/release/${BIN_NAME}" "$BIN_PATH" || exit 1

STRIP_COMMAND="strip"
case "$TARGET" in
  arm-unknown-linux-*) STRIP_COMMAND="arm-linux-gnueabihf-strip" ;;
  aarch64-unknown-linux-gnu) STRIP_COMMAND="aarch64-linux-gnu-strip" ;;
  *-pc-windows-msvc) exit 0;;
esac;

"$STRIP_COMMAND" "$BIN_PATH"

echo "${BIN_PATH}"
