# SPARK 路线图

## 命题

> **同一个 WIT 契约 + 同一个 Wasm Component，可以同时是 Web / React Native 前端和 Rust 后端的统一业务组件。**
> Component 本身**不知道**自己跑在前端还是后端 —— UI 是宿主的事，状态与行为是 Component 的事。

## 三个概念的边界（比「代码能跑」更重要）

| 概念 | 是什么 | 例子 |
| --- | --- | --- |
| **Component** | **共享的业务状态与行为**（headless，不含任何 UI 概念） | `button.wasm`：`constructor → click → count` |
| **Host** | **平台适配与 UI / 运行环境** | Rust + wasmtime；Web(Vite+React)；RN(Hermes) |
| **Capability** | **外部能力**（storage / remote API 等），P2 才引入 | P1 中不存在 |

**黄金不变量**：三端**不要求代码相同** —— 要求的是**契约相同、Component 相同、Domain 行为相同**；
Host 可以完全不同。各 Host 各有实例与状态（后端点 3 次 → 3；Web 点 1 次 → 1），
共享的是契约与行为，**不是状态**。

**两套信任模型**（刻意分开，互不污染）：

| world | 契约 | 信任模型 |
| --- | --- | --- |
| `plugin-world` | `spark:runtime@0.4.0` | **零 import 的不可信插件沙箱**（安全边界） |
| `domain-world` | `spark:ui@0.1.0` | **前后端统一的域组件**，能力显式引入（P2 起） |

---

## P1 · 一个 Component，跑两种可信宿主（**已完成**）

**目标**：一个 headless Button 域组件，同一份 `button.wasm`，跑通 Rust 后端 + Web；RN 做 spike。

| # | 结果 | 判据 | 状态 |
| --- | --- | --- | --- |
| ① | `button.wasm` | 一次构建产出的唯一 Component artifact | ✅ |
| ② | **Rust Backend PASS** | 同一 wasm → wasmtime 宿主，点 3 次 → count=3 | ✅ |
| ③ | **Web Browser PASS** | 同一 wasm → jco → React，浏览器现场点击 count 递增、状态在组件里 | ✅ |
| ④ | RN Hermes | 如实结论 | ⚠️ 编译门 PASS / 运行时未验证，见 `hosts/rn/README.md` |

**证据链**：`wit/ui.wit` → **一次** `cargo component build` → `button.wasm` →
{Rust 宿主（wasmtime）, Web 宿主（jco → ESM）}。jco 产物是 **Host 的适配产物**，
**不是**第二个 Component。

P1 明确不做：不 import 任何 capability、不做多组件、不给 Button 加 UI 概念、
不动 `plugin-world` 与 6 个插件、不做 Registry、不接 LLM。

---

## P2 · Capability Import

给 `spark:ui` 加 `import storage`（`get/set/remove`），验证「组件不碰存储本身，只说存这个键」，
而存储实现由各 Host 提供：Rust = HashMap/Redis，Web = localStorage，RN = MMKV。

已知待决：**`AsyncStorage` 是异步的，与同步 `storage` 接口不兼容** —— 需在 MMKV（同步）、
写穿缓存、或把 WIT 改 async 之间做选择。这是 P2 的第一个决策点，不要拖到实现中途才发现。

## P3 · 跨端 Domain Components

把 Button 扩成真正成体系的域组件；验证组件间组合与前后端一致的行为。
若要自动化 Web 验收，此时引入 Playwright。

## P4 · Component Registry

组件的发现 / 版本 / 分发。注意：不要靠提交 `hosts/web/src/generated/` 来分发 ——
那是从契约派生的产物。正确做法是发布 npm 制品或 CI 上传构建产物。

## P5 · AI Agent（**冻结中**）

`spark-host/src/agent.rs` 与 `spark-host/src/deepseek.rs` 已加模块级冻结注释：
代码保留、测试保持通过、**不再迭代**。`spark-host agent` 子命令仍可用，但帮助文本已标注属未来层。

**冻结理由**：Agent 不该是架构核心，它只是**另一种 Component Consumer** ——
与 Web 宿主、Rust 后端宿主同层。先把「同一份契约 + 同一个组件服务多端」证明扎实，
接 Agent 会自然得多。

**解冻条件**：P0–P4 的跨端契约有真实使用方，或确实需要 Agent 作为组件的消费者。
