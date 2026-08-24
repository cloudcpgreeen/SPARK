//! 插件流水线：输出串联、fail-fast、结构化错误 / trap 定位到具体步骤。
//! 组件缺失时跳过（先 `cargo component build --release`）。

use std::path::PathBuf;

use spark_host::{Host, PipeFailure};

fn built_wasm(plugin_dir: &str) -> Option<PathBuf> {
    let wasm_name = format!("{}.wasm", plugin_dir.replace('-', "_"));
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../{plugin_dir}/target/wasm32-unknown-unknown/release/{wasm_name}"
    ));
    p.exists().then_some(p)
}

fn setup_plugins(label: &str, dirs: &[&str]) -> String {
    let tmp = std::env::temp_dir().join(format!("spark-pipe-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    for d in dirs {
        std::fs::copy(
            built_wasm(d).unwrap(),
            tmp.join(format!("{}.wasm", d.replace('-', "_"))),
        )
        .unwrap();
    }
    tmp.to_string_lossy().into_owned()
}

#[test]
fn pipe_threads_output_to_next() {
    if ["spark-plugin", "spark-plugin-reverse"]
        .iter()
        .any(|d| built_wasm(d).is_none())
    {
        eprintln!("skip: 组件未构建");
        return;
    }
    let dir = setup_plugins("thread", &["spark-plugin", "spark-plugin-reverse"]);
    let host = Host::new().unwrap();
    let out = host.pipe(&dir, "hello", &["upper", "reverse"]).unwrap();
    assert_eq!(out, "OLLEH"); // upper 转大写 → reverse 倒序
}

#[test]
fn pipe_fails_fast_on_declared_error() {
    if ["spark-plugin-idcard", "spark-plugin-luhn"]
        .iter()
        .any(|d| built_wasm(d).is_none())
    {
        eprintln!("skip: 组件未构建");
        return;
    }
    let dir = setup_plugins("declined", &["spark-plugin-idcard", "spark-plugin-luhn"]);
    let host = Host::new().unwrap();
    // idcard 通过（输出非数字串）→ luhn 在第二步因长度失败，fail-fast 定位到 luhn。
    match host.pipe(&dir, "110101199001010015", &["idcard", "luhn"]) {
        Err(PipeFailure::Declined { step, error }) => {
            assert_eq!(step, "luhn");
            assert_eq!(error.code, "length");
        }
        other => panic!("应 fail-fast 于 luhn: 得 {other:?}"),
    }
}

#[test]
fn pipe_fails_fast_on_trap() {
    if ["spark-plugin", "spark-plugin-reverse"]
        .iter()
        .any(|d| built_wasm(d).is_none())
    {
        eprintln!("skip: 组件未构建");
        return;
    }
    let dir = setup_plugins("trap", &["spark-plugin", "spark-plugin-reverse"]);
    let host = Host::new().unwrap();
    match host.pipe(&dir, "trap-x", &["upper", "reverse"]) {
        Err(PipeFailure::Trap { step, detail }) => {
            assert_eq!(step, "upper");
            assert!(
                detail.contains("wasm backtrace"),
                "trap 细节应含 wasm backtrace: {detail}"
            );
        }
        other => panic!("trap 输入应 fail-fast：得 {other:?}"),
    }
}
