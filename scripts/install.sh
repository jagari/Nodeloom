#!/bin/sh
# Installs nodeloom-agent and nodeloom-master on macOS or Linux.
#
#   ./install.sh                 from an unpacked release archive
#   sh install.sh                elsewhere: downloads the newest release
#                                with the GitHub CLI (the repository is
#                                private, so `gh auth login` first)
#
# Environment:
#   NODELOOM_INSTALL_DIR  where the binaries go (default ~/.local/bin)
#   NODELOOM_VERSION      release tag to download, e.g. v0.1.0-beta.1
#                         (default: the newest release, betas included)
#   NODELOOM_REPO         GitHub repository (default jagari/Nodeloom)
set -eu

REPO="${NODELOOM_REPO:-jagari/Nodeloom}"
DEST="${NODELOOM_INSTALL_DIR:-$HOME/.local/bin}"
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

say() { printf '%s\n' "$*"; }
fail() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

platform() {
  case "$(uname -s)" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    *) fail "unsupported system $(uname -s); on Windows use install.ps1" ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch=x64 ;;
    arm64 | aarch64) arch=arm64 ;;
    *) fail "unsupported CPU $(uname -m)" ;;
  esac
  if [ "$os" = macos ] && [ "$arch" = x64 ]; then
    fail "Intel Macs have no build yet; build from source (see README)"
  fi
  printf '%s-%s' "$os" "$arch"
}

# Finds the binaries: next to this script, or in a freshly downloaded
# release archive.
if [ -x "$HERE/nodeloom-agent" ] && [ -x "$HERE/nodeloom-master" ]; then
  SRC="$HERE"
else
  command -v gh >/dev/null 2>&1 ||
    fail "no binaries next to this script and no GitHub CLI (gh) to download them"
  target=$(platform)
  tag="${NODELOOM_VERSION:-}"
  if [ -z "$tag" ]; then
    # `gh release download` alone skips pre-releases; betas are what exist.
    tag=$(gh release list --repo "$REPO" --limit 1 --json tagName --jq '.[0].tagName') ||
      fail "could not list releases of $REPO (run: gh auth login)"
    [ -n "$tag" ] || fail "$REPO has no releases yet"
  fi
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  say "Downloading $tag for $target from $REPO..."
  gh release download "$tag" --repo "$REPO" --dir "$work" \
    --pattern "nodeloom-*-$target.tar.gz" --pattern "SHA256SUMS" ||
    fail "download failed"
  archive=$(ls "$work"/nodeloom-*-"$target".tar.gz)
  if [ -f "$work/SHA256SUMS" ]; then
    name=$(basename "$archive")
    expected=$(grep " $name\$" "$work/SHA256SUMS" | cut -d' ' -f1)
    if command -v sha256sum >/dev/null 2>&1; then
      actual=$(sha256sum "$archive" | cut -d' ' -f1)
    else
      actual=$(shasum -a 256 "$archive" | cut -d' ' -f1)
    fi
    [ "$expected" = "$actual" ] || fail "checksum mismatch for $name"
  fi
  tar -xzf "$archive" -C "$work"
  SRC=$(dirname "$(find "$work" -name nodeloom-agent -type f | head -n 1)")
fi

mkdir -p "$DEST"
for bin in nodeloom-agent nodeloom-master; do
  # Copy to a temporary name and rename, so a running copy is not
  # overwritten in place.
  cp "$SRC/$bin" "$DEST/.$bin.new"
  chmod 755 "$DEST/.$bin.new"
  mv -f "$DEST/.$bin.new" "$DEST/$bin"
done

# Binaries downloaded with a browser are quarantined on macOS, and the beta
# is not signed, so Gatekeeper would refuse to start them.
if [ "$(uname -s)" = Darwin ] && command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.quarantine "$DEST/nodeloom-agent" "$DEST/nodeloom-master" 2>/dev/null || true
fi

say "Installed to $DEST:"
"$DEST/nodeloom-agent" --version
"$DEST/nodeloom-master" --version

case ":$PATH:" in
  *":$DEST:"*) ;;
  *)
    say ""
    say "$DEST is not on your PATH. Add it, for example:"
    say "  echo 'export PATH=\"$DEST:\$PATH\"' >> ~/.profile && . ~/.profile"
    ;;
esac

say ""
say "Next steps (docs/BETA.md has the full guide):"
say "  1. On one machine:   nodeloom-master"
say "     It prints a dashboard link; open it in a browser."
say "  2. On each machine:  nodeloom-agent"
say "     Approve it in the dashboard when the codes match."
