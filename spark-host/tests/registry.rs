//! 注册/发现：把 .wasm 组件放进目录即注册，宿主按 `info().name` 解析运行。
//! 组件缺失时跳过（先 `cd spark-plugin && cargo component build --release`，reverse/attacker 同）。

use std::path::PathBuf;

use spark_host::Host;

/// 解开 Ok；失败则 panic（PluginError 未实现 PartialEq，不能直接 assert_eq 整个 Result）。
fn expect_ok<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("期望成功，实际失败"),
    }
}

/// 已构建的组件路径（产物名 = 目录名连字符转下划线 + `.wasm`）。
fn built_wasm(plugin_dir: &str) -> Option<PathBuf> {
    let wasm_name = format!("{}.wasm", plugin_dir.replace('-', "_"));
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../{plugin_dir}/target/wasm32-unknown-unknown/release/{wasm_name}"));
    p.exists().then_some(p)
}

const PLUGINS: [&str; 5] = [
    "spark-plugin",
    "spark-plugin-reverse",
    "spark-plugin-attacker",
    "spark-plugin-idcard",
    "spark-plugin-luhn",
];

/// 建临时插件目录并拷入三个组件，返回目录路径。
fn setup_plugins(label: &str) -> String {
    let tmp = std::env::temp_dir().join(format!("spark-plugins-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    for dir in PLUGINS {
        std::fs::copy(
            built_wasm(dir).unwrap(),
            tmp.join(format!("{}.wasm", dir.replace('-', "_"))),
        )
        .unwrap();
    }
    tmp.to_string_lossy().into_owned()
}

#[test]
fn dropped_wasm_is_registered() {
    if PLUGINS.iter().any(|d| built_wasm(d).is_none()) {
        eprintln!("skip: 组件未构建，先 `cargo component build --release`（三插件同）");
        return;
    }
    let dir = setup_plugins("discover");
    let host = Host::new().unwrap();
    let found = host.discover(&dir);
    let mut names: Vec<_> = found.iter().map(|(_, info)| info.name.clone()).collect();
    names.sort();
    assert_eq!(names, ["attacker", "idcard", "luhn", "reverse", "upper"]);
}

#[test]
fn discovered_plugin_run_by_name() {
    if PLUGINS.iter().any(|d| built_wasm(d).is_none()) {
        eprintln!("skip: 组件未构建");
        return;
    }
    let dir = setup_plugins("run");
    let host = Host::new().unwrap();
    let found = host.discover(&dir);
    let (file, info) = found
        .into_iter()
        .find(|(_, info)| info.name == "reverse")
        .unwrap();
    let (rinfo, out) = host.run(&format!("{dir}/{file}"), "hello").unwrap();
    assert_eq!(rinfo.name, info.name);
    assert_eq!(expect_ok(out), "olleh");
}
