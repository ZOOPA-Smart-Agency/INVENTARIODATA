#!/usr/bin/env bash
# Prints the version used in the build's file names: 0.1.0 for a tag v0.1.0, otherwise Cargo.toml's version and
# the run number (0.1.0-build.12).
set -euo pipefail
if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
  echo "${GITHUB_REF#refs/tags/v}"
else
  version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
  echo "${version}-build.${GITHUB_RUN_NUMBER:-0}"
fi
