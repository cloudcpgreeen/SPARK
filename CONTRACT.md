# 契约约定（CONTRACT）

> 约定一 · 接口即契约，契约即 WIT。这是 SPARK 三条协作约定里最根本的一条。
> 为什么有这个约定，见[理念宣言（圣经）](MANIFESTO.md)。

## 1. 契约即 WIT

组件边界的**唯一权威接口来源**是 `wit/` 目录下的 WIT 文件。实现必须满足 WIT 接口，接口之外的东西不构成契约——跨组件调用只走 WIT，禁止绕过契约直接读他人存储或内部函数。

## 2. 命名空间

WIT package 使用 `spark:<module>@<version>` 形式：

- `spark:core@0.1.0` — 核心领域契约（`wit/core.wit`，骨架）。
- `spark:runtime@0.4.0` — 组件运行时契约（`wit/runtime.wit`）：`plugin` 接口（`info` 元数据 + `transform` 返回 `result<string, plugin-error>`，错误带结构化 `code`/`message`；Agent 调用面 `schema` 工具清单 + `invoke(tool, args)` 结构化调用）+ `plugin-world` 世界。插件 = 导出此世界的组件。
- 未来按领域拆模块：`spark:order@x.y.z`、`spark:identity@x.y.z` 等，一个模块一个 package。

## 3. 契约优先工作流（idea 落地第一步）

无论 idea 从哪来（DeepSeek 咨询、用户需求、重构），落地顺序固定为：

1. **翻译成 WIT**：先把 idea 拆成职责，写成 `wit/` 下的 WIT 接口（含版本），写清每个函数的输入输出语义。
2. **实现**：按接口实现；接口没声明的能力不做。
3. **验收**：用契约验收——测试按接口语义断言（输入 → 期望输出），契约变更必须先改 WIT 再改实现。

> 当前阶段：`spark:runtime@0.4.0` 已落地 —— `transform` 返回 `result<string, plugin-error>`（插件可声明式失败并给出结构化 `code`/`message`，无需 panic）、`info` 携带插件元数据、`schema`/`invoke` 构成 Agent 调用面（插件对 LLM 暴露工具清单并按名结构化调用，见 `wit/runtime.wit`），配套 `spark-plugin`（Upper）/`spark-plugin-reverse`/`spark-plugin-attacker`/`spark-plugin-idcard`/`spark-plugin-luhn`/`spark-plugin-rmb` 与 `spark-host`（wasmtime 宿主，bindgen 钉死 0.4.0 契约，加载不匹配组件直接失败；支持多插件流水线 `pipe` 与 Agent 回路 `agent`）。

## 4. 版本规则

WIT package 遵循语义化版本（semver）：

| 变更类型 | 版本动作 |
| --- | --- |
| 破坏性变更（改签名/语义、删接口） | 升主版本 `x` |
| 新增接口 / 兼容扩展 | 升次版本 `y` |
| 仅实现内部修正 | 升补丁 `z` |

接口变更先改 WIT（提交里带上版本号），再改实现。`spark-core` 的 `contract_version()` 必须与 WIT package 版本对齐（见 DEVELOPMENT.md 测试）。

## 5. 组件边界

- 一个 WIT 接口 = 一个职责。
- `spark-core` 是无 HTTP 纯库：领域逻辑放这里，不碰网络/IO。
- **插件 = 导出 `plugin-world` 的零依赖 WASM 组件**：只依赖 `wit/runtime.wit`，不 import 任何宿主能力（含 WASI）。
- **宿主（`spark-host`）= 沙箱加载器**：只做加载 / 调用 / 捕获，不写领域逻辑；插件 panic（trap）必须当作可恢复错误处理，宿主进程不崩、实例间互不污染。
- 新增插件 = 新增一个满足契约的组件，宿主零改动。
- 新增领域逻辑优先进 `spark-core`；只有某边界层独有的逻辑才留在该 crate。
