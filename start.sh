#!/usr/bin/env bash
cd "$(dirname "$0")"
# Dedicated Blightnet window. Do not export BROWSER=firefox/brave here.
GST_DIR="$(pwd)/gst"
if [ -d "$GST_DIR" ]; then
  export GST_PLUGIN_PATH="$GST_DIR${GST_PLUGIN_PATH:+:$GST_PLUGIN_PATH}"
fi
exec python3 serve.py "$@"
