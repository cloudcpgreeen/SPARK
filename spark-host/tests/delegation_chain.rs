//! P4-2：Capability 经过**多个**纯委派 Provider 之后，状态所有权是否仍在 Host。
//!
//! P4-1 证了 `counter-store → delegating-store(A) → Host`：状态落在 Host，`reloaded: 3`。
//! P4-2 在链上再叠一层 Provider B（`forwarding-store`，world 与 A 逐字相同），
//! 问的是：**多一条委派边界，会不会把所有权搬走。**
//!
//! **两个测试都用 `click_times`（非空 Linker）**，因为要对照的变量是「Provider 链的形状」，
//! 不是「有没有 Host」。用 `click_times_selfcontained`（空 Linker）会把这个变量换掉。
//!
//! 一次 `click_times` 调用 = 一个 `Store` = 一个 Component instance。返回时前一个被 drop，
//! 所以「实例 A 点 3 次 → 实例 B 读回」就是「销毁 A、创建 B」。
//!
//! **实例拓扑的准确说法**（别写成 Host 分别实例化 A/B）：组合产物里 A 与 B 是**内联**的，
//! Host 只实例化一个组合组件。所以链是
//! `fresh Store → fresh composed Component instance → 其内部包含 fresh 的 A、B 实例`。

use spark_host::domain_store::{click_times, Backend};
use spark_host::Host;

/// Host 侧命名空间：与 CLI 的 `main.rs` 同一个值，保证组件发的是同一个裸 key。
const NS: &str = "counter-store";

const SKIP_1HOP: &str = "skip: dist/composed-1hop.wasm 不存在，见 CHANGELOG 的 P4-2 G4 命令";
const SKIP_2HOP: &str = "skip: dist/composed-2hop.wasm 不存在，见 CHANGELOG 的 P4-2 G4 命令";

const COMPOSED_1HOP: &str = "../dist/composed-1hop.wasm";
const COMPOSED_2HOP: &str = "../dist/composed-2hop.wasm";

fn artifact(rel: &str) -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    p.exists().then(|| p.to_string_lossy().into_owned())
}

/// **P4-2 的核心判据**：链上多一层纯委派 Provider，状态仍然一路走到 Host，
/// 并被**一个全新的实例**读到。
///
/// `backend.len()` 是**独立于组件**的观测：值真的落在 Host 的 `Backend` 里，
/// 而不是某个组件实例（或它内联的 A/B）的内存里。
#[test]
fn two_hop_delegation_keeps_the_state_at_the_host() {
    let Some(wasm) = artifact(COMPOSED_2HOP) else {
        eprintln!("{SKIP_2HOP}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    // 实例 A：点 3 次。写入要穿过 B、再穿过 A，才可能到 Host。
    assert_eq!(
        click_times(&host, &wasm, 3, NS, backend.clone()).unwrap(),
        3
    );
    // 实例 B：全新 Store、全新组合组件实例（其内部 A、B 也全是新的）。
    // 构造函数必须从 capability 读回来 —— 而 capability 这一头连着 Host。
    assert_eq!(
        click_times(&host, &wasm, 0, NS, backend.clone()).unwrap(),
        3
    );
    assert_eq!(backend.len(), 1);
}

/// **G6 的强版本**：一跳与两跳**共享同一个 `Backend` 实例**。
///
/// 先用 `composed-1hop`（Domain → B → Host）写入 3，再让 `composed-2hop`
/// （Domain → A → B → Host）的**全新实例**去读同一个 `Backend` —— 直接读到 3。
///
/// 这比「两条命令各自独立跑都得到 3」硬：它证明**两个制品指向同一个 Host 状态位置**，
/// 而不是各自在别处碰巧记住了 3。两个制品共享同一个 `counter-store.wasm`、同一个 Host、
/// 同一个 `Backend`、同一个 ns、同一个 click protocol；唯一的架构变量是
/// **是否增加了 Provider A 这一条委派边界**。
#[test]
fn adding_a_provider_hop_does_not_move_the_state() {
    let (Some(one), Some(two)) = (artifact(COMPOSED_1HOP), artifact(COMPOSED_2HOP)) else {
        eprintln!("{SKIP_2HOP} / {SKIP_1HOP}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    // 一跳写 3。
    assert_eq!(click_times(&host, &one, 3, NS, backend.clone()).unwrap(), 3);
    assert_eq!(backend.len(), 1);

    // 两跳的**全新实例**读同一个 Backend：同一把钥匙（`counter-store:count`）→ 3。
    assert_eq!(click_times(&host, &two, 0, NS, backend.clone()).unwrap(), 3);
    // 没有多出第二个键 —— 两条链落在同一个状态位置，不是各写各的。
    assert_eq!(backend.len(), 1);
}
