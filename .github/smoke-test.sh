#!/usr/bin/env bash
# Starts a packaged Chessdesk with a fresh config, checks that it comes up and stays up,
# and saves a screenshot for a human to look at. Linux runs it under Xvfb.
#
#   smoke-test.sh <binary> <screenshot.png>
set -euo pipefail

binary=$1
screenshot=$2
seconds=20

CHESSDESK_CONFIG_DIR=$(mktemp -d)
export CHESSDESK_CONFIG_DIR
log=$(mktemp)

# gpui draws nothing on an X server without a window manager, which no real desktop lacks.
if [ "$(uname)" = Linux ]; then
    openbox &
    sleep 2
fi

"$binary" >"$log" 2>&1 &
pid=$!
sleep "$seconds"

if ! kill -0 "$pid" 2>/dev/null; then
    echo "::error::Chessdesk exited within ${seconds}s"
    cat "$log"
    exit 1
fi

case "$(uname)" in
    Darwin) screencapture -x "$screenshot" || echo "::warning::no screenshot" ;;
    Linux) import -window root "$screenshot" ;;
esac

kill "$pid"
wait "$pid" || true
cat "$log"

if grep -q "panicked" "$log"; then
    echo "::error::Chessdesk panicked"
    exit 1
fi
if [ ! -f "$CHESSDESK_CONFIG_DIR/plugins/cburnett/plugin.toml" ]; then
    echo "::error::the bundled plugins weren't unpacked"
    exit 1
fi
# An empty Xvfb screen is one colour; a drawn window has many.
if [ "$(uname)" = Linux ]; then
    colors=$(identify -format %k "$screenshot")
    echo "screenshot has $colors colours"
    if [ "$colors" -lt 50 ]; then
        echo "::error::nothing was drawn"
        exit 1
    fi
fi
echo "Chessdesk ran for ${seconds}s"
