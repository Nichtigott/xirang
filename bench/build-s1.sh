#!/bin/sh
# S1 题态夹具：一个**空目录**（任务是在里面从零立起一个 XiRang 宿主）。用法：build-s1.sh <目标目录>
set -e
E="$1"
rm -rf "$E"; mkdir -p "$E"
echo "S1 fixture ready (empty) at $E"
