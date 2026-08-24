//! 端到端：真实插件组件加载 + 沙箱隔离 + 多插件可插拔。
//! 组件缺失时跳过（先 `cd spark-plugin && cargo component build --release`，reverse 同）。

use spark_host::run_plugin;

/// 插件目录名 → 组件路径（产物名 = 目录名连字符转下划线 + `.wasm`）。
fn component_path(plugin_dir: &str) -> Option<String> {
    let wasm = format!("{}.wasm", plugin_dir.replace('-', "_"));
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../{plugin_dir}/target/wasm32-unknown-unknown/release/{wasm}"));
    p.exists().then(|| p.to_string_lossy().into_owned())
}

const SKIP: &str = "skip: 组件未构建，先 `cd spark-plugin && cargo component build --release`";

#[test]
fn happy_path() {
    let Some(wasm) = component_path("spark-plugin") else {
        eprintln!("{SKIP}");
        return;
    };
    let (name, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(name, "upper");
    assert_eq!(out, "HELLO");
}

#[test]
fn trap_is_contained() {
    let Some(wasm) = component_path("spark-plugin") else {
        eprintln!("{SKIP}");
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
    let Some(wasm) = component_path("spark-plugin") else {
        eprintln!("{SKIP}");
        return;
    };
    assert!(run_plugin(&wasm, "trap-x").is_err());
    let (name, out) = run_plugin(&wasm, "ok").unwrap();
    assert_eq!(name, "upper");
    assert_eq!(out, "OK");
}

#[test]
fn second_plugin_same_host() {
    // 同一个宿主代码加载第二个插件：宿主零改动即插即用。
    let Some(wasm) = component_path("spark-plugin-reverse") else {
        eprintln!("skip: reverse 组件未构建，先 `cd spark-plugin-reverse && cargo component build --release`");
        return;
    };
    let (name, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(name, "reverse");
    assert_eq!(out, "olleh");
}
