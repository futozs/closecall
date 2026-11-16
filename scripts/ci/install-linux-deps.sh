#!/usr/bin/env bash
# System libraries Tauri 2 needs to build on Debian/Ubuntu. Extra packages can be passed as arguments.
set -euo pipefail

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  libayatana-appindicator3-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev \
  patchelf \
  "$@"
