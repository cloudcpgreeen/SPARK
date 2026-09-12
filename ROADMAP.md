# SPARK 路线图

## 命题

> **同一个 WIT 契约 + 同一个 Wasm Component，可以同时是 Web / React Native 前端和 Rust 后端的统一业务组件。**
> Component 本身**不知道**自己跑在前端还是后端 —— UI 是宿主的事，状态与行为是 Component 的事。

## 三个概念的边界（比「代码能跑」更重要）

| 概念 | 是什么 | 例子 |
| --- | --- | --- |
| **Component** | **共享的业务状态与行为**（headless，不含任何 UI 概念） | `button.wasm`：`constructor → click → count` |
| **Host** | **平台适配与 UI / 运行环境**，**也是 Capability 的实现者** | Rust + wasmtime；Web(Vite+React)；RN(Hermes) |
| **Capability** | **外部能力**的**契约**（storage / remote API 等），实现由 Host 提供 | `spark:capability/storage@0.1.0`：`get` / `set` |

**黄金不变量**：三端**不要求代码相同** —— 要求的是**契约相同、Component 相同、Domain 行为相同**；
Host 可以完全不同。各 Host 各有实例与状态（后端点 3 次 → 3；Web 点 1 次 → 1），
共享的是契约与行为，**不是状态**。

**两套信任模型**（刻意分开，互不污染）：

| world | 契约 | 信任模型 |
| --- | --- | --- |
| `plugin-world` | `spark:runtime@0.4.0` | **零 import 的不可信插件沙箱**（安全边界） |
| `domain-world` | `spark:ui@0.1.0` | **前后端统一的域组件**，目前仍零 import |
| `store-world` | `spark:store@0.1.0` | **显式import `spark:capability/storage`** 的域组件（P2 起） |
| `provider-world` | `spark:mem-store@0.1.0` | **实现** `spark:capability/storage` 的组件，零 import（P3 起） |
| `delegating-provider-world` | `spark:delegating-store@0.1.0` | **同时 import 与 export** `spark:capability/storage` 的 Provider，把能力委派给下一层（P4-0 起） |

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

## P2 · Capability Contract Spike（**已完成**）

**命题换了一层**：P1 = `Component → 多个 Host`；P2 = `Component → Capability Contract → 多个 Host Implementation`。

> 一个 Component **import** 的 Capability，能不能由不同 Host 提供不同实现，
> 而 **Component 本身完全不改变**？

**叫 Capability Contract Spike，不叫 Storage** —— 先定边界，再谈实现。

**契约**（**没有**动 `spark:ui@0.1.0`，否则 P1 的 `button.wasm` 会变成孤儿）：

```wit
package spark:capability@0.1.0;
interface storage {
  variant store-error { unavailable(string), denied(string) }
  get: func(key: string) -> result<option<string>, store-error>;
  set: func(key: string, value: string) -> result<_, store-error>;
}
```

`Ok(Some(v))` 有值 / `Ok(None)` 没有这个 key / `Err` 能力本身失败 —— 「缺失」与「失败」必须可区分。

**实验对象**：`components/counter-store` → `counter-store.wasm`（`spark:store@0.1.0` / `store-world`），
import `spark:capability/storage`，状态写穿到 capability。

| # | 结果 | 判据 | 状态 |
| --- | --- | --- | --- |
| ① | 契约 | `spark:capability@0.1.0` 独立 package；`spark:ui@0.1.0` 零改动 | ✅ |
| ② | 一份 artifact | `counter-store.wasm` 一次构建，sha256 `85691b8e…` | ✅ |
| ③ | **Rust Backend PASS** | 同一 wasm + 后端进程内 map → 点 3 次 = 3，新实例 = 3 | ✅ |
| ④ | **Web Browser PASS** | 同一 wasm + localStorage → 真实 Chrome 点击，**刷新后 count 存活** | ✅ |
| ⑤ | 错误路径 | 同一 wasm + 只读后端 / 禁存储 → 新实例 = 0 | ✅ |
| ⑥ | 换实现不改组件 | ③④⑤ 用的是**同一份未被重新编译的 wasm** | ✅ |
| ⑦ | P1 完好 | 35 个原测试全绿；P1 的 WIT / 组件 / 宿主代码零 diff | ✅ |
| ⑧ | RN | injection point exists / **runtime unverified** | ⚠️ |

**⑤ 的精确措辞**：*set failure is observable through a subsequent fresh instance*。
**不是**「错误处理已验证」—— 组件侧压根没处理 `Err` 分支。

**两种「不变」是两件事，不许合并**：
- P1：`button.wasm` **byte-for-byte 不变**（零 diff 证明）。
- P2：`counter-store.wasm` **只构建一次**，换 Capability 实现时**不重新编译**（sha256 相同证明）。

**`ns` 是 Host instance policy，不是契约语义。** 组件发裸 key（`"count"`），
`ns:key` 前缀由 Host 在实例化时加上。

### P2 明确不做 / 留给 P3

- **不回答 `AsyncStorage` 的同步/异步问题。** 契约是同步的，`AsyncStorage` 是 Promise；
  要么「内存 Map + 异步落盘」（冷启动 hydration 竞态），要么把契约改 async
  （级联 wasmtime + wit-bindgen + jco + Metro，且 `future<T>` 在本工具链实测不可用）。
  **该不该 async 应由实验结果决定，不是先入为主的 API 设计。**
- 不做 capability 的权限/授权模型；后端实现就是进程内 map（换 DB 只动 `domain_store.rs` 一个文件——这本身就是结论）。

## P3 · Capability 的实现也可以是一个 Component（**已完成**）

**命题再往下推一层**：P2 = `Component → Capability Contract → 多个 Host Implementation`；
P3 = `Component → Capability Contract ← Component` —— **实现者本身也是组件**。

```
counter-store.wasm  (Consumer, P2 冻结, SHA 85691b8e…)
        +                                          wasm-tools compose
mem-store.wasm      (Provider, 导出 storage，零 import)  ────────────────>  composed.wasm
                                                                            （零 import）
```

| # | 结果 | 判据 | 状态 |
| --- | --- | --- | --- |
| ① | Provider Component 能实现能力 | `mem_store.wasm` 零 import；`provide <wasm> k v` → `got: v` | ✅ |
| ② | 组合消除了 import | `composed.wasm` 在**空 Linker** 上 `composed <wasm> 3` → `count: 3` | ✅ |
| ② 对照 | 这条路真的是空的 | 裸 `counter-store.wasm` 在**同一个空 Linker** 上必须失败 | ✅ |
| ③ | 代价是作用域 | 同一份 `composed.wasm` → `reloaded: 0`（对照 P2 的 `3`） | ✅ |
| ④ | 两个源制品不动 | `counter-store.wasm` sha256 不变；P2 宿主代码零 diff | ✅ |
| ⑤ | 零 import：**实测** | `wasm-tools component wit` 观测 `composed.wasm` 为 0 imports | ✅ |
| ⑥ | P2 完好 | 42 个测试全绿（38 原 + 4 新增）、fmt / clippy 干净 | ✅ |
| ⑦ | Web | 真实 Chrome：P1 → 0、P2 → 3、P3 → 0；转译**无 `--map`** | ✅ |

**关键区分**：`composed.wasm` 是 **derived artifact**（两个 Component 的组合产物，
与 jco 转译产物同类），**不是**第三个 Component 源。这与「重新编译 `counter-store`」是两回事。

**③ 的措辞**：Provider 把状态放在自己的实例内存里，所以组合后的能力状态是 instance-scoped。
这是**本次实现的后果**，**不是** Component Model 的普遍定律。

**勘误（P4-0 期）**：⑤ 原写「零 import 由 `--no-imports` **强制**」。实测该 flag 在
wasm-tools 1.245.1 中是 **no-op**（带与不带产出字节相同，且带 flag 的组合产物仍保留 import），
故**不能**作为因果依据。零 import 由 `component wit` 独立观测 ——
**结论不变，理由已更正**；这是证据纠偏，不是重新打开 P3。

### P3 明确不做 / 留给后续

- **不回答「能力的作用域怎么保住」**（委派：Provider 自己 import 一个外部存储）——
  只记录，不选路。
- **不做运行期动态组合**：wasmtime 47 的 `LinkerInstance` 只有 `func_wrap` / `func_new` /
  `module` / `resource`，**没有**「把另一个组件实例的导出接进本组件导入」的一等 API；
  `wac` 需联网安装，本机不可用。**如实记录为限制，不绕路。**
- 不做 capability 的权限模型、不做真实 DB / 网络存储。

**P2 留下的待决**：Capability Contract 要不要 async。见上文 P2 末尾 —— 仍然挂着。

## P4-0 · Self-import / self-export preflight（**已完成**）

**这不是 P4，是 P4 开工前的单点风险 gate。** 唯一的问题：

> **一个 Component 能否同时 import 和 export 同一个 WIT interface，并被 `wasm-tools compose` 正确组合？**

三层（WIT 解析 / cargo-component 绑定生成 / wasm-tools compose）里任何一层不接受 → 停，不绕路。

```
counter-store.wasm  (import storage, P2 冻结)
        +
delegating-store.wasm  (同时 import 与 export storage)     ── wasm-tools compose ──>  composed-delegating.wasm
                                                                                      export counter-store
                                                                                      import storage ← 恰好这一个
```

| # | gate | 判据 | 状态 |
| --- | --- | --- | --- |
| G0.1/G0.2 | WIT 层 + cargo-component 接受「双栖」 | `cargo component build --release` → `delegating_store.wasm` | ✅ |
| G0.3 | world 结构 | **同时**有 1 行 import 与 1 行 export，都是 `spark:capability/storage@0.1.0` | ✅ |
| G0.4 | compose 成功 | 产出 `dist/composed-delegating.wasm` | ✅ |
| G0.5 | imports **== 预期集合** | 恰好 1 行 `import spark:capability/storage@0.1.0` | ✅ |
| G0.6 | unexpected imports **== ∅** | 除那一个外**没有别的 import** | ✅ |

**G0.5/G0.6 比 P3 的 `imports == ∅` 更强** —— 两个方向都显式打印，
验证的是**边界没有偷偷扩大**，不是只喊一句「相等」。

### 结论

同一个 WIT interface 可以同时被 import 与 export，当前工具链三层都接受；
**组合没有消灭 capability 边界，而是把边界往上搬了一层**
（P3 的 `composed.wasm` 看似零 import，只是因为边界正好被 Provider 吃掉了）。

**P4-0 is proven. P4 is NOT proven.**

### P4-0 的实测发现

- 同一个 interface 的 import 侧与 export 侧生成**两个不同的 Rust 类型**
  （`bindings::spark::…::StoreError` ≠ `bindings::exports::spark::…::StoreError`），
  委派必须在值这一层转换一次；**但模块路径不撞**（`exports::` 前缀），
  所以「工具链不支持双栖」这个担心不成立 —— 一个具体的工具链风险被划掉。
- 组合产物 `composed-delegating.wasm` 与 `composed.wasm` 一样是 **derived artifact**，不是 Component 源。

### P4-0 明确不做

不设计 P4-1（不写 Host 绑定、不改 `build-ui.sh`、不加 CLI 子命令、不做 Web 区块）；
不决定「外部 provider 是谁」；不碰 todo / RN / remote transport / async / 数据库。

## P4-1 · Capability 的状态所有权落回 Host（**已完成**）

**P4-1 的唯一新增变量是 state ownership。** 链路：

```
counter-store → import storage → delegating-store → import storage → Host
```

P4-0 只证了「那一行 import 存在」；P4-1 证的是**那一行是活的**。

| # | gate | 观测 | 状态 |
| --- | --- | --- | --- |
| G1 | Provider 源码无状态声明 | grep → 无输出 | ✅ |
| G2 | 组合边界 | `component wit` 恰好 1 import + 1 export | ✅ |
| G3 | Host binding 未变 | `git diff` 空；契约仍 `spark:capability@0.1.0` | ✅（无独立观测项） |
| G4 | 状态落在 Host | `stored: 1 条` | ✅ |
| G5 | 跨实例存活 | `reloaded: 3` | ✅ |
| G6 | 反事实控制（同一个 Host） | P3 的 `composed.wasm` → `reloaded: 0` / `stored: 0 条` | ✅ |

```
P3    Capability → Provider Component → Provider-local state → 实例销毁 → 0
P4-1  Capability → Provider Component → Host Capability → Host-owned state → 实例销毁 → 3
```

**结论**：Provider Component 可以作为 Capability 的中间委派层，**而不必成为状态所有者**；
当 Capability 最终由 Host 提供时，状态跨越实例生命周期保持。
**P3 的 `reloaded: 0` 由此从「结论」降回「某个实现的后果」。**

**durable 的定义（冻结）**：*survives replacement of the Component / Provider instance.*
**不等于**进程重启 / 宿主重启 / 磁盘持久化 / 数据库；`Host process restart → 未测试`。

**本轮没有新增任何 Host 代码、WIT 或组件** —— 只有 `spark-host/tests/delegating.rs`（2 个测试）。
`store` 子命令本来就在跑「实例 A 点 n 次 → 实例 B 全新读回」这个协议。

## 后续 · 跨端 Domain Components

把 Button 扩成真正成体系的域组件；验证组件间组合与前后端一致的行为。
若要自动化 Web 验收，此时引入 Playwright。

## P4 · Component Registry

> ⚠️ **本节已被重新校准的路线取代**（P4 改为 Durable Capability，`P4-0` 即其 preflight）。
> 发现 / 版本 / 分发的问题没有消失，但不再是 P4 的定义。本节留待 P4-1 设计 gate 时统一整理。

组件的发现 / 版本 / 分发。注意：不要靠提交 `hosts/web/src/generated/` 来分发 ——
那是从契约派生的产物。正确做法是发布 npm 制品或 CI 上传构建产物。

## P5 · AI Agent（**冻结中**）

`spark-host/src/agent.rs` 与 `spark-host/src/deepseek.rs` 已加模块级冻结注释：
代码保留、测试保持通过、**不再迭代**。`spark-host agent` 子命令仍可用，但帮助文本已标注属未来层。

**冻结理由**：Agent 不该是架构核心，它只是**另一种 Component Consumer** ——
与 Web 宿主、Rust 后端宿主同层。先把「同一份契约 + 同一个组件服务多端」证明扎实，
接 Agent 会自然得多。

**解冻条件**：P0–P4 的跨端契约有真实使用方，或确实需要 Agent 作为组件的消费者。
