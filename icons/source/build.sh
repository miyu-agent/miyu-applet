#!/bin/sh
# 出全部图标。应用图标、托盘图标照项目主人给的那一套 PNG 原样拷（不改色、不缩放）：一个目录下 compact-margin/、original-margin/
# 两份留白，各有 miyu-agent-<边长>x<边长>.png（16、32、48、64、128、256、512、1024），透明底。
#   sh build.sh <那一套的目录>    出应用图标和托盘图标（要 ImageMagick 7）
#   sh build.sh menu              只出菜单里的图标（要 rsvg-convert、oxipng）
#
# - 应用图标（Tauri 的 bundle.icon、Linux 包里 hicolor 的图标、Windows 编进程序的 icon.ico）：原留白（original-margin）。
#   logo.png 是原留白的 1024px，留着当原图。
# - 托盘（三个平台都是这一张彩色图，macOS 也不用模板图）：留白少的（compact-margin）64px，托盘缩到 16～32px 时徽章大一些、看得清；
#   macOS 菜单栏缩到 22 点高，两倍屏上 44px，64px 够用。
# - 菜单里的图标（施工 AP-2）：lucide/ 下的 Lucide 原稿（ISC 许可，见 lucide/LICENSE；和网页用的同一套），灰色线条 32px。
#   深色、浅色菜单上都看得见；macOS 当模板图用，系统照菜单着色。
set -eu
cd "$(dirname "$0")"
out=..

# 菜单里的图标：线条换成灰色，照 32px 出。
menu() {
    tmp=$(mktemp -d)
    mkdir -p $out/menu
    for name in square play rotate-cw power; do
        sed 's/currentColor/#808080/' "lucide/$name.svg" > "$tmp/$name.svg"
        rsvg-convert -w 32 -h 32 "$tmp/$name.svg" -o "$out/menu/$name.png"
    done
    rm -rf "$tmp"
    oxipng -q -o 4 --strip safe --nc $out/menu/*.png
}
if [ "${1:-}" = menu ]; then menu; exit 0; fi

set_dir=${1:?用法：sh build.sh <logo 那一套的目录> | menu}
o="$set_dir/original-margin/miyu-agent"
c="$set_dir/compact-margin/miyu-agent"

cp "$o-1024x1024.png" logo.png
cp "$o-512x512.png" $out/icon.png
cp "$o-32x32.png" $out/32x32.png
cp "$o-128x128.png" $out/128x128.png
cp "$o-256x256.png" $out/256x256.png
magick "$o-16x16.png" "$o-32x32.png" "$o-48x48.png" "$o-64x64.png" "$o-128x128.png" "$o-256x256.png" $out/icon.ico
cp "$c-64x64.png" $out/tray.png
# 不再压：拷来的逐字节就是原图（都是 8 位 RGBA，Tauri 认）。
