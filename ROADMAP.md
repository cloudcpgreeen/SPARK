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
| `forwarding-provider-world` | `spark:forwarding-store@0.1.0` | 与上者**逐字相同**的纯委派 Provider，用于在链上再叠一条边界（P4-2 起） |

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

## P4-2 · Multi-hop Capability Delegation（**已完成**）

**命题**：P4-1 只验证了**一级**委派。P4-2 在链上再叠一条纯委派边界，
问 **Provider 数量** 会不会改变 Host ownership。

```
counter-store → import storage → A(delegating-store) → import storage
                                                     → B(forwarding-store) → import storage → Host
```

| # | gate | 观测 | 状态 |
| --- | --- | --- | --- |
| G1 | Provider A = 已冻结的 `delegating-store` | 源码 sha256 `3c43b506…` 未变 | ✅ |
| G2 | 新增 Provider B | `forwarding-store.wasm` 构建成功 | ✅ |
| G3 | B 自己的形状 | `component wit` 恰好 2 行：1 import + 1 export | ✅ |
| G4① | `compose(A -d B)` | 退出码 0；中间制品 world 恰好 2 行 | ✅ |
| G4② | 中间制品当 `-d` | 退出码 0；两个最终制品 world 逐字相同 | ✅ |
| G5 | 两跳生命周期 | `reloaded: 3` / `stored: 1 条` | ✅ |
| G6 | 反事实 | 一跳 = 两跳 = 3；P3 控制 = 0 | ✅ |
| G7 | 源码无状态声明 | grep → 无输出 | ✅ |

### ⚠️ 工具行为事实：`wasm-tools compose` **不做传递闭包**

只读探针：`compose(store -d A)` 与 `compose(store -d A -d mem-store)` **字节相同**，
而把两个 `-d` 对调就变成 P3 的形状（0 import）。⇒ **`-d` 按顺序取第一个能满足 root import
的定义，不追那个定义自己的 import；定义是叶子。**

**后果**：一次 `-d A -d B` 造不出两跳链。**多跳只能靠「顺序组合两次」** ——
先把 A∘B 组合成中间制品，再拿它当 `-d` 喂给第二次 compose。

**所以不能写成「compose 支持多跳」。** 本次证据恰恰相反：工具本身只做一跳。

### 结论

> 在本次验证的两级纯委派 Provider 链中，增加一个 Provider 委派边界**不会改变**
> Capability State 的 Host ownership；状态仍能跨越 Domain / Provider Component 实例替换而保持。

**不许升级成**：「任意深度 Provider 链都保证 Host ownership」（两级证据不支持任意多级）；
「Component Model 保证 Provider 不持有状态」（这是本次实现的观测，不是模型定律 ——
对照 P3：`mem-store` 就是持有状态的 Provider）。
`durable` 的定义不变，仍**不等于**进程重启 / 宿主重启 / 磁盘 / 数据库。

**实例拓扑**：A 与 B 在组合产物里是**内联**的 —— Host 只实例化一个组合组件。
准确说法是 `fresh Store → fresh composed Component instance → 其内部含 fresh 的 A、B 实例`，
**不是** Host 分别实例化 A、B。

**P4-2 的真正产出**：State ownership 被从 Component Model 里单独剥离出来 ——
`Component identity ≠ Provider identity ≠ Capability state ownership`。
Provider 可以是一条又一条纯委派边界，最终状态仍落在 Host。

**P4 至此封板**，不再做第三 / 第四层 Provider。
**P5（同一 Domain Component 跨 Host）是另一个问题，不让 P4 的结论外溢**，需另开 Design Gate。

## P5 · Same Domain Component, Multiple Hosts（**已完成**）

**命题**：**同一个不可变 Domain Component artifact** 能不能被两个不同的 Host 承载，
Domain 语义一致，而 Host Capability 的实现与状态各自独立？

**这不是「两个 Host 共享状态」—— 恰恰相反，P5 明确不共享**（那会变成 Remote Capability）。

### ⚠️ P5 与 P1 的区别（不写这句，P5 读起来就是 P1 的复述）

P1 已经证明过「同一份 artifact、两个 Host」，但 `button.wasm` 是**零 import** 的 ——
它根本没有碰到 Host。P5 真正新增的是 **capability 维度**：同一个 artifact **import 一个能力**，
两个 Host 各给**不同实现**，而 Domain 行为一致。

### 设计期发现（改掉了原设计）

1. **「两个 Host 都用同一个文件」按字面不成立** —— Web Host 从不加载 `counter-store.wasm`，
   它加载 jco 的派生产物。能成立的精确版本是：**同一个冻结 artifact 是两边共同的上游**，
   Rust **直接加载**它，Web **以它为转译输入**。所以 G1 断言的是「**转译输入**的 sha」，
   且在**转译之前**检查。（与 P3 已冻结的 `derived artifact ≠ Component 源` 是同一条纪律。）
2. **`build-ui.sh` 不能用来做 P5 的 Web 构建** —— 它无条件重编译 `counter-store`，
   与「禁止重新编译」直接冲突，且每次都 `rm -rf hosts/web/src/generated`。
   ⇒ P5 另开 `p5-web.sh` 与 `src/generated-p5/`。**`build-ui.sh` 一个字未改。**
3. **此前仓库里没有任何一处断言这个冻结 sha** —— `build-ui.sh` 的 hash 是**同一次运行内和自己比**。
   ⇒ 新增 `verify-artifact.sh`：唯一 artifact identity 入口，两条 Host 路径都先跑它。
4. **Web 验收从来没有自动化过** —— P1–P3 的「点 3 次 → 3」都是人眼看、手写进文档的散文。
   ⇒ 本轮第一次变成可断言的脚本。

| # | gate | 观测 | 状态 |
| --- | --- | --- | --- |
| G1 | artifact identity | 转译**前**＋**后** `verify-artifact.sh` 均 OK，sha = `85691b8e…` | ✅ |
| G2 | Rust/Wasmtime 直接加载 | `count: 3` / `reloaded: 3` / `stored: 1 条` | ✅ |
| G3 | Web/jco 承载（Playwright headless Chromium） | `0` → click ×3 → `3` → `localStorage '3'` → reload → `3` | ✅ |
| G4 | Domain 语义一致 | 两端各自 fresh 实例 → click ×3 → `3` | ✅ |
| G5 | Capability 实现可以不同（反事实） | Rust deny：`3 / 0 / 0 条`；Web deny：`0` → click ×3 → `3` → localStorage 仍 `null` → reload → `0` | ✅ |
| G6 | State 独立 | **结构性保证，不可能失败** ⇒ **不计入 PASS** | — |

### 结论

> 同一个冻结的 Domain Component artifact（`counter-store.wasm`，SHA-256 `85691b8e…`，
> 未经重新编译）被两个不同的 Host 承载：Rust/Wasmtime **直接加载**它，Web/jco **以它为转译输入**
> 得到 Host 侧适配产物。两边的 Domain 行为一致（各自新建实例 → click ×3 → 3），
> 而两边提供的 Capability 实现不同（进程内 HashMap vs 浏览器 localStorage）。

> G5 的反事实进一步表明，在两端更换为宿主侧 deny Capability 后，Domain 的
> `click ×3 → count 3` **仍成立**，但状态不再跨 fresh instance 保留（`reload → 0`）。
> 因此本次验证证明的是 **Domain 行为与 Capability 状态机制的分离**，
> 而非 Capability failure 导致 Domain 调用失败。

**核心句**：同一个 Domain artifact，不要求同一个 Host，也不要求同一个 Capability implementation；
**Domain 与 Host Capability 的边界才是可移植性的核心。**

### 证据含义的边界（不许滑坡）

- **G3 的 `reload → 3` 只证明** Web Host 的 localStorage Capability 在工作。
  **它本身不是**「Domain 与 Capability 分离」的证明 —— 那由 **G5 的反事实**承担。
- **G5 的顺序不可颠倒**：必须**先**证明坏 Capability 下 `click ×3 → count 3` 仍成立，
  再证明 `reload → 0`。若第一步不成立，证明的是 *capability failure propagation*，
  **不是**想证的「状态持久性由 Capability 决定、Domain 行为仍然存在」。
- Web 侧的「坏 capability」是**宿主侧的第二种配置**，**不是**「组件处理了错误分支」。
  精确说法仍是 *set failure is observable through a subsequent fresh instance*。
- **G6 不可能失败**（进程内 `HashMap` vs `localStorage`：不共享介质、不共享地址空间），
  因此是结构性保证，**不算实验结果**。

**不许外推**：

- ❌ 「任意 Host 都可以承载」—— 本次验证的是这两个。
- ❌ 「RN 已经通过」—— RN 只有编译门，没有 runtime proof，另立 Runtime Gate。
- ❌ 「同一份 Component 字节直接在浏览器中执行」—— 浏览器执行的是 jco 的**派生产物**；
  相同的是**转译输入**。
- ❌ 「跨 Host 共享状态」—— P5 明确不共享。

**Web 不进 CI**：P5 证明的是**架构命题**，CI integration 是**工程化命题**，两者不该混在一个 gate 里。
另开一个很小的 *P5-CI / Web Verification Gate*，而不是偷偷塞进 P5。
（P5 页面目前是 dev-only，不进 `vite.config.ts` 的 production 入口。）

## P6 · Remote Capability（**已完成**）

**命题**：如果 Capability 从**本地 Host** 变成 **Remote Capability**，
Wasm Component 的边界到底有没有发生变化？

**不是**「Component 支持远程」—— **组件、WIT、artifact 一个字都没改**，
变的只有 `Arc<dyn CapabilityBackend>` 背后站的是谁。

```
P1    Component → Host                                  PASS / FROZEN
P2    Component → Capability → Host                     PASS / FROZEN
P3    Component → Capability ← Component                PASS / FROZEN
P4    Provider chain → Host-owned State                 PASS / FROZEN  4997bba
P5    SAME Component → DIFFERENT Hosts                  PASS / FROZEN  08dcadc
P6    Local Capability → REMOTE Capability              PASS ← 本轮
```

### ⚠️ P6 与 P4-2 的区别

P4-2 的对照臂是**两个字节不同的最终制品**；P6 的对照臂**连制品都是同一份** ——
同一个 `counter-store.wasm`、同一个 Host 二进制、同一个 `ns`、同一个 click protocol，
**唯一的架构变量是 Capability 后端的落点**（进程内 HashMap vs 网络另一头的进程）。

### 核心边界（**这是结论的一部分，不是脚注**）

> 「Remote 是纯 Host concern」成立的前提是：**该 Host 能为同步 WIT 调用提供阻塞式 IO。**

Rust Host 满足（`ureq` 在 wasmtime 的调用线程上阻塞，同步返回后照常 lowering；
**零 async、零新依赖**）。Web/jco 在当前**同步 artifact + 同步 WIT**下**结构上不满足**：
jco 1.29.0 生成的代码里字面写着
`"non async exports cannot synchronously call async functions"`，且走的是 sync 分支。
要让它可行必须 async WIT 或 `asyncImports`（两条都会改契约），且 JSPI 目前只有 Chromium 支持。

### 设计期发现（改掉了原设计）

1. **`ureq` 已在树内，wasmtime 全程同步** —— `async|tokio|block_on|call_async` 在
   `spark-host/` 零命中 ⇒ **WIT 不变、组件不重编译**即可接上远端后端。
2. **契约里的 `unavailable` 至今从未被任何 Host 产生过** —— 网络失败天然落进它，
   **不需要扩契约就能表达失败**。
3. **`denied` 不能挪用** —— P2 已冻结为**宿主侧只读模式**，拿它接网络失败等于静默重定义。
4. **沙箱的 epoch 预算原本是 per-Store 而非 per-call** —— 本地调用微秒级所以从未触发，
   但一次阻塞的远端调用就可能吃掉预算，而超时表现为 `store trap:` + **`SUCCESS` 退出码**，
   证据会被静默污染。**这是被网络路径暴露出来的既有隐患，不是 P6 造出来的。**
   修复**只落在** `domain_store::click_times`；`domain.rs` / `compose.rs` 的同类债务
   **刻意未同步修改**（独立债务，不扩大爆炸半径）。
5. **`len()` 是「独立于组件」观测的落点** —— 远端臂需要等价的远端侧观测 ⇒ `/stats`。

| # | gate | 观测 | 状态 |
| --- | --- | --- | --- |
| G0 | artifact / WIT 冻结 | `counter_store.wasm` sha 未变、`capability.wit` 未变、关键路径 git zero-diff | ✅ |
| G1 | Component **没有访问网络的能力** | 零 WASI + world imports **恰好** `spark:capability/storage@0.1.0` + 源码零 transport 字样 | ✅ |
| G2 | Local 对照（两个不同进程） | `count: 3` / `reloaded: 3` / `stored: 1 条`，两次相同 | ✅ |
| G3 | Remote 反事实 | ③ `3` → ④ **新 client 进程** `count: 6` → ⑤ 零点击直读 `6` | ✅ |
| G4 | **状态真的在远端（主证据）** | 第三方写入 `counter-store:count = 100` → 客户端读到 `count: 100`；换空 server B → `0` | ✅ |
| G5 | 失败反事实 | **先** `count: 3` 成立，**再** `reloaded: 0`；错误是 `Unavailable` **不是** `Denied` | ✅ |
| G6 | sync/async impedance 的落点 | **设计发现，无可跑判据** ⇒ **不计入 PASS** | — |
| G7 | 没有新增 Component Model 层 | `wit/` 与 `components/` 零 diff、文件清单不变 | ✅ |

### 结论

> 同一个未经重新编译的 Domain Component artifact，在不修改 WIT 的前提下，
> 可以由 Rust Host 的**本地** Capability Backend 或**远程 HTTP** Capability Backend 承载；
> 两者的 Domain 行为保持一致，而 **Capability state 的实际落点**可以从
> Host 进程内存**移动到远端服务进程**。

> 本次证明**依赖 Rust Host 能够为同步 WIT 调用提供阻塞式 I/O**；
> 它**不证明**当前同步 WIT artifact 可以在 Web/jco 等不能同步阻塞的 Host 上
> 直接采用同样的 Remote Backend。

```
        同一个冻结 artifact（未重新编译）
                    │
        ┌───────────┴───────────┐
   Local Backend          Remote Backend
   进程内 HashMap          HTTP → 另一个进程
        │                       │
        └───────────┬───────────┘
             Domain 语义一致
                    ↓
        State ownership / persistence location
            由 Capability Backend 决定
```

**G4 为什么是主证据**：那个 `100` **不是客户端进程产生过的任何东西** ——
它只可能来自远端。`/stats` 由客户端调用，只是佐证。

### 证据含义的边界（不许滑坡）

- **G5 的顺序不可颠倒**：必须**先** `count: 3` 成立，**再** `reloaded: 0`。
  若第一步不成立，证明的是 *capability failure propagation*，**不是**想证的分离。
- **失败臂的断言不经组件**：组件吞掉 `Err`，错误值从组件侧不可观测
  （P2 的精确说法是 *set failure is observable through a subsequent fresh instance*）。
  变体断言直接打在 `RemoteBackend` 上，**没有为观测往契约里加探针**。
- **G6 没有可跑判据**，它是「记录一个位置」——写进记录，**不包装成实验发现**。
- **不许由此宣布「契约已经足够」**：实测只证明它能**表达**失败，
  没证明它能**区分**网络不可达与远端 5xx。

**不许外推**：

- ❌ 「Component 支持远程」—— Component 什么都没变，**变的是 Host**。
- ❌ 「Wasm Component Model 自带 RPC」—— 分布式落在 **Host Capability Adapter**。
- ❌ 「任意 Host 都支持 Remote Capability」—— **Rust Host 能，是因为它能阻塞**。
- ❌ 「当前 Capability contract 已经足够完整」。
- ❌ 「durable = 跨进程 / 重启永久持久化」—— P4 的冻结定义**不动**，
  持久性由**后端**决定，不由 Component 决定。
- ❌ 「RPC / WebSocket / gRPC 都已经验证」—— 本轮只验证了**一个 HTTP 后端**。
- ❌ 「Web / RN Remote Runtime 已验证」—— 两者都不在本轮范围内。

### 冻结为下一轮的问题

`store-error` 的 2 个 variant **能表达**远端失败，但 `unavailable`
**区分不了**「连不上」与「远端 500」。**要不要扩契约不在本轮决定** —— 那是独立的契约实验。

**P6 明确不做**：❌ Web / RN 的 Remote Runtime 验证 ❌ async WIT / `future<T>` ❌ JSPI
❌ RPC / WebSocket / gRPC ❌ 真实数据库 / 对象存储 ❌ 鉴权 / 重试 / 一致性
❌ 跨 Host 共享状态 ❌ 改 `wit/capability.wit` ❌ 重编译任何 component
❌ 顺手修 `domain.rs` / `compose.rs` 的同类 epoch 债务

## 后续 · 跨端 Domain Components

把 Button 扩成真正成体系的域组件；验证组件间组合与前后端一致的行为。
若要自动化 Web 验收，此时引入 Playwright。

## P4 · Component Registry

> ⚠️ **本节已被重新校准的路线取代**（P4 改为 Durable Capability，`P4-0` 即其 preflight）。
> 发现 / 版本 / 分发的问题没有消失，但不再是 P4 的定义。本节留待 P4-1 设计 gate 时统一整理。

组件的发现 / 版本 / 分发。注意：不要靠提交 `hosts/web/src/generated/` 来分发 ——
那是从契约派生的产物。正确做法是发布 npm 制品或 CI 上传构建产物。

## P5 · AI Agent（**冻结中**）

> ⚠️ **本节与上面的 P5 不是同一件事**。上面的 P5 是**重新校准后的路线**里的
> 「同一个 Domain Component，多个 Host」（已完成）；本节是**旧编号**下被冻结的 Agent 层，
> 保留原样作为历史记录。Agent 的定位（另一种 Component Consumer）没有变，
> 只是它在校准后的路线里**不再是 P5**。

`spark-host/src/agent.rs` 与 `spark-host/src/deepseek.rs` 已加模块级冻结注释：
代码保留、测试保持通过、**不再迭代**。`spark-host agent` 子命令仍可用，但帮助文本已标注属未来层。

**冻结理由**：Agent 不该是架构核心，它只是**另一种 Component Consumer** ——
与 Web 宿主、Rust 后端宿主同层。先把「同一份契约 + 同一个组件服务多端」证明扎实，
接 Agent 会自然得多。

**解冻条件**：P0–P4 的跨端契约有真实使用方，或确实需要 Agent 作为组件的消费者。
