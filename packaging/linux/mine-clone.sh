#!/bin/sh
# Launcher installed as /usr/bin/mine-clone by the RPM.
# The game keeps its data (saves, settings, screenshots, resource packs) in the
# folder it runs from, so each user gets a folder in ~/.local/share/mine-clone
# that links back to the read-only game files in /opt/mine-clone.
set -e
GAME=/opt/mine-clone
DIR="${XDG_DATA_HOME:-$HOME/.local/share}/mine-clone"
mkdir -p "$DIR/resourcepacks" "$DIR/shaderpacks"

# Game data: a link, so updates of the package reach existing players.
# Replace the link with a real folder to edit the packs by hand.
if [ ! -e "$DIR/packs" ] || [ -L "$DIR/packs" ]; then
    ln -sfn "$GAME/packs" "$DIR/packs"
fi
for f in "$GAME"/resourcepacks/* "$GAME"/shaderpacks-examples/*; do
    [ -e "$f" ] || continue
    case "$f" in
        */resourcepacks/*) dest="$DIR/resourcepacks/$(basename "$f")" ;;
        *) dest="$DIR/shaderpacks/$(basename "$f")" ;;
    esac
    [ -e "$dest" ] || [ -L "$dest" ] || ln -s "$f" "$dest"
done
for doc in README.md COMO-JOGAR.txt; do
    [ -e "$DIR/$doc" ] || ln -s "$GAME/$doc" "$DIR/$doc"
done

cd "$DIR"
exec "$GAME/mine-clone" "$@"
