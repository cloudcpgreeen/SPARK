#!/usr/bin/env bash
# P5：把冻结的 Domain Component 转译给 Web Host。
#
# 刻意**不走 build-ui.sh**，两个原因：
#   1. build-ui.sh:16 无条件跑 `cargo component build --release` —— 与 P5 G1
#      「禁止重新编译 Domain Component」直接冲突，而且每次运行都可能覆盖冻结字节。
#   2. build-ui.sh:51 是 `rm -rf hosts/web/src/generated` —— 会删掉 P1–P3 的产物。
#
# P5 有自己的输出目录 hosts/web/src/generated-p5/，P1–P3 的 generated/ 一个字不碰。
set -euo pipefail
cd "$(dirname "$0")"

ARTIFACT="components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm"
OUT="hosts/web/src/generated-p5"

# G1：**先**断言转译输入。顺序本身就是证据 —— 证明同一个字节流是这次转译的输入。
./verify-artifact.sh

rm -rf "$OUT"
mkdir -p "$OUT"

# 同一个 artifact，第二次转译，只是换了一个 capability 映射。
# 这正是 P5 要的东西：派生产物是 Host 的适配产物，被转译的 Component 始终是同一份。
npx --no-install jco transpile "$ARTIFACT" -o "$OUT" --name counter-store \
  --map 'spark:capability/storage=./storage.js'

# jco 把 import 绑成一个相对路径 ./storage.js，所以实现文件要落在它旁边。
cp hosts/web/src/capability/storage-p5.js "$OUT/storage.js"

# 再断言一次：证明这次转译**没有**动过源制品（不是「应该没动」，是观测到没动）。
./verify-artifact.sh

echo "OK: P5 Web 产物在 $OUT"
