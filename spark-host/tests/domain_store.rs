//! P2 Capability Contract Spike：`spark:store` / `store-world` —— 组件 import
//! `spark:capability/storage@0.1.0`，由宿主提供实现。组件缺失时跳过
//! （先 `cd components/counter-store && cargo component build --release`）。
//!
//! 观察方式：**新建实例读到什么**。健康存储 → 读到上次写进去的值；拒绝写入 → 读到 0。
//! 三个测试用的是同一份 wasm，**没有任何一次重新编译**。

use spark_host::domain_store::{click_times, Backend};
use spark_host::Host;

const SKIP: &str =
    "skip: 组件未构建，先 `cd components/counter-store && cargo component build --release`";

fn component_path() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm",
    );
    p.exists().then(|| p.to_string_lossy().into_owned())
}

/// 状态不再住在组件实例里：上一个实例写的值，新实例读得到。
#[test]
fn state_survives_a_new_instance() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    assert_eq!(
        click_times(&host, &wasm, 3, "counter-store", backend.clone()).unwrap(),
        3
    );
    // 全新实例、不点击：读到的就是 capability 里存下来的值。
    assert_eq!(
        click_times(&host, &wasm, 0, "counter-store", backend.clone()).unwrap(),
        3
    );
    assert_eq!(backend.len(), 1);
}

/// 命名空间隔离是 **Host 的实例策略**，不是契约语义：同一个后端换 ns 就读不到对方的值。
#[test]
fn namespaces_isolate() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::new();

    assert_eq!(
        click_times(&host, &wasm, 3, "alpha", backend.clone()).unwrap(),
        3
    );
    assert_eq!(
        click_times(&host, &wasm, 0, "alpha", backend.clone()).unwrap(),
        3
    );
    assert_eq!(
        click_times(&host, &wasm, 0, "beta", backend.clone()).unwrap(),
        0
    );
    assert_eq!(backend.len(), 1);
}

/// 错误路径：`set` 返回 `Err`，组件静默，但**下一个新实例读到 0** ——
/// `Err` 穿过组件 ABI 后产生了可观测结果。
///
/// 注意这只证明 *set failure is observable through a subsequent fresh instance*，
/// **不是**「错误处理已验证」：组件压根没处理 Err 分支。
#[test]
fn denied_writes_are_observable_through_a_fresh_instance() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    let backend = Backend::read_only();

    // 实例自己还是数到了 3（内存状态正常）。
    assert_eq!(
        click_times(&host, &wasm, 3, "counter-store", backend.clone()).unwrap(),
        3
    );
    // 但什么都没写进去，所以新实例从 0 起。
    assert_eq!(
        click_times(&host, &wasm, 0, "counter-store", backend.clone()).unwrap(),
        0
    );
    assert!(backend.is_empty());
}
