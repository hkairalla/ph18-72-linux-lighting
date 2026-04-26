#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$REPO_ROOT/app"
DAEMON_DIR="$REPO_ROOT/daemon"
VENV_ACTIVATE="$APP_DIR/.venv/bin/activate"
BACKEND_MODE="${PH18_UI_BACKEND:-cargo}"

if [[ ! -f "$VENV_ACTIVATE" ]]; then
  echo "Missing app virtual environment at $VENV_ACTIVATE" >&2
  echo "Create it with:" >&2
  echo "  cd $APP_DIR && python3 -m venv .venv && source .venv/bin/activate && pip install -e ." >&2
  exit 1
fi

echo "[dev-ui] repo: $REPO_ROOT"
echo "[dev-ui] backend mode: $BACKEND_MODE"

if [[ "$BACKEND_MODE" == "cargo" ]]; then
  echo "[dev-ui] rebuilding daemon..."
  cargo build --manifest-path "$DAEMON_DIR/Cargo.toml"
fi

echo "[dev-ui] launching UI..."
cd "$APP_DIR"
source "$VENV_ACTIVATE"
PH18_UI_BACKEND="$BACKEND_MODE" python -m ph18_72_lighting_ui.main
