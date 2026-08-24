//! 端到端：真实插件组件加载 + 沙箱隔离。
//! 组件缺失时跳过（先 `cd spark-plugin && cargo component build --release`）。

use spark_host::run_plugin;

fn component_path() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm");
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[test]
fn happy_path() {
    let Some(wasm) = component_path() else {
        eprintln!("skip: 组件未构建，先 `cd spark-plugin && cargo component build --release`");
        return;
    };
    let (name, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(name, "upper");
    assert_eq!(out, "HELLO");
}

#[test]
fn trap_is_contained() {
    let Some(wasm) = component_path() else {
        eprintln!("skip: 组件未构建");
        return;
    };
    let e = run_plugin(&wasm, "trap-me").unwrap_err();
    assert!(
        e.to_string().contains("wasm backtrace"),
        "插件 trap 应以 wasm 层错误返回，而不是宿主崩溃: {e}"
    );
}

#[test]
fn isolated_after_trap() {
    let Some(wasm) = component_path() else {
        eprintln!("skip: 组件未构建");
        return;
    };
    assert!(run_plugin(&wasm, "trap-x").is_err());
    let (name, out) = run_plugin(&wasm, "ok").unwrap();
    assert_eq!(name, "upper");
    assert_eq!(out, "OK");
}
