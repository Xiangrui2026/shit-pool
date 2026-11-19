use std::{collections::BTreeSet, fmt::Write};

use serde::Deserialize;

use super::{Artifact, Snapshot, Template};
use crate::{images::mime_type, tex};

const PAPER_CORE: &str = include_str!("../../render/ccho/paper-core.sty");
const CCHO_PROFILE: &str = include_str!("../../render/ccho/profile.json");

#[derive(Deserialize)]
struct Profile {
    paper_size: String,
    section_format: String,
    body_pt: u8,
    title_pt: u8,
    line_spread: f64,
    margins_mm: Margins,
    colors: Colors,
}

#[derive(Deserialize)]
struct Margins {
    left: u8,
    right: u8,
    top: u8,
    bottom: u8,
    footskip: u8,
}

#[derive(Deserialize)]
struct Colors {
    answer: [u8; 3],
    prompt: [u8; 3],
}

fn profile_source() -> Result<String, &'static str> {
    let profile: Profile =
        serde_json::from_str(CCHO_PROFILE).map_err(|_| "render_template_error")?;
    let paper = match profile.paper_size.as_str() {
        "A4" => "a4paper",
        _ => return Err("render_template_error"),
    };
    let section = match profile.section_format.as_str() {
        "question" => "题",
        "part" => "部分",
        _ => return Err("render_template_error"),
    };
    let rgb = |[red, green, blue]: [u8; 3]| format!("{red},{green},{blue}");
    let mut source = String::new();
    for (key, value) in [
        ("PaperBodyPt", profile.body_pt.to_string()),
        ("PaperTitlePt", profile.title_pt.to_string()),
        ("PaperLineSpread", profile.line_spread.to_string()),
        ("PaperSectionWord", section.into()),
        ("PaperAnswerRGB", rgb(profile.colors.answer)),
        ("PaperPromptRGB", rgb(profile.colors.prompt)),
        ("PaperPaperSize", paper.into()),
        ("PaperFontScheme", "newtx".into()),
        ("PaperLeft", profile.margins_mm.left.to_string()),
        ("PaperRight", profile.margins_mm.right.to_string()),
        ("PaperTop", profile.margins_mm.top.to_string()),
        ("PaperBottom", profile.margins_mm.bottom.to_string()),
        ("PaperFootskip", profile.margins_mm.footskip.to_string()),
    ] {
        writeln!(source, "\\def\\{key}{{{value}}}").unwrap();
    }
    Ok(source)
}

pub(super) struct PaperInput {
    pub sources: Vec<(String, Vec<u8>)>,
    pub images: BTreeSet<String>,
}

pub(super) fn prepare(snapshot: &Snapshot, artifact: Artifact) -> Result<PaperInput, &'static str> {
    match snapshot.template {
        Template::Ccho => ccho(snapshot, artifact),
    }
}

fn ccho(snapshot: &Snapshot, artifact: Artifact) -> Result<PaperInput, &'static str> {
    let mut paper = format!(
        "\\documentclass[11pt,a4paper,fontset=fandol]{{ctexart}}\n\
         \\input{{profile.tex}}\n\\usepackage{{paper-core}}\n\
         \\usepackage{{tikz}}\n\\usepackage[version=4]{{mhchem}}\n\
         \\begin{{document}}\n\\PaperTitle{{{}}}{{}}\n\\PaperAnswerHeading\n",
        tex::escape_title(&snapshot.title),
    );
    let answers = matches!(artifact, Artifact::Answers);
    let mut sources = Vec::new();
    let mut references = BTreeSet::new();
    for (index, question) in snapshot.questions.iter().enumerate() {
        let number = index + 1;
        tex::collect_images(&question.stem, &mut references);
        writeln!(
            paper,
            "\\begingroup\n\\PaperSection{{{number}}}{{}}{{}}\n{}\n\\par\n\\endgroup\n\
             \\PaperIfAnswers{{\\par\\begingroup\\color{{paperanswer}}\\textbf{{答案：}}\
             \\InputIfFileExists{{answer-{number}.tex}}{{}}{{未录入答案}}\\par\\endgroup}}",
            question.stem,
        )
        .unwrap();
        if answers && !question.answer.trim().is_empty() {
            tex::collect_images(&question.answer, &mut references);
            sources.push((
                format!("answer-{number}.tex"),
                question.answer.as_bytes().to_vec(),
            ));
        }
    }
    paper.push_str("\\end{document}\n");
    let mut images = BTreeSet::new();
    for reference in references {
        let name = reference
            .strip_prefix("images/")
            .filter(|name| mime_type(name).is_some())
            .ok_or("unsupported_image_reference")?;
        images.insert(name.to_owned());
    }
    if images.len() > 32 {
        return Err("render_images_too_large");
    }
    let (driver_name, mode) = match artifact {
        Artifact::Questions => ("questions.tex", "questions"),
        Artifact::Answers => ("answers.tex", "answers"),
    };
    let driver = format!("\\def\\PaperMode{{{mode}}}\n\\input{{paper.tex}}\n");
    sources.extend([
        ("paper-core.sty".into(), PAPER_CORE.as_bytes().to_vec()),
        ("profile.tex".into(), profile_source()?.into_bytes()),
        ("paper.tex".into(), paper.into_bytes()),
        (driver_name.into(), driver.into_bytes()),
    ]);
    Ok(PaperInput { sources, images })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use super::*;
    use crate::questions::SelectedQuestion;

    fn sample() -> Snapshot {
        Snapshot {
            title: "石库样卷 50%".into(),
            template: Template::Ccho,
            questions: vec![
                SelectedQuestion {
                    id: 9,
                    source: "原卷".into(),
                    original_number: Some(5),
                    stem: "求 $1+1$。".into(),
                    answer: "答案标记 $2$。".into(),
                },
                SelectedQuestion {
                    id: 2,
                    source: "原卷".into(),
                    original_number: None,
                    stem: "求 $2+2$。".into(),
                    answer: String::new(),
                },
            ],
        }
    }

    #[test]
    fn preserves_order_and_omits_answers_from_question_input() {
        let snapshot = sample();
        let mut main_source = None;
        for artifact in [Artifact::Questions, Artifact::Answers] {
            let output = prepare(&snapshot, artifact).unwrap();
            let main = output
                .sources
                .iter()
                .find(|(name, _)| *name == "paper.tex")
                .unwrap();
            let source = std::str::from_utf8(&main.1).unwrap();
            assert!(source.contains(r"石库样卷 50\%"));
            assert!(
                source.find(r"\PaperSection{1}").unwrap()
                    < source.find(r"\PaperSection{2}").unwrap()
            );
            if let Some(previous) = main_source.replace(source.to_owned()) {
                assert_eq!(source, previous);
            }
            let answers = output
                .sources
                .iter()
                .find(|(name, _)| name == "answer-1.tex");
            assert_eq!(answers.is_some(), matches!(artifact, Artifact::Answers));
            if let Some((_, content)) = answers {
                let content = std::str::from_utf8(content).unwrap();
                assert!(content.contains("答案标记"));
            }
            assert!(
                !output
                    .sources
                    .iter()
                    .any(|(name, _)| name == "answer-2.tex")
            );
        }
    }

    #[test]
    fn rejects_unsupported_graphics_references() {
        let mut snapshot = sample();
        snapshot.questions[0].answer = r"\includegraphics{external.png}".into();
        assert!(prepare(&snapshot, Artifact::Questions).is_ok());
        assert!(matches!(
            prepare(&snapshot, Artifact::Answers),
            Err("unsupported_image_reference")
        ));
    }

    #[test]
    #[ignore = "requires XeLaTeX and Poppler; only compiles a fixed synthetic fixture"]
    fn compiles_and_checks_both_ccho_pdfs() {
        let mut snapshot = sample();
        let png = crate::images::test_png();
        let image_directory = tempfile::tempdir().unwrap();
        let images = crate::images::ImageStore::new(crate::storage::LocalFileStore::new(
            image_directory.path().into(),
        ));
        let image = images.save(&png).unwrap();
        snapshot.questions[0].stem.push_str(&format!(
            "\n\n\\includegraphics[width=2cm]{{images/{image}}}",
        ));
        let output =
            tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        for artifact in [Artifact::Questions, Artifact::Answers] {
            let folder = output.path().join(artifact.name());
            fs::create_dir_all(folder.join("images")).unwrap();
            fs::write(folder.join("images").join(&image), &png).unwrap();
            for (name, bytes) in prepare(&snapshot, artifact).unwrap().sources {
                fs::write(folder.join(name), bytes).unwrap();
            }
            for _ in 0..2 {
                let result = Command::new("xelatex")
                    .current_dir(&folder)
                    .args([
                        "-no-shell-escape",
                        "-interaction=nonstopmode",
                        "-halt-on-error",
                    ])
                    .arg(format!("{}.tex", artifact.name()))
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stdout)
                );
            }
            let log = fs::read_to_string(folder.join(format!("{}.log", artifact.name()))).unwrap();
            assert!(!log.contains("Missing character:") && !log.contains("Overfull \\"));
            let result = Command::new("pdftotext")
                .current_dir(&folder)
                .args(["-enc", "UTF-8", &format!("{}.pdf", artifact.name()), "-"])
                .output()
                .unwrap();
            assert!(result.status.success());
            let text = String::from_utf8(result.stdout).unwrap();
            let answers = matches!(artifact, Artifact::Answers);
            assert_eq!(text.contains("参考答案"), answers, "{text}");
            assert_eq!(text.contains("答案标记"), answers, "{text}");
            assert_eq!(text.contains("未录入答案"), answers, "{text}");
            let compact = text.split_whitespace().collect::<String>();
            assert!(
                matches!((compact.find("第1题"), compact.find("第2题")), (Some(first), Some(second)) if first < second),
                "{text}"
            );
        }
        if std::env::var_os("SHIT_POOL_KEEP_RENDER_FIXTURE").is_some() {
            println!("CChO fixture: {}", output.keep().display());
        }
    }
}
