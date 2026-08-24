use std::process::ExitCode;

use spark_host::{discover_plugins, run_plugin};

const PLUGINS_DIR: &str = "plugins";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "list" => list(),
        [cmd, name, input] if cmd == "run" => run_named(name, input),
        [wasm, input] => run_path(wasm, input),
        _ => {
            eprintln!("usage: spark-host <plugin.wasm> <input> | run <name> <input> | list");
            ExitCode::from(2)
        }
    }
}

/// 发现并列出 `plugins/` 下的插件（name = 组件自注册名）。
fn list() -> ExitCode {
    let found = discover_plugins(PLUGINS_DIR);
    if found.is_empty() {
        eprintln!("未发现插件：把 .wasm 组件放进 {PLUGINS_DIR}/ 即注册");
        return ExitCode::SUCCESS;
    }
    for (file, info) in &found {
        println!(
            "{:<10} {:<6} {:<22} ({file})",
            info.name, info.version, info.description
        );
    }
    ExitCode::SUCCESS
}

/// 按 `info().name` 解析并运行插件。
fn run_named(name: &str, input: &str) -> ExitCode {
    let found = discover_plugins(PLUGINS_DIR);
    let Some((file, _)) = found.iter().find(|(_, info)| info.name == name) else {
        let names: Vec<_> = found.iter().map(|(_, info)| info.name.as_str()).collect();
        eprintln!(
            "未找到插件 `{name}`；{PLUGINS_DIR}/ 下可发现: {}",
            names.join(", ")
        );
        return ExitCode::from(1);
    };
    run_path(&format!("{PLUGINS_DIR}/{file}"), input)
}

/// 直接给组件路径运行。
fn run_path(wasm: &str, input: &str) -> ExitCode {
    match run_plugin(wasm, input) {
        Ok((info, Ok(out))) => {
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            println!("output: {out}");
            ExitCode::SUCCESS
        }
        Ok((info, Err(msg))) => {
            // 插件声明式失败：结果是 err，不是崩溃。
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            eprintln!("plugin error: {msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            // 插件 trap = 可控错误：宿主不崩。
            eprintln!("plugin trap: {e}");
            ExitCode::SUCCESS
        }
    }
}
