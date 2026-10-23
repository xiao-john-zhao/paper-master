// A deliberately conservative, reviewable vocabulary. Unknown terms retain their wording.
pub const VOCABULARY: &[(&str, &[&str])] = &[
    ("6G", &["6g", "sixth generation"]),
    ("5G", &["5g", "fifth generation"]),
    ("世界模型", &["world model", "world models"]),
    ("具身智能", &["embodied intelligence", "embodied ai"]),
    (
        "生成式人工智能",
        &[
            "generative ai",
            "generative artificial intelligence",
            "genai",
        ],
    ),
    ("扩散模型", &["diffusion model", "diffusion models"]),
    (
        "视觉语言模型",
        &["vision-language model", "vision language model", "vlm"],
    ),
    ("深度学习", &["deep learning"]),
    ("机器学习", &["machine learning"]),
    (
        "任务卸载",
        &["task offloading", "computation offloading", "计算卸载"],
    ),
    ("物联网", &["internet of things", "iot"]),
    (
        "车联网",
        &[
            "internet of vehicles",
            "vehicular network",
            "vehicular networks",
            "iov",
        ],
    ),
    (
        "无线通信",
        &["wireless communication", "wireless communications"],
    ),
    ("信道估计", &["channel estimation"]),
    ("轨迹优化", &["trajectory optimization", "航迹优化"]),
    ("功率控制", &["power control"]),
    ("隐私保护", &["privacy preservation", "privacy-preserving"]),
    (
        "通感一体化",
        &[
            "isac",
            "integrated sensing and communication",
            "integrated sensing and communications",
            "joint communication and sensing",
            "通信感知一体化",
            "感知通信一体化",
            "通感融合",
        ],
    ),
    (
        "无人机",
        &[
            "uav",
            "uavs",
            "unmanned aerial vehicle",
            "unmanned aerial vehicles",
            "无人飞行器",
        ],
    ),
    ("资源分配", &["resource allocation", "资源配置", "资源调配"]),
    ("波束成形", &["beamforming", "波束赋形"]),
    (
        "可重构智能表面",
        &[
            "ris",
            "reconfigurable intelligent surface",
            "reconfigurable intelligent surfaces",
            "智能反射面",
            "irs",
            "intelligent reflecting surface",
        ],
    ),
    ("强化学习", &["reinforcement learning", "rl"]),
    ("深度强化学习", &["deep reinforcement learning", "drl"]),
    (
        "多智能体强化学习",
        &[
            "multi-agent reinforcement learning",
            "marl",
            "多代理强化学习",
        ],
    ),
    (
        "语义通信",
        &["semantic communication", "semantic communications"],
    ),
    (
        "大语言模型",
        &[
            "large language model",
            "large language models",
            "llm",
            "llms",
            "大型语言模型",
        ],
    ),
    (
        "边缘计算",
        &[
            "edge computing",
            "mobile edge computing",
            "mec",
            "移动边缘计算",
        ],
    ),
    ("联邦学习", &["federated learning"]),
    (
        "非正交多址",
        &["noma", "non-orthogonal multiple access", "非正交多址接入"],
    ),
    ("数字孪生", &["digital twin", "digital twins"]),
    ("多输入多输出", &["mimo", "multiple-input multiple-output"]),
    ("凸优化", &["convex optimization"]),
    ("随机几何", &["stochastic geometry"]),
    ("排队论", &["queueing theory", "queuing theory"]),
    ("公平性", &["fairness", "公平性能"]),
    ("能量效率", &["energy efficiency", "能源效率", "能效"]),
    ("频谱效率", &["spectral efficiency", "spectrum efficiency"]),
    ("智能体", &["ai agent", "ai agents", "人工智能代理"]),
];

fn key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn canonical(value: &str) -> String {
    let value = value.trim();
    let normalized = key(value);
    VOCABULARY
        .iter()
        .find(|(name, aliases)| {
            key(name) == normalized || aliases.iter().any(|alias| *alias == normalized)
        })
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| value.to_string())
}
pub fn normalize(values: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .iter()
        .map(|value| canonical(value))
        .filter(|value| !value.is_empty() && seen.insert(key(value)))
        .collect()
}
pub fn from_evidence(text: &str) -> Vec<String> {
    let text = text.to_lowercase();
    VOCABULARY
        .iter()
        .filter(|(name, aliases)| {
            text.contains(name)
                || aliases.iter().any(|term| {
                    // Acronyms must be whole tokens: do not match RIS inside 'comparison'.
                    if term.len() <= 5 && term.is_ascii() {
                        text.split(|c: char| !c.is_ascii_alphanumeric())
                            .any(|word| word == *term)
                    } else {
                        text.contains(term)
                    }
                })
        })
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_are_conservative_and_idempotent() {
        let tags = normalize(&[
            "ISAC".into(),
            "通感融合".into(),
            "beamforming".into(),
            "波束赋形".into(),
            "新方法".into(),
        ]);
        assert_eq!(tags, ["通感一体化", "波束成形", "新方法"]);
        assert_eq!(normalize(&tags), tags);
        assert!(!from_evidence("A comparison is performed.").contains(&"可重构智能表面".into()));
    }
}
