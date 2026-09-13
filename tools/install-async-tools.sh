#!/usr/bin/env bash
#
# P7-P4.2 · ②：async Component 生产线的工具安装入口（仓库定义，取代开发者记忆）。
#
# 范围：**只装 async 线（B）的工具**。0.1.0 同步线的 cargo-component 仍按
# CONTRIBUTING §2 装 —— 本脚本不接管它，也不碰任何 component / pin / artifact。
# 为什么要脚本而不是一行文档：`cargo install` 成功 ≠ PATH 上先命中的就是它
# （Round 1 的对照实验正是用 1.245.1 遮蔽 PATH 才验出版本门有效）。
# 所以这里装完**核对一次**，让「精确 pin」是实测的，不是文档声称的。
set -euo pipefail

# 精确 pin：这条路径的「可重复」不能靠 semver 运气。
WASM_TOOLS=1.259.0

cargo install wasm-tools --version "$WASM_TOOLS" --locked

actual=$(wasm-tools --version | awk '{print $2}')
[ "$actual" = "$WASM_TOOLS" ] || {
  echo "FAIL: 需要 wasm-tools ${WASM_TOOLS}，当前 $actual" >&2
  echo "      PATH 上先命中的不是刚装的那个 —— 检查 ~/.cargo/bin 是否在 PATH 前段。" >&2
  exit 1
}

echo "OK: wasm-tools $actual"
