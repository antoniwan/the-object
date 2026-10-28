#!/usr/bin/env bash
# Moves the version everywhere it is written: ./scripts/version.sh 1.3.0
# Cargo.toml (the source), Cargo.lock, the page's meta tag and structured data, llms.txt,
# and CHANGELOG.md, where what is under [Unreleased] becomes the new version's entry.
# With no argument it prints the current version.
set -euo pipefail
cd "$(dirname "$0")/.."
old=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
new=${1:-}
[ -n "$new" ] || { echo "$old"; exit 0; }
[[ $new =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $new" >&2; exit 1; }
[ "$new" != "$old" ] || { echo "already $new" >&2; exit 1; }

sed -i "0,/^version = \"$old\"/s//version = \"$new\"/" Cargo.toml
sed -i "/^name = \"chaos-object\"/{n;s/^version = \"$old\"/version = \"$new\"/}" Cargo.lock
sed -i "s|<meta name=\"version\" content=\"$old\">|<meta name=\"version\" content=\"$new\">|; s|\"softwareVersion\": \"$old\"|\"softwareVersion\": \"$new\"|" web/index.html
sed -i "s|^Version $old\.|Version $new.|" web/llms.txt

repo=https://github.com/antoniwan/the-object
today=$(date -u +%F)
sed -i "s|^## \[Unreleased\]$|## [Unreleased]\n\n## [$new] - $today|" CHANGELOG.md
sed -i "s|^\[Unreleased\]: .*|[Unreleased]: $repo/compare/v$new...HEAD\n[$new]: $repo/compare/v$old...v$new|" CHANGELOG.md
echo "$old -> $new. Write what changed under ## [$new] in CHANGELOG.md, then cargo test."
