#!/bin/sh
# Renders assets/demo.gif from assets/demo/flow.txt: one frame per line that
# appears, a terminal look, then ffmpeg builds the gif. Needs ImageMagick
# (convert) and ffmpeg. The text is recreated from a real run (plan 07 try);
# edit flow.txt and run this again to change the gif.
set -eu
cd "$(dirname "$0")/../.."

flow=assets/demo/flow.txt
out=assets/demo.gif
font=${DEMO_FONT:-$HOME/.local/share/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Regular.ttf}
bold=${DEMO_FONT_BOLD:-$HOME/.local/share/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Bold.ttf}
[ -f "$font" ] || { echo "font not found: $font (set DEMO_FONT)" >&2; exit 1; }
[ -f "$bold" ] || bold=$font
command -v convert >/dev/null || { echo "needs ImageMagick (convert)" >&2; exit 1; }
command -v ffmpeg >/dev/null || { echo "needs ffmpeg" >&2; exit 1; }

w=980 h=590 size=17 lh=27 top=70 left=28 maxlines=18
nb=$(printf '\302\240')   # ImageMagick trims leading spaces; a no-break space keeps the indent
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

color() {
  case $1 in
    p) echo '#e6edf3' ;;   # what you type
    t) echo '#79c0ff' ;;   # a tool call
    r) echo '#7ee787' ;;   # its result
    w) echo '#e3b341' ;;   # a refusal
    a) echo '#c9d1d9' ;;   # the agent's answer
    *) echo '#c9d1d9' ;;
  esac
}

# A line the model prints wraps at 74 columns so it stays inside the window.
wrap() { printf '%s\n' "$1" | fold -s -w 74 | sed '2,$s/^/  /'; }

: >"$tmp/lines"        # kind|text, already wrapped, one per row
: >"$tmp/steps"        # the row count after each source line
n=0
while IFS='|' read -r kind text; do
  if [ "$kind" = s ]; then
    printf 's|\n' >>"$tmp/lines"; n=$((n + 1))
  else
    wrap "$text" | while IFS= read -r row; do printf '%s|%s\n' "$kind" "$row"; done >>"$tmp/lines"
    n=$(wc -l <"$tmp/lines")
  fi
  echo "$n $kind" >>"$tmp/steps"
done <"$flow"

frame=0
: >"$tmp/list"
while read -r count kind; do
  frame=$((frame + 1))
  args=""
  first=1
  [ "$count" -gt "$maxlines" ] && first=$((count - maxlines + 1))
  y=$top
  row=0
  while IFS='|' read -r k text; do
    row=$((row + 1))
    [ "$row" -lt "$first" ] && continue
    [ "$row" -gt "$count" ] && break
    f=$font; [ "$k" = p ] && f=$bold
    esc=$(printf '%s' "$text" | sed "s/'/'\\\\''/g; s/^  /$nb$nb/")
    args="$args -font '$f' -fill '$(color "$k")' -annotate +$left+$y '$esc'"
    y=$((y + lh))
  done <"$tmp/lines"
  eval "convert -size ${w}x${h} xc:'#0d1117' \
    -fill '#161b22' -draw 'rectangle 0,0 $w,44' \
    -fill '#ff5f56' -draw 'circle 26,22 26,29' \
    -fill '#ffbd2e' -draw 'circle 50,22 50,29' \
    -fill '#27c93f' -draw 'circle 74,22 74,29' \
    -font '$font' -pointsize 15 -fill '#8b949e' -annotate +400+28 'claude code · herdr' \
    -pointsize $size $args '$tmp/f$frame.png'"
  case $kind in
    p) d=2.2 ;; t) d=0.9 ;; r) d=1.1 ;; w) d=1.6 ;; a) d=2.6 ;; *) d=0.4 ;;
  esac
  printf "file '%s'\nduration %s\n" "$tmp/f$frame.png" "$d" >>"$tmp/list"
done <"$tmp/steps"
# The last frame is held, then the loop restarts.
printf "file '%s'\nduration 3.5\nfile '%s'\n" "$tmp/f$frame.png" "$tmp/f$frame.png" >>"$tmp/list"

ffmpeg -v error -y -f concat -safe 0 -i "$tmp/list" \
  -vf "split[a][b];[a]palettegen=max_colors=48[p];[b][p]paletteuse=dither=none" \
  -loop 0 "$out"
echo "wrote $out ($(wc -c <"$out") bytes)"
