#!/usr/bin/env bash
# Signs files for the app's updater: sign.sh <file>... writes <file>.sig next to each. The app installs an update
# only when its signature matches the public key in tauri.conf.json (plugins > updater > pubkey).
# The private key is the repository secret TAURI_SIGNING_PRIVATE_KEY. Without it the files are not signed, which
# is fine for test builds; a release (a v* tag) then fails, since its update files could not be installed.
set -euo pipefail
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
    echo "::error::The repository secret TAURI_SIGNING_PRIVATE_KEY is missing: a release needs it to sign its update files."
    exit 1
  fi
  echo "No signing key: update files left unsigned (not a release)."
  exit 0
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
for file in "$@"; do
  # paths relative to the repository: a relative path also works for the Windows programs Git Bash starts
  [[ "$file" != /* && -f "$file" ]] || { echo "::error::$file is not a file in the repository"; exit 1; }
  (cd app && npx tauri signer sign "../$file" > /dev/null)
  test -s "$file.sig" || { echo "::error::No signature made for $file"; exit 1; }
  echo "Signed $file"
done
