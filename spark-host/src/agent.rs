//! SPARK Agent 回路：把 `plugins/` 下满足 `plugin-world` 的组件当作 LLM 可见的「工具」，
//! 由 [`Predictor`] 决定下一步（现在 = 本地算法预测；将来 = DeepSeek harness 实现同一 trait）。
//! 工具调用走宿主沙箱（epoch + StoreLimits），工具输出按**不可信数据**处理：截断 + 包装，
//! 防 prompt 注入与上下文炸弹（见 SECURITY.md）。回路有迭代上限，失控 Predictor 不会无限循环。

use std::collections::HashMap;

use serde_json::json;

use crate::exports::spark::runtime::plugin::PluginInfo;
use crate::Host;

/// 单次 Agent 决策：直接给最终答复，或调用某插件暴露的某个工具。
#[derive(Debug, Clone)]
pub enum Decision {
    /// 不调工具，结束回路，给出最终答复。
    Final(String),
    /// 调用工具：`tool` = 工具名（LLM 视角），`args` = JSON 对象字符串。
    Call { tool: String, args: String },
}

/// Predictor = Agent 回路的「决策者」。现在用本地算法模拟 LLM 会怎么选工具，
/// 将来接 DeepSeek harness（flash/pro）时实现同样的 trait，宿主回路零改动。
pub trait Predictor {
    /// 根据用户 prompt 与已执行工具结果，决定下一步。
    fn decide(&self, prompt: &str, history: &[ToolResult], tools: &[ToolHandle]) -> Decision;
}

/// 宿主交给 Predictor 的每个工具的视图（来自插件 `schema()`）。
#[derive(Debug, Clone)]
pub struct ToolHandle {
    pub name: String,
    pub plugin: String,
    pub description: String,
    pub parameters: Vec<(String, String, String)>, // (name, type, description)
}

/// 一次已执行的工具调用的结果（输出已截断）。
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub plugin: String,
    pub tool: String,
    /// 本轮的调用 id（harness 重建 OpenAI 协议消息用）。
    pub call_id: String,
    /// 发送给插件的原始 args JSON 字符串（harness 重建 assistant tool_calls 用）。
    pub args: String,
    /// 截断后的原始工具输出（成功时）；失败 / 沙箱切断时为空。
    pub output: String,
    /// 人类可读结果行（含成功/失败/切断）。
    pub rendered: String,
    pub ok: bool,
}

/// Agent 回路的最终结果。
#[derive(Debug)]
pub struct AgentResult {
    /// 最终答复。
    pub answer: String,
    /// 回路里执行过的所有工具调用。
    pub calls: Vec<ToolResult>,
}

/// 工具结果截断上限：防输出炸弹 + 缩小注入面。
pub const TOOL_RESULT_LIMIT: usize = 4096;

/// 回路最大步数。
pub const MAX_STEPS: usize = 8;

/// 跑一遍 Agent 回路：Predictor 决策 → 沙箱调用 → 结果喂回 → 直到 Final 或步数上限。
pub fn run_agent(
    host: &Host,
    dir: &str,
    prompt: &str,
    predictor: &dyn Predictor,
    max_steps: usize,
) -> AgentResult {
    // 工具注册表：工具名 → (插件文件名, 插件信息)。同名工具后注册者生效。
    let mut registry: HashMap<String, (String, PluginInfo)> = HashMap::new();
    let mut tools: Vec<ToolHandle> = Vec::new();
    for (file, info, schemas) in host.schemas(dir) {
        for s in schemas {
            if registry
                .insert(s.name.clone(), (file.clone(), info.clone()))
                .is_some()
            {
                eprintln!("警告：工具名 {} 重复，后注册者生效", s.name);
            }
            tools.push(ToolHandle {
                name: s.name.clone(),
                plugin: info.name.clone(),
                description: s.description.clone(),
                parameters: s
                    .parameters
                    .iter()
                    .map(|p| {
                        (
                            p.name.clone(),
                            p.parameter_type.clone(),
                            p.description.clone(),
                        )
                    })
                    .collect(),
            });
        }
    }
    if tools.is_empty() {
        return AgentResult {
            answer: "未发现任何工具：plugins/ 下没有满足契约的组件".into(),
            calls: Vec::new(),
        };
    }

    let mut history: Vec<ToolResult> = Vec::new();
    for _step in 0..max_steps {
        match predictor.decide(prompt, &history, &tools) {
            Decision::Final(text) => {
                return AgentResult {
                    answer: text,
                    calls: history,
                }
            }
            Decision::Call { tool, args } => {
                let call_id = format!("call_{}", history.len() + 1);
                let args = args.clone();
                let Some((file, info)) = registry.get(&tool).cloned() else {
                    history.push(ToolResult {
                        plugin: String::new(),
                        tool: tool.clone(),
                        call_id: call_id.clone(),
                        args: args.clone(),
                        output: String::new(),
                        rendered: format!("工具 {tool} 未注册"),
                        ok: false,
                    });
                    continue;
                };
                let path = format!("{dir}/{file}");
                let result = match host.invoke(&path, &tool, &args) {
                    Ok((_, Ok(out))) => {
                        let output = truncate(&out, TOOL_RESULT_LIMIT);
                        ToolResult {
                            plugin: info.name.clone(),
                            tool: tool.clone(),
                            call_id: call_id.clone(),
                            args: args.clone(),
                            rendered: format!("工具 {tool} 返回: {output}"),
                            output,
                            ok: true,
                        }
                    }
                    Ok((_, Err(error))) => ToolResult {
                        plugin: info.name.clone(),
                        tool: tool.clone(),
                        call_id: call_id.clone(),
                        args: args.clone(),
                        output: String::new(),
                        rendered: format!(
                            "工具 {tool} 失败 [{code}]: {message}",
                            code = error.code,
                            message = error.message
                        ),
                        ok: false,
                    },
                    Err(e) => ToolResult {
                        plugin: info.name.clone(),
                        tool: tool.clone(),
                        call_id: call_id.clone(),
                        args: args.clone(),
                        output: String::new(),
                        rendered: format!("工具 {tool} 被沙箱切断: {e:#}"),
                        ok: false,
                    },
                };
                history.push(result);
            }
        }
    }
    AgentResult {
        answer: format!("达到迭代上限（{max_steps} 步）"),
        calls: history,
    }
}

/// 截断到字符边界，超限加标记。
fn truncate(s: &str, limit: usize) -> String {
    let mut t = s.to_string();
    if t.len() > limit {
        let mut end = limit;
        while !t.is_char_boundary(end) {
            end -= 1;
        }
        t.truncate(end);
        t.push_str("…[截断]");
    }
    t
}

/// 本地算法预测：关键词 → 工具选择 + 参数提取。
/// 当前只是「预测」DeepSeek 会怎么选工具；真正干活时由 harness 实现 [`Predictor`]。
pub struct AlgorithmPredictor;

impl Predictor for AlgorithmPredictor {
    fn decide(&self, prompt: &str, history: &[ToolResult], _tools: &[ToolHandle]) -> Decision {
        // 两步规则：读「然后/再」之后的意图词决定第二步工具（不写死），输入 = 上一步输出。
        // `history.len() == 1` 保证只链一次，之后直接 Final——证明回路会迭代，又不失控。
        if let Some(last) = history.last() {
            if last.ok && history.len() == 1 {
                if let Some(tool) = second_step_intent(prompt) {
                    return Decision::Call {
                        tool: tool.into(),
                        args: json!({"text": last.output}).to_string(),
                    };
                }
            }
            let answer = if last.output.is_empty() {
                last.rendered.clone()
            } else {
                last.output.clone()
            };
            return Decision::Final(answer);
        }

        if prompt.contains("身份证") {
            return Decision::Call {
                tool: "idcard".into(),
                args: json!({"id": token_after(prompt, "身份证")}).to_string(),
            };
        }
        if prompt.contains("银行卡") || prompt.contains("卡号") {
            let mut n = token_after(prompt, "银行卡号");
            if n.is_empty() {
                n = token_after(prompt, "银行卡");
            }
            if n.is_empty() {
                n = token_after(prompt, "卡号");
            }
            return Decision::Call {
                tool: "luhn".into(),
                args: json!({"card_number": n}).to_string(),
            };
        }
        // 人民币/金额 → rmb，须在「大写」→ upper 之前：金额大写里也含「大写」二字。
        if prompt.contains("人民币") || prompt.contains("金额") {
            let mut n = between(prompt, "把", "转成");
            if n.is_empty() {
                n = token_after(prompt, "人民币");
            }
            if n.is_empty() {
                n = token_after(prompt, "金额");
            }
            return Decision::Call {
                tool: "rmb".into(),
                args: json!({"amount": n}).to_string(),
            };
        }
        if prompt.contains("大写") {
            return Decision::Call {
                tool: "upper".into(),
                args: json!({"text": between(prompt, "把", "转大写")}).to_string(),
            };
        }
        if prompt.contains("倒序") {
            return Decision::Call {
                tool: "reverse".into(),
                args: json!({"text": between(prompt, "把", "倒序")}).to_string(),
            };
        }
        if prompt.contains("attacker") {
            let action = token_after(prompt, "跑");
            let action = if action.is_empty() {
                "loop".to_string()
            } else {
                action
            };
            return Decision::Call {
                tool: "attacker".into(),
                args: json!({"action": action}).to_string(),
            };
        }

        Decision::Final(format!("（本地预测：未匹配到工具，直接回答）{prompt}"))
    }
}

/// 取 `marker` 之后的第一个空白分隔 token。
fn token_after(s: &str, marker: &str) -> String {
    s.split_once(marker)
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .unwrap_or_default()
        .to_string()
}

/// 读「然后/再」之后第二步的意图词；未识别返回 None（不再链）。
fn second_step_intent(prompt: &str) -> Option<&'static str> {
    let rest = prompt
        .split_once("然后")
        .or_else(|| prompt.split_once("再"))
        .map(|(_, r)| r)?;
    if rest.contains("倒序") {
        Some("reverse")
    } else if rest.contains("大写") {
        Some("upper")
    } else {
        None
    }
}

/// 取 `open` 与 `close` 之间的子串（用于「把 X 转大写」这类表达）。
fn between(s: &str, open: &str, close: &str) -> String {
    s.split_once(open)
        .and_then(|(_, rest)| rest.split_once(close))
        .map(|(mid, _)| mid.trim().to_string())
        .unwrap_or_default()
}
