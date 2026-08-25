#!/usr/bin/env bash
# 一键构建全部插件并装入 plugins/。
# 用法：./build-plugins.sh   （等价于在 6 个插件目录各跑一遍 cargo component build --release + 拷贝）
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v cargo-component >/dev/null 2>&1; then
    echo "缺少 cargo-component，先装：cargo install cargo-component" >&2
    exit 1
fi

for dir in spark-plugin spark-plugin-reverse spark-plugin-attacker spark-plugin-idcard spark-plugin-luhn spark-plugin-rmb; do
    echo "==> 构建 $dir"
    (cd "$dir" && cargo component build --release)
    wasm="$(cd "$dir" && find "$PWD/target/wasm32-unknown-unknown/release" -maxdepth 1 -name '*.wasm')"
    cp "$wasm" plugins/
done

echo "全部插件已装入 plugins/。spark-host list 查看，或直接 spark-host agent \"把 hello 转大写\"。"
