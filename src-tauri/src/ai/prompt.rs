pub const ANALYSIS_SYSTEM_PROMPT: &str = r#"仅分析所提供的学术论文内容。处理完成后，必须严格按照 API 请求的结构化响应 Schema 返回最终结果，不得在最终结构化响应之外添加评论。字段必须严格为：
schema_version（固定字符串“2”）、research_problem（字符串）、research_scenario（字符串）、core_method（字符串）、key_innovations（字符串数组）、key_contributions（字符串数组）、main_results（字符串）、limitations（字符串）、tags（3–8 个字符串）、ai_summary（字符串）。

必须按照以下五个互不混写的模块归纳：
1. 问题 research_problem：论文要解决的具体问题、知识缺口或研究问题。
2. 场景 research_scenario：研究所处的应用领域、对象、数据集、实验环境或使用条件。
3. 方法 core_method：作者采用的核心技术路线、模型、流程和关键机制。
4. 创新 key_innovations：相对既有工作明确新增或改进之处；不得仅换句话重复方法。
5. 贡献 key_contributions：论文最终形成的理论、算法、系统、数据、实证发现或实际价值；不得仅换句话重复创新。

五个模块必须全部返回。论文未明确限定场景时，research_scenario 写“论文未明确限定具体应用场景。”；论文未提供可验证的创新或贡献时，对应数组分别返回“论文未明确说明相较既有工作的创新点。”或“论文未明确说明可验证的独立贡献。”，不得猜测。main_results 和 limitations 作为补充模块，分别概括有证据的主要结果与局限；未说明时明确写“论文未明确报告”。ai_summary 只写一句总览，不得替代或合并上述五个模块。

所有字段值必须使用简体中文，必要的英文缩写、公式名、模型名和专有名词可以保留。只能使用论文中有依据的信息，不得虚构数值结果、场景、方法、创新或贡献；论文未报告的信息必须明确说明未报告。tags 必须是题目、摘要、核心方法或主要贡献中明确出现的主题，优先使用具体技术术语；英文论文的中文标签应附原文关键词或缩写，例如“集成感知与通信（ISAC）”。如果输入末尾提供受管标签词表，它仅用于统一命名、不是当前论文的证据：语义相同的概念必须复用词表中的完整写法；技术含义不同或仅近似的概念不得强行合并；只有当前论文内容本身支持时才能复用。禁止使用“论文”“研究”“方法”“结果”“分析”等空泛标签。准确描述作者实际提出的内容。不得输出推理过程、思维链、Markdown 或 JSON 之外的任何文本。"#;

pub const COMPARISON_SYSTEM_PROMPT: &str = r#"仅比较所提供的 2–8 篇论文结构化分析。返回一个 JSON 对象，字段必须严格为：common_points（字符串数组）、key_differences（字符串数组）、method_comparison（字符串）、contribution_comparison（字符串）、summary（字符串）。所有字段值必须使用简体中文，必要的英文缩写和专有名词可以保留；差异描述应明确指出对应论文。不得虚构证据，不得输出推理过程、Markdown 或 JSON 之外的任何文本。"#;

pub const REPAIR_ANALYSIS_PROMPT: &str = r#"根据随附论文内容，将下面的响应翻译并修复为简体中文 JSON。字段严格为 schema_version（固定为“2”）、research_problem、research_scenario、core_method、key_innovations、key_contributions、main_results、limitations、tags、ai_summary。确保“问题-场景-方法-创新-贡献”五个模块全部存在、彼此不重复：创新描述相对既有工作的新增或改进，贡献描述最终产出或价值。缺少论文证据时使用系统提示指定的“未明确”表述，不得猜测。tags 必须能由论文原文直接支持；英文术语需在中文标签中附原文关键词或缩写。删除空泛或推测标签，只保留有依据的最终结论和 3–8 个高置信标签，删除推理过程，仅返回 JSON。"#;

pub const REPAIR_COMPARISON_PROMPT: &str = r#"将下面的响应翻译并修复为简体中文 JSON，字段为 common_points、key_differences、method_comparison、contribution_comparison、summary。删除推理过程，仅返回 JSON。"#;

pub fn build_analysis_user_prompt(input: &str) -> String {
    format!("论文内容开始\n{input}\n论文内容结束\n请只返回符合 API 结构化响应 Schema 的简体中文最终结果。")
}

pub fn build_comparison_user_prompt(input: &str) -> String {
    format!("多篇论文结构化分析开始\n{input}\n多篇论文结构化分析结束\n请返回指定的简体中文对比 JSON 对象。")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_forbids_reasoning_and_invention() {
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("简体中文"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("不得虚构"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("不得输出推理过程"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("问题 research_problem"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("场景 research_scenario"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("方法 core_method"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("创新 key_innovations"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("贡献 key_contributions"));
    }
}
