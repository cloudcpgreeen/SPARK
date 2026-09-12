//! 端到端：`spark:ui` / `domain-world` 跨端域组件（headless Button 计数器）。
//! 组件缺失时跳过（先 `cd components/button && cargo component build --release`）。
//!
//! 这里证的是「同一个 `button.wasm` 跑在 Rust 后端」这一半；另一半是 Web 宿主
//! （`hosts/web`）跑同一份产物。两端不共享状态，只共享契约与行为。

use spark_host::domain::click_times;
use spark_host::Host;

const SKIP: &str = "skip: 组件未构建，先 `cd components/button && cargo component build --release`";

/// button 组件路径（目录名 `button` → 产物 `button.wasm`）。
fn component_path() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../components/button/target/wasm32-unknown-unknown/release/button.wasm");
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[test]
fn click_accumulates() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    assert_eq!(click_times(&host, &wasm, 0).unwrap(), 0);
    assert_eq!(click_times(&host, &wasm, 3).unwrap(), 3);
}

/// 每次调用都是新 Store + 新实例：状态住在组件实例里，宿主不持有，实例互不污染。
#[test]
fn instances_do_not_share_state() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    assert_eq!(click_times(&host, &wasm, 5).unwrap(), 5);
    // 上一次点了 5 次，这一次从 0 开始 —— 不是 6。
    assert_eq!(click_times(&host, &wasm, 1).unwrap(), 1);
}
