# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)。

## [未发布]

## P5 · Same Domain Component, Multiple Hosts

> **核心问题**：同一个 Headless Domain Component，能否在不同 Host 上运行，
> 并保持相同的 Domain 语义？
> **不是**「两个 Host 共享状态」—— 恰恰相反：同一个不可变 artifact 被两个不同 Host
> **独立**承载，Domain 语义相同，而 Host Capability 实现与状态可以不同。

```
P1    Component → Host                                  PASS / FROZEN
P2    Component → Capability → Host                     PASS / FROZEN
P3    Component → Capability ← Component                PASS / FROZEN
P4    Provider chain → Host-owned State                 PASS / FROZEN  4997bba
P5    SAME Component → DIFFERENT Hosts                  PASS ← 本轮
```

### P5 与 P1 的区别（不写这一句，P5 会被读成 P1 的复述）

P1 已经证明过「同一份 artifact、两个 Host」—— 但 `button.wasm` 是**零 import** 的，
它根本没碰到 Host。P5 新增的是 **capability 维度**：同一个 artifact **import 一个
capability**，两个 Host 各给**不同实现**，而 Domain 行为一致。

**P5 证的是 Capability abstraction survives host substitution。**

### 设计期发现

**① G1 按字面写法不成立，必须精确措辞。**
Web Host 从不加载 `counter_store.wasm` —— 它加载的是 jco 的**派生产物**。
能成立的精确版本是：**同一个冻结的 Component artifact 是两边共同的上游** ——
Rust **直接加载**它，Web **以它为转译输入**。
这与 P3 已冻结的 **derived artifact ≠ Component 源** 是同一条纪律，不是削弱。
（不这样写，就会重演 `--no-imports` 那次的「结论对 ≠ 证明理由对」。）

**② `build-ui.sh` 不能用来做 P5 的 Web 构建。** 它第 16 行无条件跑
`cargo component build --release`（与 G1「禁止重新编译」冲突，且每次运行都可能覆盖
冻结字节），第 51 行 `rm -rf hosts/web/src/generated` 会删掉产物。
⇒ P5 有自己的脚本与输出目录（`hosts/web/src/generated-p5/`），**`build-ui.sh` 零 diff**。

**③ 在 P5 之前，G1 这个值没有被任何东西断言过。**
`build-ui.sh:32,46-52` 只是把 `counter-store.wasm` 和它在**同一次运行内**比对
（证明的是「compose 不改消费者」，不是「等于这个常量」）；仓库里所有 `85691b8e…`
都出现在文档里，手抄、还截断了；Rust 侧唯一的 `sha256` 字样在
`spark-host/src/compose.rs:6`，是一句注释。
新增 `verify-artifact.sh` 把它变成会失败的断言。

**④ P1–P3 的 Web 验收从来没有自动化过。**
`hosts/web/package.json` 只有 `dev`/`build`/`preview`，CI 两个 job 全是 Rust；
所有「真实 Chrome：点 3 次 → 3」都是人眼看、手写进文档的散文。
本轮第一次把它变成可断言的证据。**没有回头修改 P1–P3 的历史记录。**

### 证据

#### G1 — Artifact identity（**断言，不是打印**）

`verify-artifact.sh` 在**转译前**断言，在**转译后**再断言一次：

```
OK: counter_store.wasm sha256 = 85691b8e50549e0608c893ef91836fb3f5056fa380e1e34236a9a93c33e7e584
（... jco transpile ...）
OK: counter_store.wasm sha256 = 85691b8e50549e0608c893ef91836fb3f5056fa380e1e34236a9a93c33e7e584
```

顺序本身就是证据：**同一个字节流是这次转译的输入**。

#### G2 — Rust Host

```bash
cargo run -p spark-host -- store \
  components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm 3
# count: 3 / reloaded: 3 / stored: 1 条
```

`store` 子命令**即使 trap 也返回 SUCCESS**，所以证据是匹配字面量，不是退出码。

#### G3 / G4 — Web Host（自动化真浏览器）+ Domain 语义一致

`hosts/web` 新增 `@playwright/test` 与 `p5.html` **独立入口**（不改 `App.tsx` ——
它是 P1–P3 的证据载体）：

```
npx playwright test
✓ G3/G4 · 同一个 artifact 在 Web Host 上跑出与 Rust Host 相同的 Domain 行为
✓ G5 · deny 反事实：Domain 行为不变，只有持久性改变
2 passed
```

`fresh BrowserContext` ⇒ localStorage 天然为空，且**把「初始渲染 0 + key 为 null」
本身写成 hermeticity 断言**，不是约定。

#### G5 — Capability 分离（**本轮最有价值的 gate**）

两端都用**同一个、未重新编译的** artifact：

```
Rust Host（Backend::read_only）
  normal → count: 3 / reloaded: 3 / stored: 1 条
  deny   → count: 3 / reloaded: 0 / stored: 0 条

Web Host（?deny=1，宿主侧的第二种 Capability 配置）
  normal → 0 → click×3 → 3 → localStorage '3' → reload → 3
  deny   → 0 → click×3 → 3   ← 硬判据第一步：Domain 行为仍成立
                         → localStorage 仍为 null  ← 写入从未落地
                         → reload → 0
```

**deny 下 `click ×3 → count 3` 仍然成立。** 所以被隔离出来的是
「Domain 操作」与「Capability 状态机制」两个变量，
**不是** *Capability 出错 → Component 出错*。

**证据责任的归属（不许混淆）**：G3 的 `reload → 3` **只证明** Web Host 的
localStorage Capability 在工作；**Domain 与 Capability 的分离由 G5 的反事实承担**。

#### G6 — 不计入 PASS 含金量

Rust 的进程内 `HashMap` 与浏览器 `localStorage` 不共享地址空间、存储介质与运行时，
**结构上不可能失败**。保留为 **structural guarantee / sanity boundary**，
**不包装成实验发现**。

#### 计划外的 regression 证据

`p5.html` 是新增的第二个 Vite 入口，实际在浏览器里验证了 P1–P3 的 demo 没有回归：

```
index.html 初始三段 count: [0, 0, 0]
P2 点 3 次后:            [0, 3, 0]   localStorage counter-store:count = 3
P2 刷新后:               [0, 3, 0]
p5.html count: 0                     ← ns 是 p5-counter-store，与 P2 互不干扰
```

不作为正式 gate，也没有为了它回去改 P1–P3 的记录。

### 结论（P5 PASS）

> 同一个冻结的 Domain Component artifact（`counter-store.wasm`，SHA-256 `85691b8e…`，
> 未经重新编译）被两个不同的 Host 承载：Rust/Wasmtime **直接加载**它，Web/jco
> **以它为转译输入**得到 Host 侧适配产物。两边的 Domain 行为一致
> （各自新建实例 → click ×3 → 3），而两边提供的 Capability 实现不同
> （进程内 HashMap vs 浏览器 localStorage）。

> G5 的反事实进一步表明，在两端更换为宿主侧 deny Capability 后，Domain 的
> `click ×3 → count 3` 仍成立，但状态不再跨 fresh instance 保留（`reload → 0`）。
> 因此，本次验证证明的是 **Domain 行为与 Capability 状态机制的分离**，
> 而非 *Capability failure 导致 Domain 调用失败*。

**核心一句**：

> 同一个 Domain artifact，不要求同一个 Host，也不要求同一个 Capability implementation；
> **Domain 与 Host Capability 的边界才是可移植性的核心。**

**明确不许推出**：

- ❌ 任意 Host 都可以承载 —— 本次验证的是这两个。
- ❌ RN 已经通过 —— RN 只有编译门、没有 runtime proof，仍是独立的 Runtime Gate。
- ❌ 同一份 Component 字节直接在浏览器中执行 —— 浏览器执行的是 jco 的**派生产物**。
- ❌ 跨 Host 共享状态 —— 恰恰相反，P5 明确不共享（那会变成 Remote Capability）。
- ❌ 「所有 Domain 在所有 Host 上都一致」—— 本次验证的是 counter 这一个 Domain 的行为。

### 新增

| 文件 | 内容 |
| --- | --- |
| `verify-artifact.sh` | G1：冻结 artifact 的 sha256 断言 |
| `p5-web.sh` | P5 的 Web 构建（先断言 → 再 `jco transpile` → 再断言），刻意不走 `build-ui.sh` |
| `hosts/web/p5.html`、`src/p5-main.tsx`、`src/P5App.tsx` | P5 的独立入口与页面 |
| `hosts/web/src/capability/storage-p5.js` | P5 的 Web capability 实现：ns `p5-counter-store` + 宿主级 deny 开关 |
| `hosts/web/playwright.config.ts`、`e2e/p5.spec.ts` | Web 验收自动化（2 个测试） |

`hosts/web/src/generated-p5/` 是 derived artifact，不入库。
**没有新增 Rust 测试**（46 个不变）；既有 `domain_store.rs` 已覆盖同一契约。

### P5 明确不做

**Web 不进 CI。** P5 证明的是**架构命题**，CI integration 是**工程化命题**。
现在已经有 `test:p5` + Playwright + fresh BrowserContext + headless Chromium，
足以形成可重复的本地证据。要把 Web 验收变成 CI gate，应另开一个很小的
**P5-CI / Web Verification Gate**，而不是偷偷塞进 P5。

## P4-2 · Multi-hop Capability Delegation

> **唯一的核心问题**：Capability 经过**多个**纯委派 Provider Component 之后，
> 状态是否仍由 Host 持有，并跨越整个 Provider 链的实例替换而保持？
> **P4-2 唯一新增的变量是委派链上多一条 Provider 边界。**

```
P1    Component → Host                              PASS / FROZEN
P2    Component → Capability → Host                 PASS / FROZEN
P3    Component → Capability ← Component            PASS / FROZEN
P4-0  import + export same Capability               PASS / FROZEN  9289898
P4-1  Provider A → Host                             PASS / FROZEN  78b817c
P4-2  Provider A → Provider B → Host                PASS ← 本轮
```

### 设计期发现：`wasm-tools compose` **不做传递闭包**

动手前先做了只读探针（输出到 stdout，未写任何文件）：

```
A = compose(counter-store -d delegating-store)                → 顶层 1 import + 1 export
B = compose(counter-store -d delegating-store -d mem-store)   → 与 A **字节相同**（sha 1aeeaea1…）
C = compose(counter-store -d mem-store -d delegating-store)   → 顶层 0 import（= P3 的形状）
```

**A == B 而 A != C** ⇒ `-d` 的语义是：**按顺序取第一个能满足 root import 的定义，
不追那个定义自己的 import —— 定义是叶子。** 顺序敏感本身就是「首个匹配生效」的独立印证。

**后果**：一次 `-d A -d B` **造不出两跳链** —— A 接走 root 的 import，A 自己的 import
变成悬空的顶层 import，产物退化成 P4-1 的制品。

**方法：顺序组合两次**（同一个工具、同一个契约，只是施加两次）：

```
provider-chain-2hop = compose(delegating-store -d forwarding-store)    → import storage + export storage
composed-2hop       = compose(counter-store    -d provider-chain-2hop) → export counter-store + import storage
```

设计期**唯一未能只读验证**的一步（把中间制品当 `-d` 喂给第二次组合）**已实测通过**：
退出码 0，产物 world 正确。原本的推理依据 —— 该情形与 P4-1 的 compose 结构完全相同
（root 的 import 由 definition 的 export 满足，definition 自身另有一个未满足的 import）—— 成立。

### 新增

| 文件 | 内容 |
| --- | --- |
| `wit/forwarding-store.wit` | Provider B 的 world，与 `spark:delegating-store@0.1.0` **逐字相同** |
| `components/forwarding-store/` | 纯委派 Provider，源码是 `delegating-store` 的复制（约 40 行） |
| `spark-host/tests/delegation_chain.rs` | 2 个测试 |

**刻意不抽公共 crate**：只有两个实例，复制的成本低于抽象。二者一旦分化，说明实验控制被破坏。

### 验收（G1–G7，全绿）

| # | gate | 观测 | 结果 |
| --- | --- | --- | --- |
| G1 | Provider A = 已冻结的 `delegating-store` | 源码 sha256 `3c43b506…` 未变 | ✅ |
| G2 | 新增 Provider B | `forwarding_store.wasm` 构建成功 | ✅ |
| G3 | B 自己的形状 | `component wit` **恰好 2 行**：1 import + 1 export | ✅ |
| G4① | `compose(A -d B)` | 退出码 0；中间制品 world 恰好 2 行 | ✅ |
| G4② | 中间制品当 `-d` | 退出码 0；两个最终制品 world 逐字相同 | ✅ |
| G5 | 两跳生命周期 | `reloaded: 3` / `stored: 1 条` | ✅ |
| G6 | 反事实 | 一跳 = 两跳 = 3；P3 控制 = 0 | ✅ |
| G7 | 源码无状态声明 | grep → 无输出 | ✅ |

```
cargo run -p spark-host -- store dist/composed-1hop.wasm 3   # Domain → B → Host
# count: 3 / reloaded: 3 / stored: 1 条

cargo run -p spark-host -- store dist/composed-2hop.wasm 3   # Domain → A → B → Host
# count: 3 / reloaded: 3 / stored: 1 条

cargo run -p spark-host -- store dist/composed.wasm 3        # P3 控制（Provider 自持状态）
# count: 3 / reloaded: 0 / stored: 0 条
```

两个最终制品**共享同一个 `counter-store.wasm`、同一个 Host、同一个 `Backend`、
同一个 namespace（`counter-store`）、同一个 click protocol**；
**唯一的架构变量是：是否增加 Provider A 这一条委派边界。**
（**不**简写成「唯一变量只有是否存在 Provider A」—— 最终 Wasm artifact 当然不同，
这种简写会变成新的证据漏洞。）

**G6 的强版本（已进测试）**：一跳与两跳**共享同一个还没被换掉的 `Backend` 实例** ——
先用 `composed-1hop` 写 3，再用 `composed-2hop` 的全新实例去读同一个 `Backend`，直接读到 3，
且 `backend.len()` 仍是 1（没有多出第二个键）。这比「两条命令各自跑都得到 3」硬：
它证明**两个制品指向同一个 Host 状态位置**。

**实例拓扑的准确说法**：A 与 B 在组合产物里是**内联**的，Host 只实例化一个组合组件 ——

```
fresh Store → fresh composed Component instance → 其内部包含 fresh 的 A、B 实例
```

**不能**写成 Host 分别实例化 A、B —— 那会给 Component Model 的实例拓扑制造错误直觉。

### 结论（P4-2 PASS）

> 在本次验证的两级纯委派 Provider 链中，增加一个 Provider 委派边界**不会改变**
> Capability State 的 Host ownership；状态仍能跨越 Domain / Provider Component 实例替换而保持。

> In the two-level pure-delegation chain verified here, adding one more Provider delegation
> boundary **does not change** Host ownership of the Capability State; the state still survives
> replacement of the Domain / Provider Component instances.

**这条结论的边界（明确不许升级成）**：

- ❌ 「任意深度 Provider 链都保证 Host ownership」—— 两级证据支持不了任意多级。
- ❌ 「Component Model 保证 Provider 不持有状态」—— Provider 不持有状态是**本次实现的观测**，
  不是 Component Model 的定律（对照 P3：`mem-store` 就是持有状态的 Provider）。
- ❌ 「compose 支持多跳」—— 本次证据恰恰相反：**多跳靠顺序组合，工具本身只做一跳**。
- ❌ `durable` 扩大解释 —— 仍只指 *survives replacement of the Component / Provider instance*；
  不等于进程重启 / 宿主重启 / 磁盘 / 数据库，`Host process restart → 未测试`。

**P4-2 的真正产出**：State ownership 被从 Component Model 里单独剥离出来了 ——

```
Component identity  ≠  Provider identity  ≠  Capability state ownership
```

Provider 可以是一条又一条纯委派边界，最终状态仍落在 Host；**Provider 本身可以完全没有最终状态**。

### 工具行为事实（冻结）

> 当前 `wasm-tools compose` **不会在一次 compose 中自动递归追踪 Provider 的 imports**；
> 但可以通过 **sequential composition**，把多级 Provider 委派链**逐级内联**。

`wasm-tools compose` 在 1.245.1 中已 deprecated（提示改用 `wac`），本机 `wac` 不可安装
（需联网）。**如实记录为限制，不绕路。**

### P4 封板

P4-2 是 P4 的最后一步。**不做第三 / 第四层 Provider** —— 边际价值极低，
且会把「证明多级委派仍保持 Host ownership」逐渐变成穷举实验。P4 的边界已经足够清楚。

**P5 是另一个问题，不让 P4 的结论外溢到跨 Host** —— 需另开 Design Gate。

## P4-1 · Capability 的状态所有权落回 Host

> **唯一的核心问题**：当 Provider Component 自己不持有最终状态，而是把 Capability 继续向上委派时，
> 状态能否真正回到 Host，并跨越 Provider / Domain Component 实例生命周期保持？
> **P4-1 唯一新增的变量是 state ownership。**

```
counter-store → import storage → delegating-store → import storage → Host
```

### 新增

**`spark-host/tests/delegating.rs`**（2 个测试）—— 本轮**唯一**新增的文件。

**没有新增任何 Host 代码、WIT 或组件。** `store` 子命令（`main.rs:166`）本来就在跑
「实例 A 点 n 次 → 实例 B 全新读回」这个协议；而 `composed-delegating.wasm` 的 world 与
`store-world` 逐字相同（P4-0 G0.5/G0.6 已验），所以它直接就能吃，一行都不用改。

### 验收（G1–G6，全绿）

| # | gate | 观测 | 结果 |
| --- | --- | --- | --- |
| G1 | Provider 源码无状态声明 | `grep -E 'HashMap\|thread_local\|OnceLock\|Mutex\|RefCell\|Cell\|static'` → 无输出 | ✅ |
| G2 | 组合边界 | `component wit` **恰好 2 行**：1 import + 1 export，无第三条 | ✅ |
| G3 | Host binding 未变 | `git diff` 空；契约仍 `spark:capability@0.1.0` | ✅ **无独立观测项**，见下 |
| G4 | 状态落在 Host | `stored: 1 条`（= Host `Backend` 的 `len()`） | ✅ |
| G5 | 跨实例存活 | `reloaded: 3` | ✅ |
| G6 | 反事实控制 | **同一个 Host** 上 P3 的 `composed.wasm` → `reloaded: 0` / `stored: 0 条` | ✅ |

```
cargo run -p spark-host -- store dist/composed-delegating.wasm 3
# count: 3 / reloaded: 3 / stored: 1 条

cargo run -p spark-host -- store dist/composed.wasm 3        # 对照
# count: 3 / reloaded: 0 / stored: 0 条
```

一次 `store` 调用内部跑两次 `click_times` ⇒ 两个 `Store` ⇒ 两个 Component instance，
中间没有任何东西共享内存 —— **只有 `Arc<Backend>` 是共享的**。那就是「销毁 A、创建 B」。

**这个对照比 P3 的更强**：P3 的对照是「裸消费者 vs 空 Linker」；这里
**同一个 Host、同一个 `Backend`、同一个 ns、同样点 3 次，唯一变量是 Provider**。
`stored: 0 条` 是第二个独立观测 —— 它直接证明 Host **从未**看见那些写入。

### 结论（P4-1 PASS）

> A Provider Component may delegate a Capability to its Host **without owning the Capability state itself.**
> State written through the delegated Capability **survives replacement of the Domain / Provider
> Component instance**, because ownership remains at the Host boundary.

**P3 与 P4 的关系由此解释清楚**：

```
P3    Capability → Provider Component → Provider-local state → 实例销毁 → 0
P4-1  Capability → Provider Component → Host Capability → Host-owned state → 实例销毁 → 3
```

**P3 的 `reloaded: 0` 从「结论」降回「某个实现的后果」** —— 它从来不是 Component Model 的定律，
只是 `mem-store` 把状态放在自己实例里的后果。

### 措辞纪律：什么叫 durable

**durable = survives replacement of the Component / Provider instance.**

**不等于**：survives process restart / host restart / disk persistence / database durability。
**`Host process restart → 未测试`。** 以后真做磁盘/数据库，那是 P4-2，单独定义。

两处不许滑坡：
- **G3 没有独立的可观测项**，它的证据是 G4 的 `stored:` 那一行（Host 的 `backend.len()`）——
  不许把 G3 写成独立通过。
- **G1 的 grep 只证明源码里没有状态声明**，证明不了「Provider 是无状态的」
  （生成的 bindings 必然持有实例表之类）；行为的证据在 Host 一侧。

### P4-1 明确不做（如实记录）

- ❌ Web UI / RN / Remote Capability / async / `future<T>` / 数据库 / 网络 / Registry / CLI 扩展 / 新 Capability。
- ❌ 不改 `wit/capability.wit`、`counter-store`、`mem-store`、`delegating-store`、P1–P3 冻结制品与测试。
- `build-ui.sh` **未改**：脚本职责到「Web 宿主加载什么」为止，P4-1 没有 Web 宿主，等真有消费者再加。

### 已知粗糙处

- `dist/composed-delegating.wasm` **没有**进 `build-ui.sh`，靠上面那两条命令重现（dist/ 不入库）。

## P4-0 · Self-import / self-export preflight

> **P4 开工前的单点风险 gate。** 唯一的问题是：
> **一个 Component 能否同时 import 和 export 同一个 WIT interface，并被 `wasm-tools compose` 正确组合？**
> 三层（WIT 解析 / cargo-component 绑定生成 / wasm-tools compose）里任何一层不接受 → 停，不绕路。

### 新增

- **`wit/delegating-store.wit`**（`spark:delegating-store@0.1.0` / `delegating-provider-world`）：
  **同一个 interface 同时出现在 import 与 export 两侧**。**不 bump `spark:capability`** ——
  接口身份不变，P2 的 `counter-store.wasm` 与 P3 的 `mem-store.wasm` 才都不是孤儿。
- **`components/delegating-store`**：一个**既导出、又导入** `spark:capability/storage@0.1.0`
  的 Provider 组件，它自己不做存储、把能力**委派**给下一层。
  P4-0 只问工具链认不认这个形状，**不谈它委派给谁**（那是 P4-1）。

### 验收（gate G0.1–G0.6，全绿）

| # | 结果 | 判据 |
| --- | --- | --- |
| G0.1/G0.2 | WIT 层与 cargo-component 接受「双栖」 | `cargo component build --release` 退出码 0 → `delegating_store.wasm`（sha256 `a0d5ce2e…`） |
| G0.3 | 生成物的 world 结构 | world 里**同时**有 1 行 `import` 与 1 行 `export`，都是 `spark:capability/storage@0.1.0` |
| G0.4 | compose 成功 | `counter-store.wasm` + `delegating-store.wasm` → `dist/composed-delegating.wasm` |
| G0.5 | imports **== 预期集合** | 恰好 1 行：`import spark:capability/storage@0.1.0` |
| G0.6 | unexpected imports **== ∅** | 全集只有那 1 个 import + `export spark:store/counter-store@0.1.0` |

**G0.5/G0.6 比 P3 的 `imports == ∅` 更强**：两个方向都显式打印（该有的都在 / 不该有的一个都没有），
这才是在验证**边界没有偷偷扩大**，而不是只喊一句「相等」。

### 结论（P4-0 PASS）

> A Component **may** import and export the same WIT interface.
>
> `cargo-component` accepts both `import spark:capability/storage@0.1.0` and
> `export spark:capability/storage@0.1.0`; `wasm-tools compose` successfully composes
> `counter-store` + `delegating-store`; the resulting component preserves **exactly** the
> expected Capability boundary (`export spark:store/counter-store@0.1.0` /
> `import spark:capability/storage@0.1.0`); **no unexpected imports were observed**.
>
> Therefore `Component → import Capability → Provider Component → export Capability`
> is supported by the current toolchain.
> **Composition does not inherently eliminate the Capability boundary; it can move that
> boundary upward.**
>
> **P4-0 is proven. P4 is NOT proven.**

### P4-0 的两个实测发现

1. **同一个 interface 的 import 侧与 export 侧是两个不同的 Rust 类型。**
   `bindings::spark::capability::storage::StoreError` ≠
   `bindings::exports::spark::capability::storage::StoreError` —— 结构同、名字同、类型不同，
   委派必须在值这一层做一次转换（编译器 E0308 逼出来的）。
   **但模块路径不撞**（靠 `exports::` 前缀分开）。所以「工具链不接受同一 interface 双栖」这个担心不成立。
2. **组合把边界往上搬了一层，而不是消灭它。**
   P3 的 `composed.wasm` 零 import，是因为边界正好被 Provider 吃掉；
   P4-0 的 `composed-delegating.wasm` 露出 1 个 import，是因为 Provider 底下还站着一个 Provider ——
   同一个组合技巧，边界只是从更下一层透出来。

### P4-0 明确不做（如实记录）

- **不设计 P4-1**：不写 Host 绑定、不改 `build-ui.sh`、不加 CLI 子命令、不做 Web 区块。
- 不决定「外部 provider 是谁」；不碰 todo / RN / remote transport / async / 数据库。
- **未动** `components/mem-store`、`components/counter-store`、`wit/capability.wit`、
  P1–P3 宿主代码、`build-ui.sh`、REFERENCE.md。

### 已知粗糙处

- `build-ui.sh` 的 `--no-imports` 与 `REFERENCE.md` 里的同名字样**暂未改动** ——
  那属于 P3 封板脚本，不是 P4-0 的范围。该 flag 实测为 no-op，见 P3 的勘误。
- `wasm-tools compose` 已废弃（stderr 提示 `Please use wac instead`），
  但 `wac` 需联网安装、本机不可用 —— 与 P3 记录的约束相同。

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
| ⑤ | 零 import：**实测** | `composed.wasm` 经 `wasm-tools component wit` 观测为 **0 imports**，不是肉眼观察 |
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

**【P4-0 期勘误】⑤ 的原措辞「零 import **由工具强制**」是错的 —— 结论对，理由错。**
A/B 实测（同一输入、只差这一个 flag）：`wasm-tools compose` 带与不带 `--no-imports`
产出**字节完全相同**（sha256 均为 `1b8e5c07…`），且带 flag 的组合产物**仍保留 1 个 import** ——
该 flag 在 wasm-tools 1.245.1 中**未产生任何约束效果**，因此**不能**作为零 import 的因果依据。
零 import 是 compose **真的把 Provider 接上了**换来的，由 `wasm-tools component wit` 独立观测。
**这是证据纠偏，不是重新打开 P3**：结论不变、制品不变、脚本与测试不变、sha256 不变。

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
