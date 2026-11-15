set -eu
export WLR_BACKENDS=x11 WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 GDK_BACKEND=wayland
export XDG_RUNTIME_DIR="$RUNNER_TEMP/sway-runtime"
mkdir -pm 700 "$XDG_RUNTIME_DIR" "$RUNNER_TEMP/native-workflows"
cat >"$RUNNER_TEMP/sway.conf" <<'EOF'
xwayland disable
default_border none
default_floating_border none
output * mode 1280x720
seat seat0 fallback true
for_window [title="Font QA drag source"] floating enable, resize set 200 100, move position 10 10
EOF
sway -c "$RUNNER_TEMP/sway.conf" >"$RUNNER_TEMP/native-workflows/sway.log" 2>&1 &
compositor=$!
trap 'kill "$compositor" 2>/dev/null || true' EXIT
for i in {1..100}; do
  [[ -S "$(find "$XDG_RUNTIME_DIR" -name 'wayland-*' -type s | head -1)" ]] && break
  sleep .15
done
export SWAYSOCK="$(find "$XDG_RUNTIME_DIR" -name 'sway-ipc*.sock' | head -1)"
export WAYLAND_DISPLAY="$(basename "$(find "$XDG_RUNTIME_DIR" -name 'wayland-*' -type s | head -1)")"
# Application clients only have the Wayland display; keep Xvfb available to the compositor.
export COMPOSITOR_DISPLAY=$DISPLAY
unset DISPLAY
swaymsg -t get_outputs
status=0
for scenario in drop batch cancel; do python3 qa/wayland.py "$scenario" "$GUI" || status=1; done
exit "$status"
