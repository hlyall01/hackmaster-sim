#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WIN_SCRIPT="$(wslpath -w "$SCRIPT_DIR/create_dev_codesign_cert.ps1")"

if [[ -z "${CODESIGN_PFX_PASSWORD-}" ]]; then
  echo "CODESIGN_PFX_PASSWORD is required." >&2
  exit 1
fi

pw_escaped=${CODESIGN_PFX_PASSWORD//\'/\'\'}
ps_cmd="\$env:CODESIGN_PFX_PASSWORD = '$pw_escaped'; & '$WIN_SCRIPT'"
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$ps_cmd"
