# SPARK

> 组件运行时：领域逻辑写成满足 `spark:runtime` 契约的 WASM 组件，宿主沙箱加载调用。契约即 WIT。

## 三份约定

| 文档 | 内容 |
| --- | --- |
| [`CONTRACT.md`](CONTRACT.md) | 约定一 · 契约：接口即契约，契约即 **WIT**；契约优先工作流与版本规则 |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | 约定二 · 开发：项目结构、构建测试、代码风格、新增功能流程 |
| [`DEPLOYMENT.md`](DEPLOYMENT.md) | 约定三 · 交付：交付门禁、版本发布、运行/配置/安全（预留） |

## 结构

- Cargo workspace：`spark-core`（无 HTTP 领域库）、`spark-host`（wasmtime 宿主）。
- `spark-plugin`：插件组件（独立 workspace），产出零依赖 WASM 组件，导出 `spark:runtime/plugin`。
- `wit/`：`core.wit`（`spark:core@0.1.0` 骨架）、`runtime.wit`（`spark:runtime@0.1.0`，`plugin-world` 契约）。

## 快速上手

```bash
cd spark-plugin && cargo component build --release
cargo run -p spark-host -- spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm "hello"
```

示例插件 `Upper`：输入转大写；输入以 `trap` 开头时触发 panic，宿主以 trap 捕获、进程不崩（沙箱隔离）。

## 新增插件

写一个满足 `plugin-world` 契约的组件（见 `wit/runtime.wit`），宿主零改动即可加载。

现有示例插件：`spark-plugin`（upper，输入转大写）、`spark-plugin-reverse`（reverse，输入倒序）。两个插件用同一个 `spark-host` 二进制加载（见 `spark-host/tests/isolation.rs` 的 `second_plugin_same_host`）。
