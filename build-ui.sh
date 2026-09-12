#!/usr/bin/env bash
# 构建跨端域组件（每个一次），并把同一份 wasm 分发给各宿主。
# 用法：./build-ui.sh
#
# 关键：每个 Component 只 build 一次。Web / RN 侧拿到的是 jco 的**适配产物**
# （js + core.wasm），不是第二个 Component —— 契约与制品始终唯一。
#
# P2：`counter-store.wasm` 的 sha256 会打印出来。换 Host 的 capability 实现时，
# 这个值必须不变 —— 那就是「组件不需要重新编译」的证据。
set -euo pipefail
cd "$(dirname "$0")"

build_component() {
  local dir="$1" out="$2"
  echo "==> 构建 components/${dir}（唯一一次 Component build）" >&2
  (cd "components/$dir" && cargo component build --release)
  echo "$PWD/components/$dir/target/wasm32-unknown-unknown/release/${out}.wasm"
}

BUTTON="$(build_component button button)"
STORE="$(build_component counter-store counter_store)"
PROVIDER="$(build_component mem-store mem_store)"

echo
echo "==> 契约自检（世界 + import/export）"
npx --no-install jco wit "$BUTTON" | grep -E "^world|  (world|export|import)" || true
npx --no-install jco wit "$STORE" | grep -E "^world|  (world|export|import)" || true

echo
echo "==> 制品 sha256（P2 判据：换 Host 实现时 STORE 的值不变）"
echo "button.wasm        $(shasum -a 256 "$BUTTON" | cut -d' ' -f1)"
SHA_BEFORE="$(shasum -a 256 "$STORE" | cut -d' ' -f1)"
echo "counter-store.wasm $SHA_BEFORE"

echo
echo "==> P3 组合：Provider Component 实现 capability，import 被组合消掉"
# wasm-tools compose 要求**定义组件的文件名**是 kebab-case；cargo-component 产出的
# mem_store.wasm 不符合，所以复制一份再喂给它。文件名不参与接口身份，只影响这一步。
mkdir -p dist
cp "$PROVIDER" dist/mem-store.wasm
wasm-tools compose "$STORE" -d dist/mem-store.wasm --no-imports -o dist/composed.wasm
echo "mem-store.wasm     $(shasum -a 256 "$PROVIDER" | cut -d' ' -f1)"
echo "composed.wasm      $(shasum -a 256 dist/composed.wasm | cut -d' ' -f1)"

# composed.wasm 是**派生制品**，不是第三个 Component 源。组合不该动到任何一个源制品。
SHA_AFTER="$(shasum -a 256 "$STORE" | cut -d' ' -f1)"
if [ "$SHA_BEFORE" != "$SHA_AFTER" ]; then
  echo "FAIL: 组合动了 counter-store.wasm（$SHA_BEFORE → $SHA_AFTER）" >&2
  exit 1
fi
echo "OK: 组合前后 counter-store.wasm 的 sha256 不变"

echo
echo "==> jco transpile → hosts/web/src/generated/<name>（Host 适配产物，非第二个 Component）"
rm -rf hosts/web/src/generated
mkdir -p hosts/web/src/generated
npx --no-install jco transpile "$BUTTON" -o hosts/web/src/generated/button --name button
# --map 生成的是**字面相对 import**，所以 capability 实现必须落在产物目录旁边。
npx --no-install jco transpile "$STORE" -o hosts/web/src/generated/store --name counter-store \
  --map 'spark:capability/storage=./storage.js'
cp hosts/web/src/capability/storage.js hosts/web/src/generated/store/storage.js
# 这一行**没有 --map** —— 它没有 import 需要注入，这正是 P3 的证据。
npx --no-install jco transpile dist/composed.wasm -o hosts/web/src/generated/composed --name composed

echo
echo "组件产物: $BUTTON"
echo "          $STORE"
echo "Provider: $PROVIDER"
echo "组合产物: $PWD/dist/composed.wasm（derived，非 Component 源）"
echo "Rust 后端: cargo run -p spark-host -- domain $BUTTON 3"
echo "           cargo run -p spark-host -- store $STORE 3        # 健康存储 → reloaded: 3"
echo "           cargo run -p spark-host -- store $STORE 3 --deny # 拒绝写入 → reloaded: 0"
echo "           cargo run -p spark-host -- provide $PROVIDER k v # → got: v"
echo "           cargo run -p spark-host -- composed dist/composed.wasm 3 # → count: 3 / reloaded: 0"
echo "Web 宿主 : cd hosts/web && npm install && npm run dev"
