use crate::domain::paper::Paper;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthorContext {
    pub authors: Vec<String>,
    pub first_author: Option<String>,
    pub corresponding_authors: Vec<String>,
    pub corresponding_evidence: Vec<String>,
    pub related_papers: Vec<RelatedPaper>,
}
#[derive(Debug, Serialize)]
pub struct RelatedPaper {
    pub id: String,
    pub title: String,
    pub shared_authors: Vec<String>,
    pub relationship: String,
    pub evidence: String,
}

pub fn corresponding_authors(authors: &[String], text: &str) -> (Vec<String>, Vec<String>) {
    let marker = Regex::new(r"(?i)corresponding\s+authors?\s*[:：]?|correspondence\s+(?:to|should be addressed to)\s*[:：]?").unwrap();
    let mut names = Vec::new();
    let mut evidence = Vec::new();
    // Only explicit labels, never infer correspondence from author order or a lone email.
    let lines: Vec<_> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if let Some(found) = marker.find(line) {
            let remainder = &line[found.end()..];
            let wrapped = !authors
                .iter()
                .any(|name| remainder.to_lowercase().contains(&name.to_lowercase()))
                || remainder.trim_end().ends_with("and")
                || remainder.trim_end().ends_with(',');
            let following = if wrapped {
                format!("{} {}", remainder, lines.get(i + 1).copied().unwrap_or(""))
            } else {
                remainder.to_string()
            };
            let following = following.split(')').next().unwrap_or("");
            let lower = following.to_lowercase();
            for name in authors {
                if lower.contains(&name.to_lowercase()) && !names.contains(name) {
                    names.push(name.clone());
                }
            }
            let quote: String = format!("{}{}", &line[found.start()..found.end()], following)
                .chars()
                .take(500)
                .collect();
            if authors
                .iter()
                .any(|name| lower.contains(&name.to_lowercase()))
                && !evidence.contains(&quote)
            {
                evidence.push(quote);
            }
        }
    }
    (names, evidence)
}

pub fn build_context(paper: &Paper, text: &str, library: &[(Paper, String)]) -> AuthorContext {
    let (corresponding, evidence) = corresponding_authors(&paper.authors, text);
    let mut related = Vec::new();
    for (other, other_text) in library {
        if other.id == paper.id {
            continue;
        }
        let shared: Vec<String> = paper
            .authors
            .iter()
            .filter(|author| {
                other
                    .authors
                    .iter()
                    .any(|value| value.trim().eq_ignore_ascii_case(author.trim()))
            })
            .cloned()
            .collect();
        if shared.is_empty() {
            continue;
        }
        let (other_corresponding, _) = corresponding_authors(&other.authors, other_text);
        let common_corresponding = corresponding
            .iter()
            .find(|name| other_corresponding.contains(name));
        let (relationship, reason) = if let Some(name) = common_corresponding {
            (
                "潜在同课题组（待核实）",
                format!("共同通信作者：{name}；不能仅据姓名确认课题组或排除同名。"),
            )
        } else if shared.len() >= 2 {
            (
                "潜在合作团队（待核实）",
                "至少两位署名一致；需结合机构与研究方向核实。".into(),
            )
        } else {
            (
                "同作者候选（待核实）",
                "至少一位署名一致，存在同名可能。".into(),
            )
        };
        related.push(RelatedPaper {
            id: other.id.clone(),
            title: other.title.clone(),
            shared_authors: shared,
            relationship: relationship.into(),
            evidence: reason,
        });
    }
    AuthorContext {
        authors: paper.authors.clone(),
        first_author: paper.authors.first().cloned(),
        corresponding_authors: corresponding,
        corresponding_evidence: evidence,
        related_papers: related,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correspondence_requires_explicit_evidence() {
        let authors = vec!["Ada Lovelace".into(), "Alan Turing".into()];
        assert!(
            corresponding_authors(&authors, "Ada Lovelace, ada@example.edu")
                .0
                .is_empty()
        );
        let (names, evidence) = corresponding_authors(
            &authors,
            "(Corresponding authors: Alan Turing and\nAda Lovelace.)",
        );
        assert_eq!(names.len(), 2);
        assert!(!evidence.is_empty());
        let names = corresponding_authors(&authors, "K101. (Corresponding author: Ada Lovelace.)\nAda Lovelace and Alan Turing are with the School.").0;
        assert_eq!(names, vec!["Ada Lovelace"]);
    }
}
