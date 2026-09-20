#!/usr/bin/env bash
set -euo pipefail

mkdir -p screenshots
rm -f screenshots/latest.png

cargo run --bin autobattler -- \
  --headless-screenshots \
  --auto-screenshots \
  --auto-start-run \
  --auto-screenshot-count 1 \
  --auto-screenshot-interval 0.5

if [ ! -f screenshots/latest.png ]; then
  echo "Expected screenshots/latest.png was not created." >&2
  exit 1
fi

echo "Saved screenshots/latest.png"
