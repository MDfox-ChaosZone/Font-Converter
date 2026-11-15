set -eu
if [[ "$(uname)" == Darwin ]]; then
    python3 qa/verify.py "$1" "$GUI"
else
    xvfb-run -a -s "-screen 0 1600x1000x24" dbus-run-session -- bash -c '
        export GDK_BACKEND=x11
        unset WAYLAND_DISPLAY
        openbox >/dev/null 2>&1 &
        sleep 1
        python3 qa/verify.py "$1" "$GUI"' test "$1"
fi
