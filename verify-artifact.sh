#!/usr/bin/env bash
# P5 G1：断言 Domain Component artifact 就是那个冻结的字节流。
#
# 在这之前，仓库里**没有任何一处**断言过这个值：
#   - build-ui.sh:32,46-52 确实 hash 了 counter-store.wasm，但那是把它和它**自己**
#     在同一次运行内比对 —— 证明的是「compose 不改消费者」，不是「等于这个常量」。
#   - 所有 `85691b8e…` 都出现在文档里，手抄、还截断了。
#   - Rust 侧唯一的 sha256 字样在 spark-host/src/compose.rs:6，是一句注释。
#
# P5 把它变成一句会失败的断言：两个 Host 的验收都必须先过这一关。
set -euo pipefail
cd "$(dirname "$0")"

ARTIFACT="components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm"
EXPECTED="85691b8e50549e0608c893ef91836fb3f5056fa380e1e34236a9a93c33e7e584"

[ -f "$ARTIFACT" ] || { echo "FAIL: $ARTIFACT 不存在" >&2; exit 1; }

ACTUAL="$(shasum -a 256 "$ARTIFACT" | cut -d' ' -f1)"

if [ "$ACTUAL" != "$EXPECTED" ]; then
  {
    echo "FAIL: Domain Component artifact 不是冻结的那一份。"
    echo "  expected $EXPECTED"
    echo "  actual   $ACTUAL"
    echo
    echo "P5 禁止重新编译 counter-store。先查清是谁动了这个文件，"
    echo "不要用「重新编译一次让它变回去」来绕过 —— 那恰好是被禁止的动作。"
  } >&2
  exit 1
fi

echo "OK: counter_store.wasm sha256 = $EXPECTED"
