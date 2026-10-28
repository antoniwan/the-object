#!/usr/bin/env bash
# Runs after ./build.sh on Vercel. On a production build, if the project has BUTLER_API_KEY
# and ITCH_TARGET (user/game) set, it pushes the itch build to that page's html5 channel,
# so merging to main updates itch and the site together. Never fails the deploy.
set -uo pipefail
cd "$(dirname "$0")/.."

if [ "${VERCEL_ENV:-}" != production ] || [ -z "${BUTLER_API_KEY:-}" ] || [ -z "${ITCH_TARGET:-}" ]; then
  echo "itch-push: not a production build with BUTLER_API_KEY and ITCH_TARGET; skipping"
  exit 0
fi

tmp=$(mktemp -d)
# The short commit, as itch shows it on the upload and in `butler status`.
version=$(printf %s "${VERCEL_GIT_COMMIT_SHA:-unknown}" | cut -c1-7)
# Chained with && because set -e does nothing inside a || list.
./scripts/itch.sh --dir &&
  curl -sSfL -o "$tmp/butler.zip" https://broth.itch.zone/butler/linux-amd64/LATEST/archive/default &&
  { command -v unzip >/dev/null || command -v python3 >/dev/null || dnf install -y -q unzip; } &&
  { if command -v unzip >/dev/null; then unzip -q "$tmp/butler.zip" -d "$tmp"; else python3 -m zipfile -e "$tmp/butler.zip" "$tmp"; fi; } &&
  chmod +x "$tmp/butler" &&
  "$tmp/butler" push dist/itch "$ITCH_TARGET:html5" --userversion "$version" &&
  echo "itch-push: pushed $version to $ITCH_TARGET:html5" ||
  echo "itch-push: push to $ITCH_TARGET failed; the site deploys anyway" >&2
exit 0
