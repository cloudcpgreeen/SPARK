#!/usr/bin/env bash
#
# P7-P4.1 · B 路线的仓库内生产路径（wit-bindgen + wasm-tools）。
#
# 范围（P7-P4.0 范围锁）：**只服务新增 async / future<T> Component**。
# 既有 0.1.0 组件仍走 build-ui.sh + cargo-component（wit-bindgen-rt 0.41）——
# 两条依赖线服务不同的 Component 集合。本脚本不调用、不修改 build-ui.sh，
# 也不碰 counter_store.wasm / wit/capability.wit / 那 11 个 pin。
#
# 为什么不用 cargo-component：其内嵌 wit-bindgen 0.41 与当前 future<T> ABI 的
# intrinsic 集合已错代（G9）。这里是 additive 路径，不是替代品。
set -euo pipefail

cd "$(dirname "$0")"

# 精确 pin：这条路径的「可重复」不能靠 semver 运气。
WASM_TOOLS=1.259.0

COMPONENT=future-reader
WIT_DIR="components/$COMPONENT/wit"
WORLD=future-reader-world
# 与 cargo 的 core wasm 产物**不同名**：否则重跑时 cargo 认为产物是新鲜的，
# 而 embed 拿到的会是上一次的 component 而不是 core module。
CORE="components/$COMPONENT/target/wasm32-unknown-unknown/release/future_reader.wasm"
EMBEDDED="$CORE.embedded.wasm"
OUT="components/$COMPONENT/target/wasm32-unknown-unknown/release/future_reader.component.wasm"

# ── ① 工具（B 路线的两个工具都要；wit-bindgen 已由 Cargo.toml 精确 pin）──
actual=$(wasm-tools --version | awk '{print $2}')
[ "$actual" = "$WASM_TOOLS" ] || {
  echo "FAIL: 需要 wasm-tools ${WASM_TOOLS}，当前 $actual" >&2
  echo "      cargo install wasm-tools --version ${WASM_TOOLS} --locked" >&2
  exit 1
}

# ── ② 读取 0.3.0 WIT（不复制、不内联：软链到 wit/capability-0.3.0.wit）──
# 契约身份可核：sha 必须与 G6 选中的那份逐字相同。
CONTRACT_SHA=$(shasum -a 256 wit/capability-0.3.0.wit | awk '{print $1}')
echo "contract  wit/capability-0.3.0.wit  sha256=$CONTRACT_SHA"

# ── ③ 生成 bindings + 编译 core wasm（wit-bindgen 在 generate! 宏里跑）──
cargo build --release --target wasm32-unknown-unknown --manifest-path "components/$COMPONENT/Cargo.toml"

# ── ④ component embed + new ──
wasm-tools component embed "$WIT_DIR" --world "$WORLD" "$CORE" -o "$EMBEDDED"
wasm-tools component new "$EMBEDDED" -o "$OUT"

# ── ⑤ validate ──
# 用 `all` 而不是具体 feature 名：feature 名在版本间改过，`all` 是稳定的。
wasm-tools validate --features all "$OUT"

echo "OK: $OUT"
echo "    sha256=$(shasum -a 256 "$OUT" | awk '{print $1}')"
echo "    size=$(wc -c < "$OUT" | tr -d ' ') B"
wasm-tools component wit "$OUT"
