#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "${SCRIPT_DIR}/../../.." && pwd)

# shellcheck disable=SC1091
. "${SCRIPT_DIR}/runtime.env"

revision=$(git -C "${REPOSITORY_ROOT}" rev-parse HEAD)
short_revision=$(git -C "${REPOSITORY_ROOT}" rev-parse --short=10 HEAD)
tag=${1:-develop}
output=${2:---load}

case "${output}" in
    --load|--push) ;;
    *) echo "usage: $0 [tag] [--load|--push]" >&2; exit 2 ;;
esac

docker buildx build \
    --file "${SCRIPT_DIR}/Dockerfile" \
    --build-arg "CODEX_REVISION=${revision}" \
    --build-arg "CODEX_VERSION=${CODEX_VERSION}" \
    --build-arg "CODEX_RUNTIME_RECIPE=${CODEX_RUNTIME_RECIPE}" \
    --build-arg "CODEX_RUST_IMAGE=${CODEX_RUST_IMAGE}" \
    --build-arg "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}" \
    --build-arg "CARGO_PROFILE_RELEASE_LTO=${CARGO_PROFILE_RELEASE_LTO}" \
    --build-arg "CARGO_PROFILE_RELEASE_CODEGEN_UNITS=${CARGO_PROFILE_RELEASE_CODEGEN_UNITS}" \
    --tag "${CODEX_RUNTIME_IMAGE}:${tag}" \
    --tag "${CODEX_RUNTIME_IMAGE}:${CODEX_VERSION}-${short_revision}-r${CODEX_RUNTIME_RECIPE}" \
    "${output}" \
    "${REPOSITORY_ROOT}"
