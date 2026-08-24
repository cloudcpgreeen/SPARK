# SPARK

> 组件运行时：领域逻辑写成满足 `spark:runtime` 契约的 WASM 组件，宿主沙箱加载调用。契约即 WIT。
> 开源协议：[GPL-3.0](LICENSE)。

## 文档

| 文档 | 内容 |
| --- | --- |
| [`MANIFESTO.md`](MANIFESTO.md) | 圣经 · 理念宣言：项目为什么存在、是什么、边界在哪、怎么保护自己 |
| [`CONTRACT.md`](CONTRACT.md) | 约定一 · 契约：接口即契约，契约即 **WIT**；契约优先工作流与版本规则 |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | 约定二 · 开发：项目结构、构建测试、代码风格、新增功能流程 |
| [`DEPLOYMENT.md`](DEPLOYMENT.md) | 约定三 · 交付：交付门禁、版本发布、运行/配置/安全 |
| [`SECURITY.md`](SECURITY.md) | 安全：威胁模型、上报流程 |
| [`EXAMPLES.md`](EXAMPLES.md) | 使用范例：手把手从零写插件、构建、跑起来 |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | 完整的版本：框架怎么形成的 + 具体实现逻辑 |
| [`RELATIONSHIPS.md`](RELATIONSHIPS.md) | 人物关系图：圣经里的所有人物与关系，一张网收束 |
| [`ESSAY.md`](ESSAY.md) | 理念外一篇：论论文安全——不是防学生，是护写作者 |

## 结构

- Cargo workspace：`spark-core`（无 HTTP 领域库）、`spark-host`（wasmtime 宿主）。
- `spark-plugin`：插件组件（独立 workspace），产出零依赖 WASM 组件，导出 `spark:runtime/plugin`。
- `wit/`：`core.wit`（`spark:core@0.1.0` 骨架）、`runtime.wit`（`spark:runtime@0.3.0`，`plugin-world` 契约，`transform` 返回 `result<string, plugin-error>`、`info` 带元数据）。

## 快速上手

```bash
cd spark-plugin && cargo component build --release
cargo run -p spark-host -- spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm "hello"
```

示例插件 `Upper`：输入转大写；输入以 `trap` 开头时触发 panic，宿主以 trap 捕获、进程不崩（沙箱隔离）；输入以 `err` 开头时返回声明式错误（值，不是崩溃）。

### 插件自注册（注册/发现）

把满足 `plugin-world` 契约的 `.wasm` 组件放进 `plugins/` 目录即注册（name 来自组件自身 `info()`，宿主零配置文件）：

```bash
mkdir -p plugins && cp spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm plugins/
cargo run -p spark-host -- list           # 发现并列出
cargo run -p spark-host -- run upper hi   # 按名字运行
cargo run -p spark-host -- pipe hi upper reverse   # 流水线：依次经 upper、reverse 串联
```

## 新增插件

写一个满足 `plugin-world` 契约的组件（见 `wit/runtime.wit`），宿主零改动即可加载。

现有示例插件：`spark-plugin`（upper，输入转大写）、`spark-plugin-reverse`（reverse，输入倒序）、`spark-plugin-attacker`（恶意示例，安全验证用：CPU/内存炸弹会被沙箱切断）、`spark-plugin-idcard`（真实业务算法：中国身份证号校验，返回性别/出生日期/地区或结构化错误）、`spark-plugin-luhn`（真实业务算法：银行卡号 Luhn 校验 + 卡组织识别）。五个插件用同一个 `spark-host` 二进制加载，宿主零改动。

`pipe` 把多个插件串成流水线：前一个插件输出喂给下一个，任一步声明式失败或 trap 即 fail-fast 并定位到具体插件（如 `pipe <身份证号> idcard luhn`：身份证校验通过后，Luhn 因长度拒绝并报 `[length]`）。
