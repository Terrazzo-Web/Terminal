#!/usr/bin/env bash

set -euo pipefail

IMAGE_NAME="${UBUNTU_IMAGE_NAME:-ubuntu-bazelisk}"
CONTAINERFILE_PATH="${UBUNTU_CONTAINERFILE:-$(dirname "$0")/Dockerfile}"
WORKSPACE_DIR="$(pwd)"

usage() {
  cat <<'EOF'
Usage: ./ubuntu.sh <command> [args...]

Runs the given command inside an ephemeral Ubuntu container with:
- the current directory mounted at /workspace
- the working directory set to /workspace
- the local user UID/GID mapped into the container

Environment overrides:
- UBUNTU_IMAGE_NAME: image tag to run (default: ubuntu-bazelisk)
- UBUNTU_CONTAINERFILE: Containerfile/Dockerfile path to build from if image is missing
EOF
}

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

if ! podman image exists "$IMAGE_NAME"; then
  if [[ ! -f "$CONTAINERFILE_PATH" ]]; then
    echo "Image '$IMAGE_NAME' does not exist and no Dockerfile was found at '$CONTAINERFILE_PATH'." >&2
    exit 1
  fi

  echo "Building image '$IMAGE_NAME' from '$CONTAINERFILE_PATH'..." >&2
  podman build -t "$IMAGE_NAME" -f "$CONTAINERFILE_PATH" "$(dirname "$CONTAINERFILE_PATH")"
fi

exec podman run --rm \
  --userns keep-id \
  -v "$WORKSPACE_DIR:/workspace:Z" \
  -w /workspace \
  "$IMAGE_NAME" \
  "$@"
