#!/usr/bin/env bash

set -euo pipefail

IMAGE_NAME="${UBUNTU_IMAGE_NAME:-ubuntu-bazelisk}"
CONTAINERFILE_PATH="${UBUNTU_CONTAINERFILE:-$(dirname "$0")/Dockerfile}"
WORKSPACE_DIR="$(pwd)"
WORKSPACE_NAME="$(basename "$WORKSPACE_DIR")"
HOST_CACHE_DIR="${UBUNTU_HOST_CACHE_DIR:-$HOME/.cache/ubuntu-sh/$WORKSPACE_NAME}/cache"
HOST_HOME_DIR="${UBUNTU_HOST_HOME_DIR:-$HOME/.cache/ubuntu-sh/$WORKSPACE_NAME}/home"
CONTAINER_NAME="${UBUNTU_CONTAINER_NAME:-ubuntu-sh-$WORKSPACE_NAME}"
SSH_PORT="${UBUNTU_SSH_PORT:-2222}"

usage() {
  cat <<'EOF'
Usage: ./ubuntu.sh <command> [args...]

Runs the given command inside a persistent Ubuntu container with:
- the current directory mounted at /workspace
- the working directory set to /workspace
- a long-lived container reused across invocations

Environment overrides:
- UBUNTU_IMAGE_NAME: image tag to run (default: ubuntu-bazelisk)
- UBUNTU_CONTAINERFILE: Containerfile/Dockerfile path to build from if image is missing
- UBUNTU_HOST_CACHE_DIR: persistent host cache directory mounted at /cache
- UBUNTU_HOST_HOME_DIR: persistent host home directory mounted at /home/ubuntu
- UBUNTU_CONTAINER_NAME: container name to reuse
- UBUNTU_SSH_PORT: localhost port forwarded to container sshd (default: 2222)
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

mkdir -p "$HOST_CACHE_DIR" "$HOST_HOME_DIR"

if ! podman container exists "$CONTAINER_NAME"; then
  podman run -d \
    --name "$CONTAINER_NAME" \
    -p "127.0.0.1:${SSH_PORT}:22" \
    -v "$WORKSPACE_DIR:/workspace:Z" \
    -v "$HOST_CACHE_DIR:/cache:Z" \
    -v "$HOST_HOME_DIR:/home/ubuntu:Z" \
    -e "HOME=/home/ubuntu" \
    -e "XDG_CACHE_HOME=/cache/xdg" \
    -e "BAZELISK_HOME=/cache/bazelisk" \
    -w /workspace \
    "$IMAGE_NAME" \
    tail -f /dev/null >/dev/null
fi

if [[ "$(podman inspect -f '{{.State.Running}}' "$CONTAINER_NAME")" != "true" ]]; then
  podman start "$CONTAINER_NAME" >/dev/null
  podman exec \
    --workdir /workspace \
    "$CONTAINER_NAME" \
    bash -lc 'mkdir -p /home/ubuntu/.ssh /run/sshd && chmod 700 /home/ubuntu/.ssh && ssh-keygen -A >/dev/null 2>&1 && pgrep -x sshd >/dev/null || /usr/sbin/sshd' >/dev/null
fi

exec_args=()
if [[ -t 0 && -t 1 ]]; then
  exec_args+=(-it)
elif [[ -t 0 ]]; then
  exec_args+=(-i)
fi

exec podman exec "${exec_args[@]}" \
  --workdir /workspace \
  "$CONTAINER_NAME" \
  "$@"
