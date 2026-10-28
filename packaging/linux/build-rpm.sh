#!/usr/bin/env bash
# Builds dist/mine-clone-<version>-<release>.x86_64.rpm from target/release/mine-clone.
# Needs rpmbuild (Fedora: dnf install rpm-build; Ubuntu: apt install rpm).
# Usage: packaging/linux/build-rpm.sh [extra-resourcepacks-dir]
#   The optional folder's .zip packs are bundled too (local builds only:
#   packs with Mojang art never go into the repository or GitHub releases).
set -euo pipefail
cd "$(dirname "$0")/../.."
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
RELEASE="${RELEASE:-1}"
BIN=target/release/mine-clone
[ -x "$BIN" ] || { echo "build first: cargo build --release -p game" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
STAGE="$WORK/stage"
mkdir -p "$STAGE/opt/mine-clone/resourcepacks" "$STAGE/icons"
install -m 0755 "$BIN" "$STAGE/opt/mine-clone/mine-clone"
cp -r packs shaderpacks-examples "$STAGE/opt/mine-clone/"
cp README.md docs/COMO-JOGAR.txt "$STAGE/opt/mine-clone/"
if [ -n "${1:-}" ]; then
    cp "$1"/*.zip "$STAGE/opt/mine-clone/resourcepacks/"
fi
cp assets/icon/icon_{32,48,64,128,256,512}.png "$STAGE/icons/"
cp packaging/linux/mine-clone.sh packaging/linux/mine-clone.desktop "$STAGE/"

rpmbuild -bb packaging/linux/mine-clone.spec \
    --define "_topdir $WORK/rpm" \
    --define "stage $STAGE" \
    --define "game_version $VERSION" \
    --define "game_release $RELEASE" \
    --define "_binary_payload w19.zstdio"
mkdir -p dist
cp "$WORK"/rpm/RPMS/x86_64/*.rpm dist/
ls -l dist/*.rpm
