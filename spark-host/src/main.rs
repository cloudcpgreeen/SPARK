use std::process::ExitCode;

use spark_host::{Host, PipeFailure};

const PLUGINS_DIR: &str = "plugins";

fn main() -> ExitCode {
    let host = match Host::new() {
        Ok(host) => host,
        Err(e) => {
            eprintln!("初始化宿主失败: {e:#}");
            return ExitCode::from(1);
        }
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "list" => list(&host),
        [cmd, name, input] if cmd == "run" => run_named(&host, name, input),
        [cmd, input, names @ ..] if cmd == "pipe" => pipe(&host, input, names),
        [wasm, input] => run_path(&host, wasm, input),
        _ => {
            eprintln!("usage: spark-host <plugin.wasm> <input> | run <name> <input> | pipe <input> <name>... | list");
            ExitCode::from(2)
        }
    }
}

/// 发现并列出 `plugins/` 下的插件（name = 组件自注册名）。
fn list(host: &Host) -> ExitCode {
    let found = host.discover(PLUGINS_DIR);
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
fn run_named(host: &Host, name: &str, input: &str) -> ExitCode {
    let found = host.discover(PLUGINS_DIR);
    let Some((file, _)) = found.iter().find(|(_, info)| info.name == name) else {
        let names: Vec<_> = found.iter().map(|(_, info)| info.name.as_str()).collect();
        eprintln!(
            "未找到插件 `{name}`；{PLUGINS_DIR}/ 下可发现: {}",
            names.join(", ")
        );
        return ExitCode::from(1);
    };
    run_path(host, &format!("{PLUGINS_DIR}/{file}"), input)
}

/// 插件流水线：输入依次过各插件，前一步输出喂下一步，任一步失败即停。
fn pipe(host: &Host, input: &str, names: &[String]) -> ExitCode {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    match host.pipe(PLUGINS_DIR, input, &names) {
        Ok(out) => {
            println!("output: {out}");
            println!("通过 {} 个插件", names.len());
            ExitCode::SUCCESS
        }
        Err(PipeFailure::Declined { step, error }) => {
            eprintln!("✗ 未通过 {step} [{code}]: {message}", code = error.code, message = error.message);
            ExitCode::from(1)
        }
        Err(PipeFailure::Trap { step, detail }) => {
            eprintln!("✗ {step} 崩溃被沙箱捕获: {detail}");
            ExitCode::from(1)
        }
    }
}

/// 直接给组件路径运行。
fn run_path(host: &Host, wasm: &str, input: &str) -> ExitCode {
    match host.run(wasm, input) {
        Ok((info, Ok(out))) => {
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            println!("output: {out}");
            ExitCode::SUCCESS
        }
        Ok((info, Err(error))) => {
            // 插件声明式失败：结果是 err，不是崩溃。
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            eprintln!("plugin error [{code}]: {message}", code = error.code, message = error.message);
            ExitCode::SUCCESS
        }
        Err(e) => {
            // 插件 trap = 可控错误：宿主不崩。
            eprintln!("plugin trap: {e}");
            ExitCode::SUCCESS
        }
    }
}
