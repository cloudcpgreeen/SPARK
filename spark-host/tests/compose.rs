//! P3：Capability 的实现也可以是一个 Component。制品缺失时跳过
//! （先 `cargo component build --release`，再 `./build-ui.sh` 生成 `dist/composed.wasm`）。
//!
//! 三件事**分开证**，不打包成一个结论：
//! ① Provider Component 能实现这个能力；
//! ② 组合把 import 消掉了（空 Linker 上能跑，裸 Consumer 在同一个空 Linker 上必须失败）；
//! ③ 代价是作用域——Provider 把状态放在自己的实例里，所以新实例读回 0。

use spark_host::compose::{click_times_selfcontained, storage_roundtrip};
use spark_host::Host;

const SKIP: &str =
    "skip: 组件未构建，先 `cd components/mem-store && cargo component build --release`";
const SKIP_COMPOSED: &str = "skip: 组合产物不存在，先 `./build-ui.sh`";

fn artifact(rel: &str) -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    p.exists().then(|| p.to_string_lossy().into_owned())
}

const PROVIDER: &str =
    "../components/mem-store/target/wasm32-unknown-unknown/release/mem_store.wasm";
const CONSUMER: &str =
    "../components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm";
const COMPOSED: &str = "../dist/composed.wasm";

/// ① **Provider Component 能实现这个能力**：单独实例化 `mem_store.wasm`，
/// `set(k, v)` 之后 `get(k)` == `Some(v)`。
#[test]
fn provider_component_implements_the_capability() {
    let Some(wasm) = artifact(PROVIDER) else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();

    // 写进去、读回来 —— Provider 确实实现了这个能力。
    assert_eq!(
        storage_roundtrip(&host, &wasm, "k", "v").unwrap(),
        Some("v".to_string())
    );
    // 空串是 Ok(Some(""))，不是 Ok(None)：契约里「有值但为空」与「没有这个 key」可区分。
    assert_eq!(
        storage_roundtrip(&host, &wasm, "empty", "").unwrap(),
        Some(String::new())
    );
}

/// ② **组合消除了 import**：`composed.wasm` 跑在**空 Linker** 上 —— 点 3 次 → 3。
///
/// 空 Linker 是这个结论的全部证据：组件只要还 import 任何东西，这里就会失败。
#[test]
fn composed_runs_on_an_empty_linker() {
    let Some(wasm) = artifact(COMPOSED) else {
        eprintln!("{SKIP_COMPOSED}");
        return;
    };
    let host = Host::new().unwrap();

    assert_eq!(click_times_selfcontained(&host, &wasm, 3).unwrap(), 3);
}

/// ② 的**对照组**：裸 `counter-store.wasm`（P2 的那份，未重新编译）在同一个空 Linker 上
/// 必须失败。没有这一条，「这条路是空的」就没被证明 —— 可能只是 Linker 没起作用。
#[test]
fn the_bare_consumer_still_needs_a_linker() {
    let Some(wasm) = artifact(CONSUMER) else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();

    let err = click_times_selfcontained(&host, &wasm, 3).unwrap_err();
    assert!(
        err.to_string().contains("spark:capability/storage@0.1.0"),
        "应当因缺少 capability 实现而失败，实际: {err:#}"
    );
}

/// ③ **代价是作用域**：同一个 `composed.wasm`，一次调用点了 3 次（实例内数到 3），
/// 另一次调用是**全新实例**，从能力里读回 **0** —— 对照 P2 的 Host 实现读到 3。
///
/// 这是**本次实现的后果**（Provider 把状态放在自己的实例内存里），
/// **不是**「Component Capability 天生只能 instance-scoped」。
#[test]
fn the_capability_is_instance_scoped_in_this_implementation() {
    let Some(wasm) = artifact(COMPOSED) else {
        eprintln!("{SKIP_COMPOSED}");
        return;
    };
    let host = Host::new().unwrap();

    assert_eq!(click_times_selfcontained(&host, &wasm, 3).unwrap(), 3);
    assert_eq!(click_times_selfcontained(&host, &wasm, 0).unwrap(), 0);
}
