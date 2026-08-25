//! Agent 回路离线测试：本地算法预测（AlgorithmPredictor）+ 沙箱工具调用，无网络。
//! 组件缺失时跳过（先对各插件 `cargo component build --release`）。

use std::path::PathBuf;

use spark_host::agent::{run_agent, AlgorithmPredictor, MAX_STEPS, TOOL_RESULT_LIMIT};
use spark_host::Host;

/// 已构建的组件路径（产物名 = 目录名连字符转下划线 + `.wasm`）。
fn built_wasm(plugin_dir: &str) -> Option<PathBuf> {
    let wasm_name = format!("{}.wasm", plugin_dir.replace('-', "_"));
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../{plugin_dir}/target/wasm32-unknown-unknown/release/{wasm_name}"
    ));
    p.exists().then_some(p)
}

const PLUGINS: [&str; 6] = [
    "spark-plugin",
    "spark-plugin-reverse",
    "spark-plugin-attacker",
    "spark-plugin-idcard",
    "spark-plugin-luhn",
    "spark-plugin-rmb",
];

/// 建临时插件目录并拷入全部组件，返回目录路径。
fn setup_plugins(label: &str) -> String {
    let tmp = std::env::temp_dir().join(format!("spark-agent-{label}-{}", std::process::id()));
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

/// 任一组件缺失即跳过（和现有 isolation/registry 测试同款门禁）。
fn skip_if_missing() -> bool {
    PLUGINS.iter().any(|d| built_wasm(d).is_none())
}

const SKIP: &str = "skip: 组件未构建，先对各插件 `cargo component build --release`";

#[test]
fn upper_via_agent() {
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("upper");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "把 hello 转大写",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].tool, "upper");
    assert_eq!(r.calls[0].output, "HELLO");
    assert_eq!(r.answer, "HELLO");
}

#[test]
fn idcard_via_agent() {
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("idcard");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "校验身份证 110101199001010015",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].tool, "idcard");
    assert_eq!(r.calls[0].output, "男 · 1990-01-01 · 地区 110101");
    assert_eq!(r.answer, "男 · 1990-01-01 · 地区 110101");
}

#[test]
fn agent_loop_iterates_two_steps() {
    // 「然后」触发第二步：idcard 结果 → reverse，证明回路会迭代而非单次调用。
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("twostep");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "校验身份证 110101199001010023 然后倒序",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 2);
    assert_eq!(r.calls[0].tool, "idcard");
    assert_eq!(r.calls[1].tool, "reverse");
    // reverse 的输出与输入等长、内容不同。
    assert_eq!(r.calls[1].output.len(), r.calls[0].output.len());
    assert_ne!(r.calls[1].output, r.calls[0].output);
    assert_eq!(r.answer, r.calls[1].output);
}

#[test]
fn agent_chains_upper_then_reverse() {
    // 第二步按「然后」后的意图词走（不写死）：upper → reverse → OLLEH。
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("upperthenreverse");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "把 hello 转大写然后倒序",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 2);
    assert_eq!(r.calls[0].tool, "upper");
    assert_eq!(r.calls[0].output, "HELLO");
    assert_eq!(r.calls[1].tool, "reverse");
    assert_eq!(r.calls[1].output, "OLLEH");
    assert_eq!(r.answer, "OLLEH");
}

#[test]
fn attacker_still_cut_off_via_agent() {
    // Agent 路径下沙箱不破：CPU 炸弹（loop）仍被 epoch 切断，宿主存活、回路可响应。
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("attacker");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "让 attacker 跑 loop",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].tool, "attacker");
    assert!(!r.calls[0].ok);
    assert!(r.calls[0].rendered.contains("被沙箱切断"));
    assert!(r.answer.contains("被沙箱切断"));
}

#[test]
fn rmb_via_agent() {
    // 金额大写走 rmb 工具：「人民币」关键词须先于「大写」→ upper 命中。
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("rmb");
    let host = Host::new().unwrap();
    let r = run_agent(
        &host,
        &dir,
        "把 12345.67 转成人民币大写",
        &AlgorithmPredictor,
        MAX_STEPS,
    );
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].tool, "rmb");
    assert_eq!(r.calls[0].output, "壹万贰仟叁佰肆拾伍元陆角柒分");
    assert_eq!(r.answer, "壹万贰仟叁佰肆拾伍元陆角柒分");
}

#[test]
fn tool_result_truncated() {
    // 工具输出截断守卫：超长输出被切到上限并加标记，注入面/上下文炸弹受限。
    if skip_if_missing() {
        eprintln!("{SKIP}");
        return;
    }
    let dir = setup_plugins("truncate");
    let host = Host::new().unwrap();
    let long = "A".repeat(TOOL_RESULT_LIMIT + 100);
    let prompt = format!("把 {long} 转大写");
    let r = run_agent(&host, &dir, &prompt, &AlgorithmPredictor, MAX_STEPS);
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].output.len(), TOOL_RESULT_LIMIT + "…[截断]".len());
    assert!(r.calls[0].output.ends_with("…[截断]"));
    assert!(r.calls[0].rendered.starts_with("工具 upper 返回: "));
}
