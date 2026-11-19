use crate::api::Question;

pub(crate) fn tex(question: &Question) -> String {
    let mut answers = String::new();
    if let Some(solutions) = &question.solutions {
        for solution in solutions {
            let title = escape_title(&solution.title);
            let label = if solution.is_default {
                "（默认）"
            } else {
                ""
            };
            answers.push_str(&format!(
                "\n\\section*{{{title}{label}}}\n{}\n",
                solution.body
            ));
        }
    } else if !question.answer.trim().is_empty() {
        answers = format!("\n\\section*{{答案}}\n{}\n", question.answer);
    }
    format!(
        r"% !TeX program = xelatex
\documentclass[UTF8,fontset=fandol]{{ctexart}}
\usepackage{{amsmath,amssymb}}
\usepackage{{graphicx}}
\usepackage{{tikz}}
\usepackage[version=4]{{mhchem}}
\begin{{document}}

\section*{{题干}}
{}
{answers}

\end{{document}}
",
        question.stem
    )
}

fn escape_title(title: &str) -> String {
    let mut text = String::new();
    for character in title.chars() {
        match character {
            '\\' => text.push_str(r"\textbackslash{}"),
            '~' => text.push_str(r"\textasciitilde{}"),
            '^' => text.push_str(r"\textasciicircum{}"),
            '\r' | '\n' | '\t' => text.push(' '),
            '{' | '}' | '#' | '$' | '%' | '&' | '_' => {
                text.push('\\');
                text.push(character);
            }
            _ => text.push(character),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{Question, tex};

    #[test]
    fn uses_the_raw_stem_and_answer_in_a_standalone_document() {
        let question: Question = serde_json::from_value(serde_json::json!({
            "id": 42, "category": "数学", "source": "样题",
            "stem": r"求 \(\sqrt{4}\)。", "answer": "$2$", "notes": "", "tags": ""
        }))
        .unwrap();
        let document = tex(&question);
        assert!(document.contains(r"\documentclass[UTF8,fontset=fandol]{ctexart}"));
        assert!(document.contains(&question.stem));
        assert!(document.contains(&question.answer));
        assert!(document.ends_with("\\end{document}\n"));
    }

    #[test]
    fn emits_all_solutions_and_escapes_their_titles() {
        let question: Question = serde_json::from_value(serde_json::json!({
            "id": 42, "category": "数学", "source": "样题",
            "stem": "题干", "answer": "$2$", "notes": "", "tags": "",
            "solutions": [
                {"id": 1, "title": "解法_1", "body": "$2$", "is_default": true},
                {"id": 2, "title": "解法#2", "body": "$1+1$", "is_default": false}
            ]
        }))
        .unwrap();
        let document = tex(&question);
        assert!(document.contains(r"\section*{解法\_1（默认）}"));
        assert!(document.contains(r"\section*{解法\#2}"));
        assert!(document.contains("$1+1$"));
        assert_eq!(document.matches("$2$").count(), 1);
    }
}
