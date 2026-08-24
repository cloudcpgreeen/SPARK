//! 端到端：真实插件组件加载 + 沙箱隔离 + 多插件可插拔。
//! 组件缺失时跳过（先 `cd spark-plugin && cargo component build --release`，reverse/attacker 同）。

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
    let (info, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(info.name, "upper");
    assert_eq!(info.version, "0.2.0");
    assert_eq!(out, Ok("HELLO".into()));
}

#[test]
fn declared_error_not_trap() {
    // 插件声明式失败（result 的 err）是值，不是 trap，更不是宿主崩溃。
    let Some(wasm) = component_path("spark-plugin") else {
        eprintln!("{SKIP}");
        return;
    };
    let (info, out) = run_plugin(&wasm, "err-x").unwrap();
    assert_eq!(info.name, "upper");
    let Err(msg) = out else {
        panic!("err 开头应返回声明式 err，而非 ok: {out:?}");
    };
    assert!(
        !msg.contains("wasm backtrace"),
        "声明式错误不是 trap: {msg}"
    );
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
    let (info, out) = run_plugin(&wasm, "ok").unwrap();
    assert_eq!(info.name, "upper");
    assert_eq!(out, Ok("OK".into()));
}

#[test]
fn second_plugin_same_host() {
    // 同一个宿主代码加载第二个插件：宿主零改动即插即用。
    let Some(wasm) = component_path("spark-plugin-reverse") else {
        eprintln!("skip: reverse 组件未构建，先 `cd spark-plugin-reverse && cargo component build --release`");
        return;
    };
    let (info, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(info.name, "reverse");
    assert_eq!(out, Ok("olleh".into()));
}

#[test]
fn attacker_cpu_bomb_cut_off() {
    // 恶意插件：正常输入不受影响；CPU 炸弹（死循环）被 epoch 时间预算切断，不挂死宿主。
    let Some(wasm) = component_path("spark-plugin-attacker") else {
        eprintln!("skip: attacker 未构建，先 `cd spark-plugin-attacker && cargo component build --release`");
        return;
    };
    let (info, out) = run_plugin(&wasm, "hello").unwrap();
    assert_eq!(info.name, "attacker");
    assert_eq!(out, Ok("hello".into()));
    let e = run_plugin(&wasm, "loop").unwrap_err();
    assert!(
        e.to_string().contains("wasm backtrace"),
        "死循环应以 wasm trap 切断而非挂死: {e}"
    );
}

#[test]
fn attacker_memory_bomb_cut_off() {
    // 恶意插件：内存炸弹（无限分配）被 StoreLimits 切断，宿主内存不被耗尽。
    let Some(wasm) = component_path("spark-plugin-attacker") else {
        eprintln!("skip: attacker 未构建");
        return;
    };
    let e = run_plugin(&wasm, "alloc").unwrap_err();
    assert!(
        e.to_string().contains("wasm backtrace"),
        "内存炸弹应以 wasm trap 切断: {e}"
    );
}
