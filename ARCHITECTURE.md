# 架构与实现（ARCHITECTURE）—— 框架是怎么形成的

> 完整的版本：把「思路、具体实现逻辑、框架怎么形成的」一次讲清。
> [MANIFESTO](MANIFESTO.md) 回答为什么，本文回答**怎么实现的**。
> 本文与代码一起演进——契约升版、机制改动时，先改这里。

## 0. 一句话

SPARK = **契约即 WIT 的组件运行时**：领域逻辑做成满足 WIT 契约的 WASM 组件，宿主在沙箱里加载调用。新增能力 = 新增一个组件，宿主零改动。

## 1. 框架是怎么形成的（演进史）

SPARK 不是一次设计出来的，是一步步**被真实需求逼出来的**——每加一个真实插件，宿主就多长一块能力。顺序是「先跑，再富类型」，不是提前设计。

| 阶段 | 发生了什么 | 为什么 |
| --- | --- | --- |
| **0.1.0 契约原型** | `transform(input) -> string`，插件 = 导出 `plugin-world` 的组件，宿主加载调用 | 证明「组件 + 宿主」这条路能走通 |
| **0.2.0 声明式失败 + 元数据** | `transform -> result<string, string>`（插件可以体面地说「不」，不用 panic）；`info() -> plugin-info`（name/version/description） | 插件失败该是值不是崩溃；元数据就位，为注册/发现铺路 |
| **自注册 / 发现** | 把 `.wasm` 丢进 `plugins/` 目录即注册，`Host::discover` 逐个沙箱读 `info()`，坏的跳过 | 宿主零配置文件，「新增插件宿主零改动」被反复验证 |
| **Host 长存 + 并发** | 从「每次调用新建 Engine」进化到共享 Engine + 组件编译缓存 + 单 epoch bump 线程；每次调用新建独立 Store；方法取 `&self` 多线程并发 | `Component::from_file` 每次调用都是完整编译（贵）；要支持多线程且隔离不变 |
| **0.3.0 结构化错误** | `transform -> result<string, plugin-error>`（code/message） | **流水线是消费方**：要按错误类型分支、定位失败步骤，裸 string 承载不了 |
| **流水线 pipe** | 输出串联，任一步 fail-fast 并定位 | 把核验做成真链：idcard → luhn 各自独立又能接成链 |
| **0.4.0 Agent 调用面** | `schema()` 暴露 LLM 可见工具清单 + `invoke(tool, args)` 结构化调用；宿主加 `schemas`/`invoke` + `agent` 回路模块 | 融合 rhua-chatgpt-web 的「插件 = LLM 工具」模式，但把不可信 JS 换成沙箱 WASM 组件 |

每个阶段都由一个具体痛点驱动，不是凭空加功能。

## 2. 实现逻辑（三层 + 关键机制）

### 2.1 契约层（wit/）

- `wit/runtime.wit`：`plugin-world` 世界，导出 `spark:runtime/plugin` 接口。**零 import，连 WASI 都不 import**——插件摸不到文件/网络/时钟，攻击面最小。0.4.0 起接口含 `info`/`transform`（原调用面）+ `schema`/`invoke`（Agent 调用面）。
- 宿主 `bindgen!({ path, world })` 钉死契约版本：加载不匹配组件时 `instantiate` 直接失败，不需要显式版本检查函数。
- 宿主端 bindgen 生成类型路径：`crate::exports::spark::runtime::plugin::{PluginInfo, PluginError}`（不在 crate root，编译报 E0425 时按 rustc 建议 import）。
- `spark:core@0.1.0`（`wit/core.wit`）是领域骨架契约，对应 `spark-core` 的 `contract_version()`。

### 2.2 宿主层（spark-host）

```
Host {
    engine: Engine,                                  // 共享，编译一次
    components: Mutex<HashMap<PathBuf, Arc<Component>>>,  // 组件编译缓存
    stop_bumper: Arc<AtomicBool>,                    // epoch 线程停止信号
    bumper: Option<JoinHandle>,                      // epoch bump 线程
}
```

- **每次调用新建独立 Store**（`run`/`info`/`discover`）：沙箱隔离，实例互不污染，trap 不串。
- **`new_store`**：`StoreLimits`（内存上限 16 MiB + `trap_on_grow_failure`）放进 Store 宿主数据，`store.limiter(|data| &mut data.limits)` 闭包式注册；`store.set_epoch_deadline(2)` 设 CPU 预算。
- **`Host::run` 返回分层**：`Result<(PluginInfo, Result<String, PluginError>)>` —— 外层 Err = 宿主/沙箱错误（trap、加载失败、资源越限）；内层 Err = 插件声明式失败（值，可按 code 分支）。CLI 据此四态区分：`output` / `plugin error [code]` / `plugin trap` / `pipe`。
- `Drop` 停掉 epoch 线程并 join，不泄漏后台线程。

### 2.3 沙箱（为什么这样设计）

| 攻击 | 防线 | 机制 |
| --- | --- | --- |
| 死循环 / CPU 耗尽（含空 `loop {}`） | **epoch 时间预算** | 后台线程每 10ms `increment_epoch`，越界执行即 trap |
| 内存炸弹（无限分配） | **StoreLimits 上限** | 越限即 trap |
| 读文件 / 网络 / 时钟 | **零 import** | 插件根本没有这些能力 |
| panic / 崩溃 | **trap 捕获** | 宿主进程不崩，实例互不污染 |

- **刻意不用 fuel 计量**：`loop`/`br` 指令消耗 0 fuel，空死循环会漏网。epoch 是时间预算，任何执行超预算都被切断。
- **deadline 取 `当前 epoch + 2` 而非 +1**：+1 与 bump 线程有竞态——bump 恰好落在 store 创建窗口时，正常调用会拿到 ~0ms 预算而误 trap（全量测试负载下偶现）。+2 留一格裕量，越界仍在约 10–20ms 内切断。

### 2.4 插件层（spark-plugin*）

- 每个插件是**独立 workspace**，`crate-type = ["cdylib"]`，依赖 `wit-bindgen-rt`，`[package.metadata.component] package = "spark:runtime"`。
- 构建目标**钉死 `wasm32-unknown-unknown`**（`.cargo/config.toml`）→ 产物零 WASI import。不用 wasip1/wasip2：那会把 `wasi:cli/io` 拖进组件，破坏最小能力。
- 插件实现 `Guest` trait：`info()`（name/version/description，version 用 `env!("CARGO_PKG_VERSION")`）+ `transform(input)`（原调用面）+ `schema()`（工具清单）+ `invoke(tool, args_json)`（Agent 调用面）。`invoke` 解析 JSON 参数后复用 `transform` 逻辑。
- 依赖 `wit-bindgen-rt` + `serde_json`（编译期库，**运行时组件仍零 import**，沙箱零能力不变）。
- 导出宏用**两参形式**：`bindings::export!(Upper with_types_in bindings)`（单参版会编译失败）。

### 2.5 流水线（pipe）

- `Host::pipe(dir, input, names)`：`discover` 找名字 → 逐个 `run`，前一步输出喂下一步。
- `PipeFailure::Declined { step, error }`（声明式失败，`error.code` 可编程分支）/ `PipeFailure::Trap { step, detail }`（trap，`detail` 含 wasm backtrace 定位崩溃点）。

### 2.6 Agent 回路（`spark-host/src/agent.rs`）

- **插件 = LLM 可见的工具**（借鉴 rhua-chatgpt-web 的插件模型）：`Host::schemas` 收集各插件 `schema()` → `(工具名, 插件文件, 插件信息)` 注册表；`Host::invoke` 在沙箱里按工具名调用。
- **决策者 = `Predictor` trait**：`decide(prompt, history, tools) -> Decision`。两个实现共用一个回路：`AlgorithmPredictor`（本地关键词算法，离线）；`DeepSeekPredictor`（`spark-host/src/deepseek.rs`，OpenAI 兼容 Chat Completions 真实 harness，`--model flash|pro` 激活）。换决策者宿主回路零改动。
- **回路 `run_agent`**：Predictor 决策 → `Host::invoke`（fresh Store + epoch + 16MiB）→ 结果按不可信数据包装、截断（`TOOL_RESULT_LIMIT`=4096）→ 喂回 → Final 或到 `MAX_STEPS`=8。
- **CLI**：`spark-host agent "<prompt>"` 走离线算法；`--model flash|pro` 走 DeepSeek harness（需 `DEEPSEEK_API_KEY`，Key 只进 `Authorization` 头）。

## 3. 关键取舍（为什么是这些做法，而不是别的）

- **epoch 不是 fuel**：空死循环漏网是 fuel 的已知漏洞，时间预算堵住它。
- **每次新建 Store，不是复用**：隔离优先；Engine 与组件编译缓存复用，贵的一次不付两次。
- **自注册不是配置文件**：`.wasm` 放进去就有，宿主零改动——这是可扩展性的证明，也是纪律。
- **0.3.0 才做结构化错误**：先有消费方（流水线）再富类型，不凭空造类型。
- **同步调用**：单次调用阻塞调用线程，但被 epoch 切断、阻塞有界；`Host` 线程安全，多线程并发互不干扰。
- **决策者留 trait 缝，先用本地算法**：接真实 LLM 前先用关键词规则跑通「决策 → 沙箱调用 → 结果喂回」的回路，验证最小可能性；DeepSeek 只留 `Predictor` 插入点，不先接网络、不先存 Key。

## 4. 文档地图（完整版怎么拼）

| 文档 | 回答什么 |
| --- | --- |
| [MANIFESTO](MANIFESTO.md) | 为什么存在、是什么、边界在哪 |
| [CONTRACT](CONTRACT.md) | 契约即 WIT：唯一权威接口 |
| **本文件（ARCHITECTURE）** | 怎么实现的：框架形成 + 实现逻辑 |
| [DEVELOPMENT](DEVELOPMENT.md) | 怎么写、怎么测、怎么提交 |
| [EXAMPLES](EXAMPLES.md) | 怎么用：手把手跑一遍 |
| [SECURITY](SECURITY.md) | 怎么防：威胁模型 |
| [RELATIONSHIPS](RELATIONSHIPS.md) | 谁是谁：人物与关系 |
| [DEPLOYMENT](DEPLOYMENT.md) | 怎么交付：门禁与发布 |

> 读法：先 MANIFESTO 懂魂，再本文件懂骨，EXAMPLES 上手，SECURITY 放心。
