# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)。

## [未发布]

## P3 · Capability 的实现也可以是一个 Component

> 命题：一个 Capability 的**实现者**，能不能本身也是一个 Component？
> P1 = `Component → 多个 Host`；P2 = `Component → Capability Contract → 多个 Host Implementation`；
> P3 = `Component → Capability Contract ← Component`。

### 新增

- **`wit/mem-store.wit`**（`spark:mem-store@0.1.0` / `provider-world`）：`export spark:capability/storage@0.1.0`，**零 import**。
  **没有重定义 `storage`** —— 接口身份必须与 `spark:capability@0.1.0` 逐字相同，否则组合接不上。
  也**没有 bump `spark:capability`**：那会让 P2 的 `counter-store.wasm` 变成孤儿。
- **`components/mem-store`**：Provider 组件，`RefCell` → 实际是实例级 `thread_local!` map
  （`storage` 是**按值导出**的接口，`Guest` 方法是静态的、拿不到 `&self`）。
- **`spark-host/src/compose.rs`**：`bindgen!({ path: "wit-provider", world: "provider-world" })` +
  `storage_roundtrip`（①）与 `click_times_selfcontained`（②③，**空 Linker**）。
- **CLI**：`provide <mem-store.wasm> <key> <value>`、`composed <composed.wasm> <n>`。
- **`build-ui.sh`**：构建 `mem-store` → 打印组合**前后** `counter-store.wasm` 的 sha256 并**断言相等** →
  `wasm-tools compose --no-imports -o dist/composed.wasm` → `jco transpile`（**这一行没有 `--map`**，
  它的缺席就是证据：组合产物没有 import 要注入）。
- **`hosts/web/src/App.tsx`** 第三个区块（P2 区块未动）：三个区块并排 = 三种状态归属。

### 验收（P3 交付判据）

| # | 结果 | 判据 |
| --- | --- | --- |
| ① | Provider Component 能实现能力 | `mem_store.wasm` 零 import（`wasm-tools component wit`）；`provide <wasm> k v` → `got: v` |
| ② | 组合消除了 import | `composed.wasm` 在**空 Linker** 上 `composed <wasm> 3` → `count: 3` |
| ② 对照 | 这条路真的是空的 | 裸 `counter-store.wasm` 在**同一个空 Linker** 上 trap：`imports instance spark:capability/storage@0.1.0, but a matching implementation was not found in the linker` |
| ③ | 代价是作用域 | 同一份 `composed.wasm` → `reloaded: 0`，对照 P2 的 `3` |
| ④ | 两个源制品不动 | `counter-store.wasm` sha256 仍 `85691b8e…`；P2 的 `domain_store.rs` / `Backend` 零 diff |
| ⑤ | 零 import 由工具强制 | `wasm-tools compose --no-imports` 通过，不是我肉眼观察的 |
| ⑥ | P2 完好 | 42 个测试全绿（38 原 + 4 新增），`cargo fmt --check` 与 `cargo clippy --workspace --all-targets` 干净 |
| ⑦ | Web | 真实 Chrome：P1 刷新 → `0`、P2 刷新 → `3`、P3 刷新 → **`0`**；转译无 `--map` |

**三件事分开成立，不打包成一个结论**：① 组件能实现能力／② 组合消掉了 import／③ 代价是作用域。
② 必须有**对照**（裸消费者在同一空 Linker 上失败），否则「这条路是空的」没被证明。

**③ 的措辞必须是 implementation consequence，不是 Component Model law**：
*在本次实现中*，Provider 的状态位于 Provider Component instance 内，因此组合后的能力状态具有
**instance scope**。换一个委派给外部存储的 Provider，作用域会不同 ——
「能力的作用域怎么保住」是**后续**的问题，P3 只记录不选路。

**`composed.wasm` 是 derived artifact**（两个 Component 的组合产物，与 jco 转译产物同类），
**不是**第三个 Component 源。「P2 的 `counter-store.wasm` 零重新编译」与「P1 的 `button.wasm` 字节不变」
是两条不同的证明，不许混。

### 变更

- `.gitignore` 追加 `dist/`（组合产物是构建产物，不是源）。
- `CONTRACT.md` §2/§5、`ROADMAP.md`、`README.md`、`REFERENCE.md` §1.5/§2.5/§6、`MANIFESTO.md` §2.1/§2.2/§7、
  `DEVELOPMENT.md` 的构建流程登记 P3。

### 测试

- 新增 `spark-host/tests/compose.rs`（4 个）：Provider 回环 / 组合产物跑通空 Linker /
  **对照组裸消费者必须失败** / 作用域是实例级。
- 合计 42 个测试全绿（38 原测试未改 + 4 新增），fmt 与 clippy 干净。

### P3 明确不做（如实记录）

- **不做运行期动态组合**：wasmtime 47 的 `LinkerInstance` 只有 `func_wrap` / `func_new` / `module` / `resource`，
  **没有**「把另一个组件实例的导出接进本组件导入」的一等 API；`wac` 需联网安装，本机不可用。
  因此 `wasm-tools compose`（已废弃，提示改用 `wac`）是唯一可行的静态组合路径。
- 不回答「能力的作用域怎么保住」（委派）；不做权限模型、不做真实 DB / 网络存储、不做 Registry。
- 不碰 `wit/capability.wit`、`wit/ui.wit`、`components/{button,counter-store}`、P2 的宿主代码、
  `plugin-world`、6 个插件、沙箱参数、Agent 代码。

### 已知粗糙处

- `wasm-tools compose` 要求**定义组件的文件名是 kebab-case**，而 cargo-component 产出 `mem_store.wasm`，
  所以 `build-ui.sh` 先 `cp` 成 `dist/mem-store.wasm` 再组合。文件名不参与接口身份。
- `storage_roundtrip` 是 set-then-get，所以它**观察不到** `Ok(None)`（缺 key）。

## P2 · Capability Contract Spike

> 命题：一个 Component **import** 的 Capability，能不能由不同 Host 提供不同实现，
> 而 **Component 本身完全不改变**？
> P1 = `Component → 多个 Host`；P2 = `Component → Capability Contract → 多个 Host Implementation`。

### 新增

- **能力契约 `wit/capability.wit`**（`spark:capability@0.1.0`）：`storage` 接口 —— `get: func(key) -> result<option<string>, store-error>` / `set: func(key, value) -> result<_, store-error>`，错误 `store-error { unavailable, denied }`。**`Ok(Some)` 有值 / `Ok(None)` 没有这个 key / `Err` 能力本身失败**，「缺失」与「失败」可区分。刻意独立成 package —— Capability 不属于 `spark:ui`。
- **用能力的域组件 `wit/store.wit`**（`spark:store@0.1.0`，世界 `store-world`）：`counter-store` 接口，`import spark:capability/storage@0.1.0`。**没有 bump `spark:ui@0.1.0`** —— 那会让 P1 的 `button.wasm`（导出 `spark:ui/button@0.1.0`）变成孤儿。
- **`components/counter-store`**：headless 计数器，状态写穿到 capability；构造时从 capability 读回。独立 cargo workspace，跨 package 依赖走 `[package.metadata.component.target.dependencies]`（cargo-component 不自动读 `deps/`）。
- **`spark-host/src/domain_store.rs`**：第二个宿主侧 capability 实现 —— `Backend`（进程内 map / `read_only`），`impl spark::capability::storage::Host`。CLI 新增 `store <wasm> <n> [--deny]`，打印 `count`（本实例数到几）与 `reloaded`（**新实例读到什么**）。
- **`hosts/web/src/capability/storage.js`**：Web 侧实现 = `localStorage`。无痕模式 / 存储被禁时 localStorage 自己就抛，天然是真实错误路径。App 新增 P2 区块，与 P1 区块同页对照。
- **`hosts/rn/capability/storage.js`**：RN 侧**注入点**（同步内存 Map）。⚠️ **标出的是「换哪一行」，不是已验证的实现** —— RN runtime 与 P1 一致，仍是 unverified。

### 验收（P2 交付判据）

| # | 结果 | 判据 |
| --- | --- | --- |
| ① | 契约 | `spark:capability@0.1.0` 独立 package；`spark:ui@0.1.0` 零改动 |
| ② | 一份 artifact | `counter-store.wasm` 一次构建，sha256 `85691b8e…` |
| ③ | **Rust Backend PASS** | 同一 wasm + 进程内 map → `store <wasm> 3` → `count: 3` / `reloaded: 3` |
| ④ | **Web Browser PASS** | 同一 wasm + localStorage → 真实 Chrome 点击 P2×3，刷新后仍是 `3`（同页 P1 刷新回 `0`） |
| ⑤ | 错误路径 | 同一 wasm + 只读后端 / 打断 localStorage 写入 → `count: 3` / `reloaded: 0` |
| ⑥ | 换实现不改组件 | ③④⑤ 用的是**同一份未被重新编译的 wasm** |
| ⑦ | P1 完好 | 35 个原测试全绿；P1 的 WIT / 组件 / 宿主代码零 diff |
| ⑧ | RN | injection point exists / **runtime unverified** |

**⑤ 的精确措辞**：*set failure is observable through a subsequent fresh instance*。
**不是**「get/set 错误处理已验证」—— 组件侧压根没处理 `Err` 分支。

**两种「不变」是两件事**：P1 的 `button.wasm` 是 **byte-for-byte 不变**（零 diff 证明）；
P2 的 `counter-store.wasm` 是 **只构建一次、换实现不重新编译**（sha256 相同证明）。

### 变更

- `spark-host/src/lib.rs`：抽出 `sandbox_limits()` 作为沙箱资源上限的**唯一来源**，`new_store` 改为调用它（签名与行为不变）。新增的第二类宿主 Store 复用它，避免安全参数漂移。
- `build-ui.sh`：构建两个组件（各一次）+ 契约自检 + **打印两个 wasm 的 sha256** + 分别转译到 `hosts/web/src/generated/{button,store}` + 把 capability 实现拷进产物目录（`jco --map` 生成的是字面相对 import）。
- `CONTRACT.md` §2/§5：登记两份新契约；写明 Capability = 契约 + 多个 Host 实现，且 **key 命名空间是 Host 的实例策略，不是契约语义**。

### 测试

- 新增 `spark-host/tests/domain_store.rs`（3 个）：状态跨实例存活 / 命名空间隔离 / 只读后端下 `Err` 可观测。
- 合计 38 个测试全绿（35 原测试未改 + 3 新增），`cargo fmt --check` 与 `cargo clippy --workspace --all-targets` 干净。

### P2 明确不做（留给 P3）

- **不回答 `AsyncStorage` 的同步/异步问题。** 契约是同步的、`AsyncStorage` 是 Promise；两条路（内存 Map + 异步落盘 / 契约改 async）都记一笔不选。`future<T>` 在本工具链实测不可用（`wasm-tools validate` 报 `future requires the component model async feature`）。该不该 async 应由实验结果决定，不是先入为主的 API 设计。
- 不做 capability 的权限/授权模型；后端实现就是进程内 map（换 DB 只动 `domain_store.rs` 一个文件，这本身就是结论）。

---

## P1 · 跨端域组件（闭环）

### 新增

- **跨端域组件（P1 闭环）**：新增第二份契约 `wit/ui.wit`（`spark:ui@0.1.0`，世界 `domain-world`）—— **刻意与 `plugin-world` 分开**，因为是两套信任模型（零 import 沙箱 vs 能力显式引入的域组件）。`plugin-world`、6 个插件与沙箱语义完全未动。
- **`components/button`**：headless Button 计数器域组件（`constructor → click → count`，零 import，无 `disabled`/`label`/`style`/`event`/`render` 等任何 UI 概念）。独立 cargo workspace，`cargo component build --release` 产出 `button.wasm`。
- **`spark-host/src/domain.rs`**：第二个 `bindgen!`（`domain-world`）+ `click_times()`，复用现有 Engine、组件编译缓存与 `new_store()` 的资源上限（内存 16 MiB + epoch 时间预算）。新增 CLI 子命令 `domain <button.wasm> <n>`。
- **`hosts/web`**：Vite + React 宿主。`build-ui.sh` 一次 Component build → 契约自检（`jco wit` 确认零 import）→ `jco transpile` 转译。**React 不持有 count** —— 它只重渲染后向组件读值。
- **`hosts/rn`** + `spike.sh`：RN spike 与如实结论（编译门 PASS / 运行时未验证）。
- **`ROADMAP.md`**：P1–P5 路线图、Component / Host / Capability 三分、两套信任模型。

### 验收（P1 交付判据）

| # | 结果 | 判据 |
| --- | --- | --- |
| ① | `button.wasm` | 一次构建产出的唯一 Component artifact |
| ② | **Rust Backend PASS** | 同一 wasm → wasmtime，`domain <wasm> 3` → `count: 3` |
| ③ | **Web Browser PASS** | 同一 wasm → jco → React，真实 Chrome 点击 `0 → 3`、刷新回 `0` |
| ④ | RN | 编译门 PASS、运行时未验证（`hosts/rn/README.md` 有失败模式与降级阶梯） |

关键证据：`wit/ui.wit` → **一次** `cargo component build` → `button.wasm` → {Rust 宿主, Web 宿主}。
jco 产出的 JS + core wasm 是 **Host 的适配产物**，不是第二个 Component。

### 变更

- **Agent 层冻结为 P5 · 未来层**：`agent.rs` / `deepseek.rs` 加模块级冻结注释，`agent` 子命令保留可用但帮助文本标注属未来层。**代码未删、33 个原测试保持全绿**。理由：Agent 只是另一种 Component Consumer，不该是架构核心。
- `spark-host/src/lib.rs` 过期注释修正（`spark:runtime@0.3.0` → `0.4.0`）。
- `.gitignore` 补 `components/*/src/bindings.rs`（原 `spark-plugin*/` 规则不覆盖新目录）与 `hosts/` 的 node_modules / 转译产物。

### 测试

- 新增 `spark-host/tests/domain.rs`：Button 域语义（点 3 次 → 3）+ 多实例互不污染。
- 合计 35 个测试全绿（33 原测试未改 + 2 新增），`cargo fmt --check` 与 `cargo clippy --workspace --all-targets` 干净。

## [1.2.0] - 2026-08-25

### 新增

- **Agent 调用面（`spark:runtime@0.3.0 → 0.4.0`）**：`plugin` 接口新增 `schema()`（对 LLM 暴露工具清单）+ `invoke(tool, args_json)`（按名结构化调用，JSON 参数）；六个插件各暴露一个同名工具（`upper`/`reverse`/`attacker`/`idcard`/`luhn`/`rmb`），`transform` 保留兼容。
- **Agent 回路 `spark-host agent`**：`Predictor` trait（决策者插入缝）+ 本地算法预测 `AlgorithmPredictor`（关键词 → 工具），跑通「决策 → 沙箱调用 → 结果喂回 → 最终答复」；工具输出按不可信数据处理（截断 `TOOL_RESULT_LIMIT`=4096）+ 迭代上限 `MAX_STEPS`=8；`--model flash|pro` 接 DeepSeek harness（见下方条目，国产 LLM，flash/pro）。
- **安全**：SECURITY.md 补 Agent 威胁模型——工具 schema/输出是**不可信数据**流入 Agent 回路（截断 + 迭代上限）；API Key 只走环境变量（`DEEPSEEK_API_KEY`），不采用 rhua-chatgpt-web 的浏览器 localStorage 存 Key 做法。
- 测试 +5（agent 回路：upper / idcard / 两步迭代 / attacker 经 agent 仍被沙箱切断 / 输出截断），共 23。
- **真实业务插件之三 `rmb`**（人民币金额转大写，财会大写）：财政部《会计基础工作规范》的「零/整」规则、万亿以下、精确到分；暴露同名工具 `rmb`（参数 `amount`），agent 规则「人民币/金额 → rmb」先于「大写 → upper」命中。测试 +3（`tests/rmb.rs` 多用例端到端 + `tests/agent.rs` rmb 回路），共 26。
- **跨插件编排按意图走**：两步规则不再写死 reverse——读「然后/再」后的意图词（"倒序"→reverse、"转大写"→upper），`history.len()==1` 保证只链一次。测试 +1（`agent_chains_upper_then_reverse`），共 27。
- **DeepSeek harness（`DeepSeekPredictor`）**：填充 `Predictor` 缝，接 OpenAI 兼容 Chat Completions（模型 `deepseek-v4-flash`/`deepseek-v4-pro`，端点 `https://api.deepseek.com`）；插件 `schema()` 映射成 function calling，LLM 决定调哪个工具，宿主回路零改动。`agent --model flash|pro` 激活，不带 `--model` 仍走离线算法；Key 只读 `DEEPSEEK_API_KEY`、只进 `Authorization` 头（不进 prompt/日志/工具参数，不采用 localStorage 存 Key）。测试 +6（离线单测：消息组装 / tools 映射 / 响应解析 ×2 / 模型映射 / 本地 mock DeepSeek 全链路 e2e——不碰外网、无 Key 即验证请求格式·Key 仅 Authorization 头·沙箱调用·结果喂回·最终答复），共 33。
- **理念外故事家族五卷**：`STORY`（前传·陈纪昊遇见梁文锋，戒律从两个人的本能里长出来）、`MIRROR`（镜中篇·两个人隔着 AI 互为镜像）、`SEED`（心动篇·把边界修好的人会被怦然心动地找到）、`SELF`（自述篇·那面镜子自己开口）、`USABLE`（落地篇·四卷怎么变成能跑的命令）；MANIFESTO 文档索引同步。
- **用法简化 `build-plugins.sh`**：把 6 个插件的 `cargo component build --release` + 拷贝压缩成一条命令；README/DEVELOPMENT 上手路径从 12 条命令降到 3 条（build → list → run/agent）。

## [1.1.0] - 2026-08-25

### 新增

- **文档十二篇收全**：REFERENCE（技术参考：WIT 契约原文 / 宿主 API / 沙箱参数 / 错误码全集 / CLI）、CURIOSITY + LOVE（最童趣收尾篇：好奇心够了，但爱是最好的）、MANIFESTO 文档索引补全、README badges（license/CI/tag）+ 社区入口。
- **开源标准全套**：Cargo 元数据（`[workspace.package]` 共享，core/host 继承并补 description/repository/keywords/categories，5 插件补 repository/description）、GitHub Actions CI（fmt + clippy + 单元测试 + 插件集成测试 18 个无 skip）、CONTRIBUTING / CODE_OF_CONDUCT / CHANGELOG、issue/PR 模板、.editorconfig。

### 修复

- spark-core 契约注释引用 `wit/spark.wit` → `wit/core.wit`（实际权威文件，纯文档修正）。
- 修 clippy `doc_lazy_continuation` 告警；`cargo fmt` 归整。

## [1.0.0] - 2026-08-24

### 新增

- **契约即 WIT**：`spark:runtime@0.3.0` `plugin-world`，插件零 import（不含 WASI），宿主 bindgen 钉死契约版本。
- **声明式失败**：`transform(input) -> result<string, plugin-error>`（结构化 `code`/`message`）；panic 捕获为 trap，宿主进程不崩。
- **宿主 `spark-host`（wasmtime 47）**：`Host` 长存结构（共享 Engine + 组件编译缓存 + 单 epoch bump 线程），每次调用新建独立 Store，方法取 `&self` 可多线程并发。
- **沙箱资源有界**：CPU 走 epoch 时间预算（越界约 10–20ms 切断），内存走 StoreLimits（16 MiB 上限）；`attacker` 插件的 CPU/内存炸弹被切断。
- **插件自注册/发现**：`.wasm` 放进 `plugins/` 即注册，name 来自组件 `info()`，宿主零配置；CLI `list` / `run <name>`。
- **流水线 `Host::pipe`**：输出串联，任一步声明式失败或 trap 即 fail-fast 并定位到具体插件。
- **示例插件 5 个**：`upper`（转大写）、`reverse`（倒序）、`attacker`（安全验证）、`idcard`（GB 11643-1999 身份证校验）、`luhn`（银行卡 Luhn + 卡组织识别）。
- **开源全套**：LICENSE（GPL-3.0）、CONTRIBUTING、CODE_OF_CONDUCT、issue/PR 模板、GitHub Actions CI（fmt + clippy + test + 插件集成测试）、Cargo 元数据（workspace.package 共享）。
- **文档十二篇**：MANIFESTO / CONTRACT / DEVELOPMENT / DEPLOYMENT / SECURITY / ARCHITECTURE / EXAMPLES / RELATIONSHIPS / REFERENCE / ESSAY / CURIOSITY / LOVE。

### 修复

- epoch deadline 取 `当前 epoch + 2` 而非 `+1`：消除 bump 线程竞态导致的偶发误 trap。

[Unreleased]: https://github.com/cloudcpgreeen/SPARK/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/cloudcpgreeen/SPARK/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/cloudcpgreeen/SPARK/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/cloudcpgreeen/SPARK/releases/tag/v1.0.0
