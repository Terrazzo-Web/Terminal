#!/usr/bin/env bash

set -euo pipefail

runfiles_dir="${RUNFILES_DIR:-}"
if [[ -z "$runfiles_dir" && -d "$0.runfiles" ]]; then
  runfiles_dir="$0.runfiles"
fi

if [[ -n "$runfiles_dir" && -x "${runfiles_dir}/rules_rust++rust_host_tools+terminal_rust_host_tools/bin/rustc" ]]; then
  exec "${runfiles_dir}/rules_rust++rust_host_tools+terminal_rust_host_tools/bin/rustc" "$@"
fi

if [[ -n "$runfiles_dir" && -x "${runfiles_dir}/_main/external/rules_rust++rust_host_tools+terminal_rust_host_tools/bin/rustc" ]]; then
  exec "${runfiles_dir}/_main/external/rules_rust++rust_host_tools+terminal_rust_host_tools/bin/rustc" "$@"
fi

echo "could not locate rustc in Bazel runfiles" >&2
exit 1
