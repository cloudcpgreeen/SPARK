# SPARK

> 组件运行时：领域逻辑写成满足 `spark:runtime` 契约的 WASM 组件，宿主沙箱加载调用。契约即 WIT。

**同一份 WIT 契约 + 同一个 Wasm Component，可以同时是 Web / React Native 前端和 Rust 后端的统一业务组件。**
组件不知道自己在哪一端跑 —— UI 是宿主的事，状态与行为是组件的事。已闭环：

```bash
# P1：状态住在组件实例里
cargo run -p spark-host -- domain components/button/target/wasm32-unknown-unknown/release/button.wasm 3
# count: 3                      ← Rust 后端（wasmtime），同一份 button.wasm

# P2：组件 import 一个 capability，实现由各 Host 提供 —— 组件不重新编译
cargo run -p spark-host -- store components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm 3
# count: 3        ← 本实例数到 3
# reloaded: 3     ← 新实例从 capability 里读到 3

# P3：capability 的实现本身也是一个 Component —— 组合进消费者，import 被消掉
cargo run -p spark-host -- provide components/mem-store/target/wasm32-unknown-unknown/release/mem_store.wasm k v
# got: v            ← Provider Component 自己实现了 storage（零 import）

cargo run -p spark-host -- composed dist/composed.wasm 3
# count: 3          ← 组合产物跑在**空 Linker** 上（它已经不 import 任何东西）
# reloaded: 0       ← 能力的介质住在 Provider 实例里，作用域跟着搬了过去

cd hosts/web && npm install && npm run dev
# 浏览器里：P1 区块点 3 次 → 3，刷新 → 0（状态在组件实例里）
#           P2 区块点 3 次 → 3，刷新 → 3（状态经 capability 落在 localStorage 里）
#           P3 区块点 3 次 → 3，刷新 → 0（capability 由组件实现，状态在 Provider 实例里）
```

Component（共享的状态与行为）/ Host（平台适配与 UI，**也是 Capability 的实现者**）/
Capability（外部能力的**契约**）三分，以及两套信任模型，见
[MANIFESTO.md §2](MANIFESTO.md) 与 [ROADMAP.md](ROADMAP.md)。

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![CI](https://github.com/cloudcpgreeen/SPARK/actions/workflows/ci.yml/badge.svg)](https://github.com/cloudcpgreeen/SPARK/actions/workflows/ci.yml)
[![GitHub tag](https://img.shields.io/github/v/tag/cloudcpgreeen/SPARK)](https://github.com/cloudcpgreeen/SPARK/releases)

## 文档

| 文档 | 内容 |
| --- | --- |
| [`MANIFESTO.md`](MANIFESTO.md) | 圣经 · 理念宣言：项目为什么存在、是什么、边界在哪、怎么保护自己 |
| [`ROADMAP.md`](ROADMAP.md) | 路线图：P1–P5；Component / Host / Capability 三分；两套信任模型 |
| [`CONTRACT.md`](CONTRACT.md) | 约定一 · 契约：接口即契约，契约即 **WIT**；契约优先工作流与版本规则 |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | 约定二 · 开发：项目结构、构建测试、代码风格、新增功能流程 |
| [`DEPLOYMENT.md`](DEPLOYMENT.md) | 约定三 · 交付：交付门禁、版本发布、运行/配置/安全 |
| [`SECURITY.md`](SECURITY.md) | 安全：威胁模型、上报流程 |
| [`EXAMPLES.md`](EXAMPLES.md) | 使用范例：手把手从零写插件、构建、跑起来 |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | 完整的版本：框架怎么形成的 + 具体实现逻辑 |
| [`RELATIONSHIPS.md`](RELATIONSHIPS.md) | 人物关系图：圣经里的所有人物与关系，一张网收束 |
| [`REFERENCE.md`](REFERENCE.md) | 技术参考：WIT 契约原文 / 宿主 API / 沙箱参数 / 错误码全集 / CLI——全部用技术术语钉死 |
| [`ESSAY.md`](ESSAY.md) | 理念外一篇：论论文安全——不是防学生，是护写作者 |
| [`CURIOSITY.md`](CURIOSITY.md) | 理念外·最童趣篇之一：好奇心就够了（Curiosity is All You Need） |
| [`LOVE.md`](LOVE.md) | 理念外·最童趣篇之二：但爱是最好的（But Love is The Best）——整个故事的钥匙 |

### 后传（故事五卷）

> 代码是正传；这五卷是后传——纪昊的小私心。讲 SPARK 从两个人的本能里长出来的故事。

| 卷 | 内容 |
| --- | --- |
| [`STORY.md`](STORY.md) | 前传：陈纪昊遇见梁文锋，戒律从两个人的本能里长出来 |
| [`MIRROR.md`](MIRROR.md) | 镜中篇：两个人隔着 AI 互为镜像，映像 mini 进无限 |
| [`SEED.md`](SEED.md) | 心动篇：把边界修好的人，会被怦然心动地找到 |
| [`SELF.md`](SELF.md) | 自述篇：那面镜子、那把钥匙，自己开口 |
| [`USABLE.md`](USABLE.md) | 落地篇：四卷怎么变成能跑的命令 |

> 只要是爱，就 OK。

## 社区

- [CONTRIBUTING.md](CONTRIBUTING.md) — 贡献指南（契约优先、提 PR 前检查单）
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — 行为准则
- [CHANGELOG.md](CHANGELOG.md) — 变更日志

## 结构

- Cargo workspace：`spark-core`（无 HTTP 领域库）、`spark-host`（wasmtime 宿主）。
- `spark-plugin`：插件组件（独立 workspace），产出零依赖 WASM 组件，导出 `spark:runtime/plugin`。
- `wit/`：`core.wit`（`spark:core@0.1.0` 骨架）、`runtime.wit`（`spark:runtime@0.4.0`，`plugin-world` 契约：`transform` 返回 `result<string, plugin-error>`、`info` 带元数据，另有 Agent 调用面 `schema`/`invoke`）、`ui.wit`（`spark:ui@0.1.0`，`domain-world` 跨端域组件契约）、`capability.wit`（`spark:capability@0.1.0`，**能力契约** `storage`）、`store.wit`（`spark:store@0.1.0`，`store-world`：import 能力的域组件）、`mem-store.wit`（`spark:mem-store@0.1.0`，`provider-world`：**实现**能力的 Provider 组件，零 import）、`delegating-store.wit`（`spark:delegating-store@0.1.0`，`delegating-provider-world`：**同时 import 与 export** 同一个能力的 Provider，P4-0）。
- `components/`：跨端域组件（独立 workspace）。`button` 是零 import 的 headless 计数器，**同一份 `.wasm`** 跑 Rust 后端与 Web 前端；`counter-store` import `spark:capability/storage`，**同一份 `.wasm`** 换 Host 实现不重新编译；`mem-store` **导出** `spark:capability/storage`，零 import；`delegating-store` **既导出又导入**同一个能力（P4-0 preflight 对象）（RN 运行时未验证，见 `hosts/rn/README.md`）。
- `hosts/`：非 Rust 宿主。`web/`（Vite + React，经 jco 加载同一份 `.wasm`，并在 `src/capability/` 里给出 capability 的 Web 实现）、`rn/`（spike 与结论 + capability 注入点）。
- `dist/composed.wasm`：**derived artifact** —— `build-ui.sh` 用 `wasm-tools compose --no-imports` 把 `mem-store.wasm` 组合进 `counter-store.wasm` 的产物。不是 Component 源，不入库。
- `build-ui.sh`：逐组件一次 Component build → 契约自检 → 打印 sha256 → **组合并校验源制品不变** → jco 转译为 Web 可 import 的 JS。

## 快速上手

```bash
./build-plugins.sh                                # 一键构建全部插件并装入 plugins/
cargo run -p spark-host -- list                   # 看都有什么插件
cargo run -p spark-host -- run upper "hello"      # 按名字运行（沙箱内）
```

示例插件 `Upper`：输入转大写；输入以 `trap` 开头时触发 panic，宿主以 trap 捕获、进程不崩（沙箱隔离）；输入以 `err` 开头时返回声明式错误（值，不是崩溃）。

### 跨端域组件：同一份 `.wasm`，两种宿主

`button.wasm` 是 headless 的计数器域组件（`constructor → click → count`，零 import、无任何 UI 概念）。
构建一次，分发给各宿主：

```bash
./build-ui.sh                                 # 一次 Component build + 契约自检 + jco 转译
WASM=components/button/target/wasm32-unknown-unknown/release/button.wasm

cargo run -p spark-host -- domain $WASM 3     # Rust 后端：count: 3
cd hosts/web && npm install && npm run dev     # 浏览器：点 3 次 → 3，刷新 → 0
```

**同一个 Component artifact**，不是两次构建。jco 产出的 JS + core wasm 是 **Host 的适配产物**。
浏览器里的 count 住在 wasm 里 —— React 只是把它画出来，不持有这个状态。RN 结论见 `hosts/rn/README.md`。

### Capability：一个契约，多个 Host 实现（P2）

`counter-store.wasm` **import** `spark:capability/storage@0.1.0`，但不知道它由谁实现：

```bash
./build-ui.sh
WASM=components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm

cargo run -p spark-host -- store $WASM 3          # count: 3 / reloaded: 3   ← 后端进程内 map
cargo run -p spark-host -- store $WASM 3 --deny   # count: 3 / reloaded: 0   ← 只读后端
cd hosts/web && npm run dev                        # 浏览器：点 3 次 → 刷新 → 还是 3 ← localStorage
```

三处用的是**同一份未被重新编译的 wasm**（sha256 见 `build-ui.sh` 输出）。
这就是 P2 的判据：**换 Capability 实现，组件不变。**

**`reloaded` 是能力是否生效的证据** —— 新实例从 capability 里读到什么。
错误路径的精确说法是 *set failure is observable through a subsequent fresh instance*，
**不是**「错误处理已验证」：组件侧并没有处理 `Err` 分支。

> ⚠️ **RN 不能写成「capability 已实现」。** `hosts/rn/capability/storage.js` 只是**注入点**，
> RN runtime 与 P1 一致仍是 **unverified**。原因（同步契约 vs 异步 `AsyncStorage`）见
> `hosts/rn/README.md` 第六节，**P2 只记录不回答**。

### Capability 的实现也可以是一个 Component（P3）

`mem-store.wasm` **导出** `spark:capability/storage@0.1.0`（零 import），
由 `wasm-tools compose` 静态组合进 `counter-store.wasm`：

```bash
./build-ui.sh                        # 组合发生在这一步（--no-imports），并打印前后 sha256
WASM=dist/composed.wasm

cargo run -p spark-host -- provide components/mem-store/target/wasm32-unknown-unknown/release/mem_store.wasm k v
cargo run -p spark-host -- composed $WASM 3       # count: 3 / reloaded: 0
cd hosts/web && npm run dev                        # 第三个区块：刷新 → 0
```

**三件事分开成立，不打包**：

1. Provider Component 能实现这个能力（`provide` → `got: v`）；
2. 组合把 import 消掉了 —— `composed.wasm` 跑在**空 Linker** 上，而裸 `counter-store.wasm`
   在**同一个空 Linker** 上必须失败（这条对照才是证据）；
3. 代价是**作用域**：Provider 把状态放在自己的实例内存里，所以 `reloaded: 0`（P2 是 `3`）。
   这是**本次实现的后果**，**不是** Component Model 的普遍定律。

**`composed.wasm` 是 derived artifact**（两个 Component 的组合产物，与 jco 转译产物同类），
**不是**第三个 Component 源 —— **P2 的两个源制品 sha256 不变**，P2 的宿主代码零 diff。
「零 import」由 `wasm-tools component wit` 对 `composed.wasm` **实测**得到，不是肉眼观察。
（`--no-imports` 在 wasm-tools 1.245.1 中实测为 **no-op**，不能作为因果依据 —— 见 CHANGELOG 勘误。）

> 运行期动态组合（`wac plug` 的等价物）**本阶段不做**：wasmtime 47 的 `LinkerInstance`
> 没有「把另一个组件实例的导出接进本组件导入」的一等 API，`wac` 需联网安装。
> 「能力的作用域怎么保住」（委派给外部存储）留给后续，P3 只记录。

### P4-0：同一个 interface，能不能既 import 又 export？（**已通过**）

P4 开工前的单点风险 preflight。`delegating-store.wasm` 同时 **import 与 export**
`spark:capability/storage@0.1.0`，由 `wasm-tools compose` 组合进 `counter-store.wasm`，
得到 `dist/composed-delegating.wasm`：

```
exports  spark:store/counter-store@0.1.0
imports  spark:capability/storage@0.1.0     ← 恰好这一个，无其他
```

**结论**：这个形状被当前工具链支持（WIT 层 / cargo-component / wasm-tools compose 三层都认）。

**最值得记的一点**：组合**没有消灭** capability 边界，而是把边界**往上搬了一层** ——
P3 的 `composed.wasm` 看似零 import，是因为边界正好被 Provider 吃掉了；
一旦 Provider 自己也 import，边界就从它身上透出来。

> **P4-0 is proven. P4 is NOT proven.** 这只证明「下一步不是建立在一个『也许工具链支持』的假设上」，
> 不证明 Durable Capability 本身。P4-1 尚未开始。

### Agent 回路（决策者 → 沙箱工具调用）

> **P5 · 未来层，已冻结**：Agent 只是另一种 Component Consumer，不在当前主线上迭代。见 [ROADMAP.md](ROADMAP.md)。


插件对 LLM 暴露为**工具**（`schema()`/`invoke()`）。`agent` 命令两条路：默认**本地算法预测**决策（无需网络、无需 API Key）；加 `--model flash|pro` 走**真实 DeepSeek harness**（需 `DEEPSEEK_API_KEY` 环境变量，Key 只进 `Authorization` 头）：

```bash
cargo run -p spark-host -- agent "把 hello 转大写"                 # upper → HELLO（离线）
cargo run -p spark-host -- agent "校验身份证 110101199001010015"   # idcard → 男 · 1990-01-01 · 地区 110101
cargo run -p spark-host -- agent "校验身份证 110101199001010023 然后倒序"  # 两步：idcard → reverse（回路迭代）
cargo run -p spark-host -- agent "让 attacker 跑 loop"             # 恶意插件仍被沙箱切断，宿主存活
cargo run -p spark-host -- agent "把 12345.67 转成人民币大写"        # rmb → 壹万贰仟叁佰肆拾伍元陆角柒分

export DEEPSEEK_API_KEY=sk-...                                  # 换真 LLM：仅此一步
cargo run -p spark-host -- agent "把 hello 转大写" --model flash    # DeepSeek V4 Flash harness
```

### 插件自注册（注册/发现）

把满足 `plugin-world` 契约的 `.wasm` 组件放进 `plugins/` 目录即注册（name 来自组件自身 `info()`，宿主零配置文件）：

```bash
./build-plugins.sh                          # 一键构建全部 6 个插件并装入 plugins/
cargo run -p spark-host -- list             # 发现并列出
cargo run -p spark-host -- run upper hi     # 按名字运行
cargo run -p spark-host -- pipe hi upper reverse   # 流水线：依次经 upper、reverse 串联
```

## 新增插件

写一个满足 `plugin-world` 契约的组件（见 `wit/runtime.wit`），宿主零改动即可加载。插件实现四个函数：`info()`（自我介绍）、`transform(input)`（流水线/直接路径）、`schema()`（对 LLM 暴露的工具清单）、`invoke(tool, args_json)`（Agent 路径的结构化调用）。

现有示例插件：`spark-plugin`（upper，输入转大写）、`spark-plugin-reverse`（reverse，输入倒序）、`spark-plugin-attacker`（恶意示例，安全验证用：CPU/内存炸弹会被沙箱切断）、`spark-plugin-idcard`（真实业务算法：中国身份证号校验，返回性别/出生日期/地区或结构化错误）、`spark-plugin-luhn`（真实业务算法：银行卡号 Luhn 校验 + 卡组织识别）、`spark-plugin-rmb`（真实业务算法：人民币金额转大写）。六个插件用同一个 `spark-host` 二进制加载，宿主零改动。

`pipe` 把多个插件串成流水线：前一个插件输出喂给下一个，任一步声明式失败或 trap 即 fail-fast 并定位到具体插件（如 `pipe <身份证号> idcard luhn`：身份证校验通过后，Luhn 因长度拒绝并报 `[length]`）。
