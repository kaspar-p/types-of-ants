#!/bin/bash

# Migrate legacy flat ant-fs files into the per-user namespace.
#
# Before namespacing, files lived flat at $ROOT/<name>. The server now stores
# them at $ROOT/<username>/<hash-of-path>. Rather than recompute that layout
# here, this re-uploads each flat file through the API so the SERVER decides
# where it goes -- the placement logic stays in exactly one place.
#
# Run on each ant-fs node, pointed at that node's own API, since every replica
# has its own $ROOT.
#
# Usage:
#   migrate-namespace.sh <api-base-url> <root-dir> [--delete]
#
#   <api-base-url>  e.g. http://localhost:3237  (or https://localhost:443)
#   <root-dir>      the ant-fs persist dir (ANT_FS_ROOT_PATH, e.g. ./fs)
#   --delete        remove each flat original after a successful re-upload
#
# Credentials are read from $TYPESOFANTS_SECRET_DIR/ant_fs_client_creds.secret
# (first line, "username:password"). The server namespaces by that username.

set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <api-base-url> <root-dir> [--delete]" >&2
  exit 1
fi

api="$1"
root="$2"
delete_originals="false"
if [ "${3:-}" = "--delete" ]; then
  delete_originals="true"
fi

secret_dir="${TYPESOFANTS_SECRET_DIR:?TYPESOFANTS_SECRET_DIR not set}"
creds="$(head -n 1 "$secret_dir/ant_fs_client_creds.secret")"
users_file="$secret_dir/ant_fs_users.secret"

# Preflight: a per-user namespace directory is created at $ROOT/<username> on
# first write. A legacy flat file with the same name as a username would clash
# with that directory (a path can't be both a file and a dir), and the upload
# that creates the dir would fail. Cross-check every username in the allowlist
# against the flat files and abort up front if any collide.
conflicts="false"
while IFS= read -r line || [ -n "$line" ]; do
  [ -n "$line" ] || continue
  username="${line%%:*}"
  if [ -f "$root/$username" ]; then
    echo "CONFLICT: flat file '$root/$username' collides with namespace dir for user '$username'" >&2
    conflicts="true"
  fi
done < "$users_file"

if [ "$conflicts" = "true" ]; then
  echo "Aborting: rename or hand-migrate the conflicting file(s) first." >&2
  exit 1
fi

for f in "$root"/*; do
  # Only top-level files are legacy. Per-user namespace directories are skipped.
  [ -f "$f" ] || continue

  name="$(basename "$f")"
  echo "Migrating ${name}..."

  # -T streams the file (no buffering) and defaults the method to PUT. The
  # server hashes "$name" and writes it under the authenticated username.
  curl -fsS --user "$creds" -T "$f" "${api}/${name}"

  if [ "$delete_originals" = "true" ]; then
    rm "$f"
  fi
done

echo "Done."
