#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

jcode_home="${JCODE_HOME:-$HOME/.jcode}"
stable="$jcode_home/builds/stable/jcode"
state="$jcode_home/token-optimizer"
kind_file="$state/previous-kind"
[ -f "$kind_file" ] || { echo 'No Token Optimizer rollback state found.' >&2; exit 1; }
kind="$(cat "$kind_file")"
case "$kind" in
  symlink)
    [ -f "$state/previous-target" ] || { echo 'Saved original symlink target is missing.' >&2; exit 1; }
    target="$(cat "$state/previous-target")"
    [ -e "$target" ] || { echo "Original binary target no longer exists: $target" >&2; exit 1; }
    mkdir -p "$(dirname "$stable")"
    ln -sfn "$target" "$stable"
    echo "Restored original JCode target: $target"
    ;;
  file)
    [ -f "$state/previous-binary" ] || { echo 'Saved original binary is missing.' >&2; exit 1; }
    mkdir -p "$(dirname "$stable")"
    cp -p "$state/previous-binary" "$stable"
    chmod 755 "$stable"
    echo "Restored original JCode binary: $stable"
    ;;
  missing)
    rm -f "$stable"
    echo 'Removed optimized stable link; no previous binary was found.'
    ;;
  *)
    echo "Unknown rollback state: $kind" >&2
    exit 1
    ;;
esac
