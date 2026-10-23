#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[ -n "${PREFIX:-}" ] || fail 'Run this script from Termux, not a desktop Linux shell.'
[ "$(uname -m)" = 'aarch64' ] || fail "Expected Android aarch64; got $(uname -m)."
command -v patchelf >/dev/null 2>&1 || fail "Install patchelf first: pkg install patchelf"
command -v sha256sum >/dev/null 2>&1 || fail 'sha256sum is required.'

source_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source_bin="$source_dir/jcode"
[ -f "$source_bin" ] || fail "Binary not found: $source_bin. Extract the build archive first."
[ -f "$source_dir/SHA256SUMS" ] || fail "SHA256SUMS not found beside $source_bin."
(
  cd "$source_dir"
  sha256sum -c SHA256SUMS
) || fail 'Binary checksum did not match.'

linker="$PREFIX/glibc/lib/ld-linux-aarch64.so.1"
[ -x "$linker" ] || fail "glibc loader not found: $linker. Install glibc with pkg first."

jcode_home="${JCODE_HOME:-$HOME/.jcode}"
builds="$jcode_home/builds"
versions="$builds/versions"
stable="$builds/stable/jcode"
state="$jcode_home/token-optimizer"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
dest="$versions/0.99.1-token-optimizer-$stamp"
mkdir -p "$dest/lib" "$builds/stable" "$state"

cp "$source_bin" "$dest/jcode"
chmod 755 "$dest/jcode"
if [ -d "$source_dir/lib" ]; then
  cp -a "$source_dir/lib/." "$dest/lib/"
fi

# The Termux glibc package supplies the interpreter; bundled non-glibc libraries
# are resolved relative to this versioned binary.
patchelf --set-interpreter "$linker" "$dest/jcode"
patchelf --force-rpath --set-rpath '$ORIGIN/lib' "$dest/jcode"

# Smoke-test before pointing JCode's stable symlink at the new binary.
if ! (unset LD_PRELOAD; "$dest/jcode" --version); then
  rm -rf "$dest"
  fail 'The optimized binary failed --version and was not activated. Existing JCode remains selected.'
fi

# Keep the first pre-optimizer target so repeated installs do not overwrite rollback.
if [ ! -f "$state/previous-kind" ]; then
  if [ -L "$stable" ]; then
    printf 'symlink\n' > "$state/previous-kind"
    readlink "$stable" > "$state/previous-target"
  elif [ -f "$stable" ]; then
    printf 'file\n' > "$state/previous-kind"
    cp -p "$stable" "$state/previous-binary"
  else
    printf 'missing\n' > "$state/previous-kind"
  fi
fi

cp "$source_dir/rollback-termux.sh" "$state/rollback.sh"
chmod 700 "$state/rollback.sh"
printf '%s\n' "$dest" > "$state/installed-dir"
ln -sfn "$dest/jcode" "$stable"

printf '\nInstalled optimized JCode binary:\n  %s\n' "$dest/jcode"
printf 'Rollback command:\n  bash %s\n' "$state/rollback.sh"
printf '\nEnable optimizer in this shell:\n  export JCODE_TOKEN_OPTIMIZER=1\n'
printf 'Optional measurement logging:\n  export JCODE_TOKEN_OPTIMIZER_REPORT=1\n'
