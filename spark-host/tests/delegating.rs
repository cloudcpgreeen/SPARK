//! P4-1：Capability 的**状态所有权**能否沿着 `Component → Provider → Host` 落回 Host。
//!
//! P3 的 `composed.wasm`（Provider 自己持有状态）新实例读回 0；
//! P4-1 的 `composed-delegating.wasm`（Provider 只转发，状态在 Host）应当读回 3。
//!
//! **两个测试都用 `click_times`（非空 Linker），这是对照组的关键**：
//! 同一个 Host、同一个 `Backend`、同一个 ns、同样点 3 次，唯一变量是 Provider。
//! 用 `click_times_selfcontained`（空 Linker）会把 Host 这个变量换掉，对照就不成立了。
//!
//! 一次 `click_times` 调用 = 一个 `Store` = 一个 Component instance，返回时前一个被 drop ——
//! 所以「实例 A 点 3 次 → 实例 B 读回」就是「销毁 A、创建 B」。

use spark_host::domain_store::{click_times, Backend};
use spark_host::Host;

/// Host 侧命名空间：与 CLI 的 `main.rs` 同一个值，保证组件发的是同一个裸 key。
const NS: &str = "counter-store";

const SKIP_P3: &str = "skip: dist/composed.wasm 不存在，先 `./build-ui.sh`";
const SKIP_DELEGATING: &str =
    "skip: dist/composed-delegating.wasm 不存在，见 CHANGELOG 的 P4-1 G2 命令";

fn artifact(rel: &str) -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    p.exists().then(|| p.to_string_lossy().into_owned())
}

const COMPOSED_P3: &str = "../dist/composed.wasm";
const COMPOSED_DELEGATING: &str = "../dist/composed-delegating.wasm";

/// **P4-1 的核心判据**：Provider 只做转发、自己不持有状态时，
/// 写入会一路走到 Host，并被**一个全新的实例**读到。
///
/// `stored: 1 条` 的位置在这里是 `backend.len()` —— 它是**独立于组件**的观测：
/// 值真的落在了 Host 的 `Backend` 里，而不是某个组件实例的内存里。
#[test]
fn a_delegating_provider_hands_the_state_to_the_host() {
    let Some(wasm) = artifact(COMPOSED_DELEGATING) else {
        eprintln!("{SKIP_DELEGATING}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    // 实例 A：点 3 次。`count()` 读的是它自己实例内的 Cell。
    assert_eq!(
        click_times(&host, &wasm, 3, NS, backend.clone()).unwrap(),
        3
    );
    // 实例 B：全新 Store、全新 Component instance。构造函数必须从 capability 读回来 ——
    // 而 capability 这一头连着的是 Host 的 Backend。
    assert_eq!(
        click_times(&host, &wasm, 0, NS, backend.clone()).unwrap(),
        3
    );
    // 独立观测：Host 的 Backend 里确实有一条（key = `counter-store:count`）。
    assert_eq!(backend.len(), 1);
}

/// **对照组**：P3 的 `composed.wasm` 走**同一个 Host**。
///
/// 它的 Provider（`mem-store`）把状态放在自己的实例内存里，一次都没碰过 Host，
/// 所以新实例读回 0，且 Host 的 Backend **一条都没有**。
///
/// 这一条与上一个测试合起来，才把变量锁死在「谁持有状态」上 ——
/// 单看 `reloaded: 3` 无法排除「Host 根本没参与、是别的东西记住了」。
#[test]
fn the_selfcontained_provider_never_reaches_the_host() {
    let Some(wasm) = artifact(COMPOSED_P3) else {
        eprintln!("{SKIP_P3}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    assert_eq!(
        click_times(&host, &wasm, 3, NS, backend.clone()).unwrap(),
        3
    );
    assert_eq!(
        click_times(&host, &wasm, 0, NS, backend.clone()).unwrap(),
        0
    );
    assert_eq!(backend.len(), 0);
}
