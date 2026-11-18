#!/usr/bin/env bash
# Creates a fresh copy of this repo for one demo run, e.g.: tools/new_run.sh lucid-telemetry-dryrun
# The copy has the clean starting state (main of maxreith/lucid-telemetry-demo) and no PRs.
set -euo pipefail
name=${1:?usage: tools/new_run.sh <repo-name>}
owner=${OWNER:-maxreith}
tmp=$(mktemp -d)
git clone --quiet --bare "https://github.com/$owner/lucid-telemetry-demo.git" "$tmp/repo.git"
gh repo create "$owner/$name" --private --description "Lucid telemetry migration demo run"
git -C "$tmp/repo.git" push --quiet --mirror "https://github.com/$owner/$name.git"
rm -rf "$tmp"
echo "created https://github.com/$owner/$name"
