#!/usr/bin/env bash

set -euo pipefail

IMAGE_NAME="${UBUNTU_IMAGE_NAME:-ubuntu-bazelisk}"
CONTAINERFILE_PATH="${UBUNTU_CONTAINERFILE:-$(dirname "$0")/Dockerfile}"
WORKSPACE_DIR="$(pwd)"
WORKSPACE_NAME="$(basename "$WORKSPACE_DIR")"
HOST_CACHE_DIR="${UBUNTU_HOST_CACHE_DIR:-$HOME/.cache/ubuntu-sh/$WORKSPACE_NAME}/cache"
HOST_HOME_DIR="${UBUNTU_HOST_HOME_DIR:-$HOME/.cache/ubuntu-sh/$WORKSPACE_NAME}/home"
HOST_CONTROL_DIR="${UBUNTU_HOST_CONTROL_DIR:-$HOME/.cache/ubuntu-sh/$WORKSPACE_NAME}/control"
CONTAINER_NAME="${UBUNTU_CONTAINER_NAME:-ubuntu-sh-$WORKSPACE_NAME}"

usage() {
  cat <<'EOF'
Usage: ./ubuntu.sh <command> [args...]

Runs the given command through a file-backed command agent inside a persistent Ubuntu container with:
- the current directory mounted at /workspace
- the working directory set to /workspace
- a long-lived container reused across invocations
- no per-command podman exec after the agent is started

Environment overrides:
- UBUNTU_IMAGE_NAME: image tag to run (default: ubuntu-bazelisk)
- UBUNTU_CONTAINERFILE: Containerfile/Dockerfile path to build from if image is missing
- UBUNTU_HOST_CACHE_DIR: persistent host cache directory mounted at /cache
- UBUNTU_HOST_HOME_DIR: persistent host home directory mounted at /home/ubuntu
- UBUNTU_HOST_CONTROL_DIR: persistent host control directory mounted at /control
- UBUNTU_CONTAINER_NAME: container name to reuse
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

mkdir -p "$HOST_CACHE_DIR" "$HOST_HOME_DIR" "$HOST_CONTROL_DIR/requests"

ensure_agent=0

if podman container exists "$CONTAINER_NAME"; then
  if ! podman inspect -f '{{range .Mounts}}{{println .Destination}}{{end}}' "$CONTAINER_NAME" | grep -qx '/control'; then
    echo "Container '$CONTAINER_NAME' is missing the /control mount required by this version of ubuntu.sh." >&2
    echo "Recreate it with: podman rm -f $CONTAINER_NAME" >&2
    exit 1
  fi
fi

if ! podman container exists "$CONTAINER_NAME"; then
  podman run -d \
    --name "$CONTAINER_NAME" \
    -v "$WORKSPACE_DIR:/workspace:Z" \
    -v "$HOST_CACHE_DIR:/cache:Z" \
    -v "$HOST_HOME_DIR:/home/ubuntu:Z" \
    -v "$HOST_CONTROL_DIR:/control:Z" \
    -e "HOME=/home/ubuntu" \
    -e "XDG_CACHE_HOME=/cache/xdg" \
    -e "BAZELISK_HOME=/cache/bazelisk" \
    -w /workspace \
    "$IMAGE_NAME" \
    tail -f /dev/null >/dev/null
  ensure_agent=1
  echo "Starting command agent in '$CONTAINER_NAME'... 0"
fi

if [[ "$(podman inspect -f '{{.State.Running}}' "$CONTAINER_NAME")" != "true" ]]; then
  podman start "$CONTAINER_NAME" >/dev/null
  ensure_agent=1
  echo "Starting command agent in '$CONTAINER_NAME'... 1"
fi

if [[ "$ensure_agent" -eq 1 ]]; then
  echo "Starting command agent in '$CONTAINER_NAME'... 2"
  rm -f "$HOST_CONTROL_DIR/agent.ready" "$HOST_CONTROL_DIR/agent.pid"
  podman exec -d \
    --workdir /workspace \
    "$CONTAINER_NAME" \
    bash -lc '
      mkdir -p /control/requests
      echo $$ > /control/agent.pid
      touch /control/agent.ready
      while true; do
        handled=0
        for marker in /control/requests/*.request; do
          if [[ ! -e "$marker" ]]; then
            break
          fi
          handled=1
          req_dir="${marker%.request}"
          lock="${req_dir}.running"
          if ! mv "$marker" "$lock" 2>/dev/null; then
            continue
          fi
          bash -lc "$(cat "$req_dir/cmd")" >"$req_dir/output" 2>&1
          status=$?
          printf "%s\n" "$status" >"$req_dir/exit_code"
          touch "$req_dir/done"
          rm -f "$lock"
        done
        if [[ "$handled" -eq 0 ]]; then
          sleep 0.1
        fi
      done
    ' >/dev/null
fi

for _ in $(seq 1 50); do
  if [[ -f "$HOST_CONTROL_DIR/agent.ready" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -f "$HOST_CONTROL_DIR/agent.ready" ]]; then
  echo "Command agent did not become ready in $CONTAINER_NAME." >&2
  exit 1
fi

request_id="$(date +%s%N)-$$"
request_dir="$HOST_CONTROL_DIR/requests/$request_id"
request_marker="${request_dir}.request"

mkdir -p "$request_dir"
printf -v command_line '%q ' "$@"
printf '%s\n' "$command_line" > "$request_dir/cmd"
touch "$request_marker"

for _ in $(seq 1 36000); do
  if [[ -f "$request_dir/done" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -f "$request_dir/done" ]]; then
  echo "Timed out waiting for command to finish in $CONTAINER_NAME." >&2
  exit 1
fi

if [[ -f "$request_dir/output" ]]; then
  cat "$request_dir/output"
fi

if [[ -f "$request_dir/exit_code" ]]; then
  read -r exit_code < "$request_dir/exit_code"
else
  exit_code=1
fi

rm -rf "$request_dir" "$request_marker" "${request_dir}.running"
exit "$exit_code"
