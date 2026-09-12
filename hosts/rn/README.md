# RN spike：同一个 `button.wasm` 能否成为 React Native 的宿主？

**结论：部分通过 —— 编译门 PASS，运行时门未验证。降级方案已备。**

这是 **Consumer compatibility spike**，不是 P1 的核心证明（核心是 Rust 后端与 Web 浏览器）。
**契约一个字都没改**：`wit/ui.wit` 与 `components/button/src/lib.rs` 在本次 spike 前后完全相同。

复现：`./hosts/rn/spike.sh`

---

## 一、实测结果（可复现）

| # | 门 | 结果 |
| --- | --- | --- |
| R0 | jco 产物在 Node 里跑通 | **PASS**（`count = 3`，见 `hosts/web` 同款产物） |
| R1 | jco **默认**产物的 Hermes 编译 | **FAIL** —— top-level await |
| R1.5 | 加 `--tla-compat --no-namespaced-exports` 后 | **PASS** —— Hermes 编译成字节码，exit 0、0 errors |
| R2 | **Hermes 运行时是否有 `WebAssembly`** | **未验证** —— 见下 |

## 二、两个真实失败，与它们的修法

### 失败 1：top-level await（真实阻塞）

jco 默认在模块顶层生成 `await $init;`。Hermes 没有 top-level await：

```
button.js:3556:7: error: ';' expected
await $init;
```

**修法：`jco transpile --tla-compat`** —— 它把顶层 await 换成一个导出的 `$init` promise，
调用方自己 `await mod.$init`。这正是 RN 侧该用的形态。

### 失败 2：字符串字面量导出名（真实阻塞）

```
button.js:3569:44: error: 'identifier' expected in export clause
export { button010 as 'spark:ui/button@0.1.0', }
```

任意模块命名空间名（ES2022）Hermes 不支持。

**修法：`jco transpile --no-namespaced-exports`** —— 只保留 `export { button010 as button }`。

### 不是失败的：「export 语句需要 module mode」

直接喂 hermesc 一个 ESM 文件会报这个。**它不是 RN 的阻塞点** —— Metro 在交给 Hermes 之前
就会把 ESM 转成 CJS，Hermes 从来见不到 `export`。所以正确的复现方式是先按 Metro 的方式
打成 CJS bundle 再喂 hermesc，本仓的 `spike.sh` 就是这么做的（用 esbuild 模拟这一步）。

打完 CJS 后：`hermesc -emit-binary` **exit 0，0 errors**，产出 85 KB 字节码。

## 三、未验证的那一半：Hermes 的 WebAssembly

编译过 ≠ 跑得起来。jco 产物运行时需要 `WebAssembly.instantiate` 去实例化
`button.core.wasm`，而：

- Hermes **V1（RN 0.84，2026-02）原生支持 wasm**，但这是一条**独立推进线**，
  **不是升级到 RN 0.84 就自动打开**；
- 尚未验证 Hermes 的 wasm 能否实例化 jco 生成的 core wasm（可能受 MVP 限制：
  bulk-memory / sign-ext / reference-types 等）。

**没有做 R2 的原因**：需要一个开着 wasm 的真机/模拟器 RN app，且当前没有
可用的、带 wasm 的独立 Hermes VM 二进制可供快速验证（`hermes-compiler` 只含
编译器 `hermesc`，不含 VM）。这一半没有测，就不写成测过了。

### R2 该怎么测（等 Hermes wasm 打开后）

1. 建最小 RN app（Hermes 开、JSC 关），一个屏幕；
2. `import` 经上述三个 flag 转译的产物，`await $init`，`new button.Counter()`；
3. 渲染 `count()` 到 `<Text>`，按下 `click()`；
4. **成功判据：模拟器上点 3 次，数字读数 3。** 少一点都不算。

## 四、降级阶梯（按代价从小到大）

1. **`jco transpile --js`** —— core wasm 转成纯 JS，完全绕开 `WebAssembly`。
   代价：体积大、约慢一个数量级。但若跑通，就真的证明了「同一个组件、同一份契约，跑在 RN 上」，
   性能是 P3 的优化问题。**先试这个。**
2. **预打包 + `--tla-compat`** —— 若阻塞在 Metro 而非 Hermes。
3. **JSI 桥接**一个 C++ wasm 引擎（wasmtime C API / wasmi）。工作量大，且
   **它放弃了要证的东西** —— 制品不再是同一个。不要放在 P1，留给 P3。
4. **推迟 RN 到 P3。**

**止损线**：R2 失败 **且** 降级 1（`--js`）也失败 → 停。把真实失败症状记在这里，继续往前走。

## 五、这个 spike 不影响 P1 的成立

P1 的核心证明是 ② Rust 后端 与 ③ Web 浏览器，两者都已 PASS。
RN 只是第四个 Consumer；它跑通与否，**不改变**「同一份 WIT + 同一个 `button.wasm`」
这一事实，也不改变契约。
