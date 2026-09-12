use std::process::ExitCode;

use spark_host::agent::{run_agent, AlgorithmPredictor, Predictor, MAX_STEPS};
use spark_host::deepseek::DeepSeekPredictor;
use spark_host::domain_store::{click_times, Backend};
use spark_host::{Host, PipeFailure};

const PLUGINS_DIR: &str = "plugins";

/// Host 侧的 capability 命名空间：隔离策略属于宿主，组件发的是裸 key。
const NS: &str = "counter-store";

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
        [cmd, prompt, rest @ ..] if cmd == "agent" => agent(&host, prompt, rest),
        [cmd, wasm, n] if cmd == "domain" => domain(&host, wasm, n),
        [cmd, wasm, n, rest @ ..] if cmd == "store" => store(&host, wasm, n, rest),
        [wasm, input] => run_path(&host, wasm, input),
        _ => {
            eprintln!(
                "usage: spark-host <plugin.wasm> <input> | run <name> <input> | pipe <input> <name>... | list | domain <button.wasm> <n> | store <counter-store.wasm> <n> [--deny] | agent <prompt> [--model flash|pro] (agent 属 P5 未来层)"
            );
            ExitCode::from(2)
        }
    }
}

/// Agent 命令（**P5 · 未来层，已冻结**）：`spark-host agent "<prompt>" [--model flash|pro]`。
/// 不带 `--model` = 本地算法预测（离线，无需 Key）；带 `--model` = DeepSeek harness（需 `DEEPSEEK_API_KEY`）。
/// 保留可用，但不在 P1 主线上迭代 —— Agent 只是另一种 Component Consumer，见 ROADMAP.md。
fn agent(host: &Host, prompt: &str, rest: &[String]) -> ExitCode {
    let mut model = String::new();
    let mut i = 0;
    while i < rest.len() {
        match (rest[i].as_str(), rest.get(i + 1)) {
            ("--model", Some(m)) => {
                model = m.clone();
                i += 2;
            }
            (other, _) => {
                eprintln!("未知参数: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let predictor: Box<dyn Predictor>;
    if model.is_empty() {
        predictor = Box::new(AlgorithmPredictor);
    } else {
        let key = match std::env::var("DEEPSEEK_API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                eprintln!("--model {model} 需要 DEEPSEEK_API_KEY 环境变量（Key 只走环境变量，不进 prompt/日志）");
                return ExitCode::from(2);
            }
        };
        match DeepSeekPredictor::new(key, &model) {
            Ok(h) => {
                predictor = Box::new(h);
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        }
    }
    let result = run_agent(host, PLUGINS_DIR, prompt, predictor.as_ref(), MAX_STEPS);
    for call in &result.calls {
        eprintln!("  → {}", call.rendered);
    }
    println!("{}", result.answer);
    ExitCode::SUCCESS
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
            eprintln!(
                "✗ 未通过 {step} [{code}]: {message}",
                code = error.code,
                message = error.message
            );
            ExitCode::from(1)
        }
        Err(PipeFailure::Trap { step, detail }) => {
            eprintln!("✗ {step} 崩溃被沙箱捕获: {detail}");
            ExitCode::from(1)
        }
    }
}

/// 域组件命令：`spark-host domain <button.wasm> <n>` —— 沙箱内构造 counter，点 `n` 次，打印 count。
/// 与 Web 宿主对照：同一个 `button.wasm`，各宿主各有实例与状态，共享的是契约与行为。
fn domain(host: &Host, wasm: &str, n: &str) -> ExitCode {
    let Ok(clicks) = n.parse::<u32>() else {
        eprintln!("clicks 必须是 u32: {n}");
        return ExitCode::from(2);
    };
    match spark_host::domain::click_times(host, wasm, clicks) {
        Ok(count) => {
            println!("count: {count}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("domain trap: {e}");
            ExitCode::SUCCESS
        }
    }
}

/// 域组件 + capability 命令：`spark-host store <counter-store.wasm> <n> [--deny]`。
///
/// 点 `n` 次，然后**新建一个实例**看它读到什么 —— 后者才是 capability 是否生效的证据：
/// 健康存储 → 新实例读到 n；`--deny`（拒绝写入）→ 新实例读到 0。
/// 两次实例化用的是**同一份未被重新编译的 wasm**。
fn store(host: &Host, wasm: &str, n: &str, rest: &[String]) -> ExitCode {
    let Ok(clicks) = n.parse::<u32>() else {
        eprintln!("clicks 必须是 u32: {n}");
        return ExitCode::from(2);
    };
    let deny = match rest {
        [] => false,
        [flag] if flag == "--deny" => true,
        [other, ..] => {
            eprintln!("未知参数: {other}");
            return ExitCode::from(2);
        }
    };
    let backend = if deny {
        Backend::read_only()
    } else {
        Backend::new()
    };
    match click_times(host, wasm, clicks, NS, backend.clone()) {
        Ok(count) => {
            println!("count: {count}");
            match click_times(host, wasm, 0, NS, backend.clone()) {
                Ok(reloaded) => {
                    println!("reloaded: {reloaded}");
                    println!("stored: {} 条", backend.len());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("store trap: {e}");
                    ExitCode::SUCCESS
                }
            }
        }
        Err(e) => {
            eprintln!("store trap: {e}");
            ExitCode::SUCCESS
        }
    }
}

/// 直接给组件路径运行。
fn run_path(host: &Host, wasm: &str, input: &str) -> ExitCode {
    match host.run(wasm, input) {
        Ok((info, Ok(out))) => {
            println!(
                "plugin: {} {} — {}",
                info.name, info.version, info.description
            );
            println!("output: {out}");
            ExitCode::SUCCESS
        }
        Ok((info, Err(error))) => {
            // 插件声明式失败：结果是 err，不是崩溃。
            println!(
                "plugin: {} {} — {}",
                info.name, info.version, info.description
            );
            eprintln!(
                "plugin error [{code}]: {message}",
                code = error.code,
                message = error.message
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            // 插件 trap = 可控错误：宿主不崩。
            eprintln!("plugin trap: {e}");
            ExitCode::SUCCESS
        }
    }
}
