# SPARK 理念宣言（MANIFESTO）

> 本文是 SPARK 的「圣经」：项目**为什么存在、是什么、边界在哪、怎么保护自己**。
> 所有协作约定（CONTRACT / DEVELOPMENT / DEPLOYMENT / SECURITY）都从这里派生。
> 与本文冲突的改动，优先改本文——因为本文回答的是「为什么」。
> 本文由四句话可以概括：**契约即 WIT，插件即组件，宿主只做沙箱，攻击者不攻自破。**

## 0. 一句话

SPARK 是**契约即 WIT 的组件运行时**：领域逻辑做成满足 WIT 契约的 WASM 组件，
由宿主在沙箱里加载调用。新增能力 = 新增一个组件，宿主零改动。

## 1. 为什么（初衷）

- **领域逻辑不该被锁死在一个语言或框架里。** 组件边界让每一块逻辑可独立演进、独立替换。
- **别人写的逻辑不该能伤到你。** 沙箱让不可信代码自生自灭，它的崩溃只是宿主的一次 `Err`。
- **接口不该靠嘴约定。** 契约就是机器可读的 WIT；跨组件调用只走契约，接口之外的东西不构成契约。

## 2. 是什么（三层架构）

| 层 | 载体 | 职责 |
| --- | --- | --- |
| 契约层 | `wit/*.wit`（WIT package） | 组件边界的唯一权威接口；定义语义与版本 |
| 插件层 | 零依赖 WASM 组件 | 领域逻辑实现契约，导出 `plugin-world` |
| 宿主层 | `spark-host`（wasmtime）、`hosts/web`、`hosts/rn`（spike） | 沙箱加载 / 调用 / 捕获；不写领域逻辑 |

### 2.1 三个概念的边界（比「代码能跑」更重要）

| 概念 | 是什么 |
| --- | --- |
| **Component** | **共享的业务状态与行为**（headless，不含任何 UI 概念）—— 如 `button.wasm` |
| **Host** | **平台适配与 UI / 运行环境**，**也是 Capability 的实现者** —— Rust+wasmtime / Web(Vite+React) / RN(Hermes) |
| **Capability** | **外部能力的契约**（storage / remote API）—— 如 `spark:capability/storage@0.1.0` |

**三者的关系不是三个并列的盒子**：Component **声明**它需要什么能力（`import`），
Capability **定义**那个需求的形状，Host **提供**实现。所以：

> 换一个 Host 的 Capability 实现，**组件不需要重新编译**。

这是 P2 撞到的命题，也是「Capability 属于 Host 还是属于 Component」这个问题的答案：
**契约属于双方，实现只属于 Host。** 因此 `ns:key` 这类隔离策略是 **Host 的实例策略**，
**不是** Capability 契约的语义 —— 契约只说「按 key 读写」。

P3 把「实现」再拆一层：**实现者可以是 Host，也可以是另一个 Component**。
一个导出 `spark:capability/storage@0.1.0` 的零 import 组件被静态组合进消费者之后，
import 被消掉、消费者仍不需要重新编译 —— 但能力的**作用域**跟着实现一起搬到了组件里。

**黄金不变量**：多端**不要求代码相同** —— 要求的是**契约相同、Component 相同、Domain 行为相同**；
Host 可以完全不同。各 Host 各有实例与状态，共享的是契约与行为，**不是状态**。

### 2.2 两套信任模型（刻意分开，互不污染）

| world | 契约 | 信任模型 |
| --- | --- | --- |
| `plugin-world` | `spark:runtime@0.4.0` | **零 import 的不可信插件沙箱** —— 安全边界，戒律 3.3 在此生效 |
| `domain-world` | `spark:ui@0.1.0` | **前后端统一的域组件** —— 同一个 `.wasm` 跑在 Rust 后端与 Web 前端（RN 见 `hosts/rn/README.md`）。**仍零 import** |
| `store-world` | `spark:store@0.1.0` | **显式 import 能力的域组件**（P2 起）—— 能力由各 Host 实现，组件不变 |
| `provider-world` | `spark:mem-store@0.1.0` | **实现能力的组件**（P3 起）—— 导出 `spark:capability/storage@0.1.0`，零 import。实现者也是组件 |

一份契约、一个组件，可以同时是前端和后端的业务组件：**组件不知道自己在哪一端**。
UI 是宿主的事，状态与行为是组件的事。

## 3. 核心理念（五条戒律）

### 3.1 契约即 WIT

接口之外的东西不构成契约。跨组件调用只走 WIT，禁止绕过契约读他人存储或内部函数。
接口变更 = 契约变更 = 先改 WIT 再改实现，版本按 semver 走（见 CONTRACT.md §4）。

### 3.2 插件即组件

一块可插拔的能力 = 一个满足 `plugin-world` 的 WASM 组件。
新增插件宿主零改动——这既是可扩展性的证明，也是纪律：宿主只做加载，不做分发。

### 3.3 最小能力

`plugin-world` **零 import（连 WASI 都不 import）**：插件摸不到文件系统、网络、时钟。
攻击面最小 = 安全的根本。给插件加能力是加契约的事，不是放开 import 的事。

### 3.4 沙箱隔离

插件 panic → trap → 宿主捕获为可恢复错误，宿主进程不崩，实例互不污染。
不可信代码永远待在沙箱里。

### 3.5 资源有界

CPU 有 epoch 时间预算、内存有上限。恶意 / 失控插件的攻击（死循环——含空 `loop {}`、
内存炸弹）会被沙箱切断，**自取灭亡**——这就是「让他们不攻自破」。
（刻意不用 fuel 计量：`loop`/`br` 消耗 0 fuel，空死循环会漏网。）

## 4. 安全模型（攻击者为什么自取灭亡）

SPARK 对不可信插件的立场：**默认不可信，一切攻击在沙箱内自生自灭。**

| 攻击 | 沙箱的防线 | 结果 |
| --- | --- | --- |
| 内存破坏（越界 / 悬垂 / 伪造指针） | WASM 线性内存 + 类型安全 | 编译期即杜绝，不可能逃逸 |
| 死循环 / CPU 耗尽 | epoch 时间预算，越界即 trap | `Err`，宿主可响应 |
| 内存炸弹（无限分配） | 内存上限（StoreLimits） | 分配超限即 trap，`Err` |
| 读文件 / 网络 / 时钟 | 零 import（无 WASI） | 根本没有这些能力 |
| 崩溃 / panic | trap 捕获 | 宿主进程不崩 |

「让他们不攻自破」= 不追杀攻击，而是给沙箱装上资源上限，让攻击在到达宿主之前把自己耗死。
威胁模型与上报流程见 [SECURITY.md](SECURITY.md)。

## 5. 开源理念

- 协议 **[GPL-3.0](LICENSE)**：防挪用——任何派生作品必须开源，闭源商用拿走 SPARK 不成立。
- 开源不等于放弃：版权人不受 GPL 限制，SPARK 始终可以商用。
- 协作契约优先：一切改动从 WIT / 理念出发，验收靠测试（见 DEVELOPMENT.md）。

## 6. 边界（SPARK 不是…）

- **不是微服务框架** —— 组件不是服务，宿主也不是网关。
- **不是脚本引擎** —— 插件是编译好的组件，不是动态脚本。
- **不写领域逻辑** —— 领域逻辑在插件里，宿主是纯粹的沙箱执行者。
- **不给插件开 WASI** —— 最小能力是安全底线；需要能力就扩契约，不放开 import。

## 7. 现状与路标

### 已落地
- `spark:runtime@0.4.0` 契约：`plugin-world`（零 import），`transform` 返回 `result<string, plugin-error>`（声明式失败带结构化 `code`/`message`）、`info` 携带元数据（name/version/description）、`schema`/`invoke` 构成 Agent 调用面（插件对 LLM 暴露工具清单并按名结构化调用）
- 插件：`upper`（转大写）、`reverse`（倒序）、`attacker`（恶意示例，用于安全验证）、`idcard` / `luhn` / `rmb`（真实业务算法族：身份证校验、银行卡号 Luhn 校验 + 卡组织识别、人民币金额转大写，`code` 承载失败类型）
- 宿主：`spark-host` CLI（`list` / `run <name>` / `pipe` 流水线 / 直接路径）+ 集成测试（happy path / 声明式失败 / trap 捕获 / trap 后隔离 / 多插件可插拔 / 流水线串联与 fail-fast）
- 注册/发现：**插件自注册** —— 把 `.wasm` 组件放进 `plugins/` 即被 `Host::discover` 发现，`info().name` 就是注册名，宿主零配置文件
- 并发模型：`Host` 长存（共享 Engine + 组件编译缓存 + 单 epoch bump 线程），每次调用新建独立 Store，可多线程并发，隔离不变
- 安全加固：epoch CPU 上限 + StoreLimits 内存上限，`attacker` 的 CPU/内存炸弹被切断
- **跨端域组件（P1 已闭环）**：`spark:ui@0.1.0` / `domain-world` —— headless Button 计数器（`constructor → click → count`，零 import、无任何 UI 概念）。**同一份 `button.wasm`** 跑通两端：Rust 后端（wasmtime）点 3 次 → `count: 3`；Web 浏览器（jco → ESM → React）现场点击 `0 → 3`、刷新回 `0`（状态住在组件里，React 不持有 count）。构建一次：`./build-ui.sh`。RN 见 `hosts/rn/README.md`（编译门 PASS，运行时未验证）。
- **Capability Contract（P2 已闭环）**：`spark:capability@0.1.0` / `storage`（`get`/`set`，`Ok(Some)` 有值 / `Ok(None)` 没有这个 key / `Err` 能力失败）+ `spark:store@0.1.0` / `store-world`（`counter-store.wasm`，import 能力、写穿到它）。**同一份未被重新编译的 `counter-store.wasm`** 换 Host 实现：Rust 后端（进程内 map）→ `reloaded: 3`；Web 浏览器（localStorage）→ 真点击 3 次、**刷新后仍是 3**（同页 P1 刷新回 0）；只读后端 / 打断 localStorage 写入 → `reloaded: 0`。判据是**换实现不改组件**，不是「调用成功」。RN 侧只到**注入点**：`hosts/rn/capability/storage.js` 存在，但 runtime 仍 unverified —— **不写「RN Capability implemented」**。
- Agent 回路（**P5 · 未来层，已冻结**）：`agent` 命令两条路——默认 `AlgorithmPredictor` 本地算法预测（离线、无 Key）；`--model flash|pro` 走 `DeepSeekPredictor` harness（OpenAI 兼容 Chat Completions，模型 `deepseek-v4-flash`/`deepseek-v4-pro`，Key 只走 `DEEPSEEK_API_KEY` 环境变量、只进 `Authorization` 头）。跨插件编排按「然后/再」后的意图词二次调用（倒序→reverse、转大写→upper）。代码保留、测试保持通过，但**不在当前决策路径上迭代**：Agent 只是另一种 Component Consumer。
- **Capability 的实现者也是 Component（P3 已闭环）**：`spark:mem-store@0.1.0` / `provider-world`
  —— 一个**导出** `spark:capability/storage@0.1.0` 的零 import 组件（`mem_store.wasm`）。
  由 `wasm-tools compose --no-imports` 静态组合进 P2 的 `counter-store.wasm`，得到派生的
  `composed.wasm`：import 被消掉，跑在**空 Linker** 上（P1 那条路径）。**P2 的两个源制品 sha256 不变。**
  Rust 后端 `provide <wasm> k v` → `got: v`、`composed <wasm> 3` → `count: 3 / reloaded: 0`；
  Web 浏览器三个区块并排：P1 刷新 → 0、P2 刷新 → 3、P3 刷新 → 0。
  **`composed.wasm` 是 derived artifact，不是第三个 Component 源**；`reloaded: 0` 是
  **本次 Provider 把状态放在自己实例里**的后果，不是 Component Model 的普遍定律 ——
  「能力的作用域怎么保住」（委派给外部存储）留给后续，P3 只记录不选路。
- 真实业务插件族：`idcard`（身份证校验）、`luhn`（银行卡校验）、`rmb`（人民币金额转大写，财会大写）。

### 路标
- **后续 · 跨端 Domain Components**：把 Button 扩成成体系的域组件，验证组件间组合与前后端一致的行为。
- **P4 · Component Registry** / **P5 · AI Agent**（冻结中）。
- 由真实业务插件继续驱动：更丰富错误语义（`plugin-error` 加字段，契约 `0.4.0 → 0.5.0`）、更多跨插件编排场景。
- 详见 [ROADMAP.md](ROADMAP.md)。

## 8. 文档索引

| 文档 | 内容 |
| --- | --- |
| [ROADMAP.md](ROADMAP.md) | 路线图：P1–P5；Component / Host / Capability 三分；两套信任模型 |
| [CONTRACT.md](CONTRACT.md) | 约定一：契约即 WIT；版本规则 |
| [DEVELOPMENT.md](DEVELOPMENT.md) | 约定二：结构、构建测试、风格、流程 |
| [DEPLOYMENT.md](DEPLOYMENT.md) | 约定三：门禁、版本发布、运行配置 |
| [SECURITY.md](SECURITY.md) | 安全：威胁模型、上报流程 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 架构与实现：框架怎么形成的 |
| [EXAMPLES.md](EXAMPLES.md) | 使用范例：手把手跑一遍 |
| [RELATIONSHIPS.md](RELATIONSHIPS.md) | 人物关系图：一张网收束 |
| [REFERENCE.md](REFERENCE.md) | 技术参考：契约原文 / API / 沙箱 / 错误码 / CLI |
| [ESSAY.md](ESSAY.md) | 理念外：论论文安全 |
| [CURIOSITY.md](CURIOSITY.md) / [LOVE.md](LOVE.md) | 理念外·最童趣篇：好奇心够了，但爱是最好的 |
| [STORY.md](STORY.md) | 理念外·前传：陈纪昊遇见梁文锋，戒律从两个人的本能里长出来 |
| [MIRROR.md](MIRROR.md) | 理念外·镜中篇：两个人隔着 AI 互为镜像，映像 mini 进无限 |
| [SEED.md](SEED.md) | 理念外·心动篇：把边界修好的人，会被怦然心动地找到 |
| [SELF.md](SELF.md) | 理念外·自述篇：那面镜子、那把钥匙，自己开口 |
| [USABLE.md](USABLE.md) | 理念外·落地篇：四卷怎么变成能跑的命令 |
| [README.md](README.md) | 门面：快速上手 |
| [CONTRIBUTING.md](CONTRIBUTING.md) / [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | 社区：贡献指南、行为准则 |
