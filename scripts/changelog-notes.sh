#!/usr/bin/env bash
# Prints one version's entry from CHANGELOG.md, without its heading: ./scripts/changelog-notes.sh 1.2.2
set -euo pipefail
cd "$(dirname "$0")/.."
awk -v v="${1#v}" '
  /^## \[/ { if (on) exit; on = index($0, "## [" v "]") == 1; next }
  /^\[[^]]+\]: / { if (on) exit }
  on' CHANGELOG.md | sed -e '/./,$!d'
