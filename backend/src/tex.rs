use std::{
    collections::BTreeSet,
    io::{self, Cursor, Write},
    sync::LazyLock,
};

use regex::Regex;
use zip::{ZipWriter, write::SimpleFileOptions};

static GRAPHICS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\begin\{verbatim\}[\s\S]*?(?:\\end\{verbatim\}|$)|\\begin\{verbatim\*\}[\s\S]*?(?:\\end\{verbatim\*\}|$)|\\verb\*?(?P<verbatim>[^a-zA-Z\s])|\\\\|\\%|%[^\r\n]*|\\includegraphics\*?\s*(?:\[[^\[\]]*\]\s*)?\{(?P<path>[^{}\r\n]+)\}").unwrap()
});

pub(crate) struct AnswerSection<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub is_default: bool,
}

pub(crate) fn image_references<'a>(
    stem: &'a str,
    solutions: &[AnswerSection<'a>],
) -> BTreeSet<&'a str> {
    let mut paths = BTreeSet::new();
    collect_images(stem, &mut paths);
    for solution in solutions {
        collect_images(solution.body, &mut paths);
    }
    paths
}

pub(crate) struct Archive(ZipWriter<Cursor<Vec<u8>>>);

impl Archive {
    pub(crate) fn new(id: i32, stem: &str, solutions: &[AnswerSection<'_>]) -> io::Result<Self> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive.start_file(format!("question-{id}.tex"), SimpleFileOptions::default())?;
        archive.write_all(document(stem, solutions).as_bytes())?;
        Ok(Self(archive))
    }

    pub(crate) fn add_image(&mut self, name: &str, data: &[u8]) -> io::Result<()> {
        self.0
            .start_file(format!("images/{name}"), SimpleFileOptions::default())?;
        self.0.write_all(data)
    }

    pub(crate) fn finish(self) -> io::Result<Vec<u8>> {
        Ok(self.0.finish()?.into_inner())
    }
}

pub(crate) fn collect_images<'a>(source: &'a str, paths: &mut BTreeSet<&'a str>) {
    let mut offset = 0;
    while let Some(found) = GRAPHICS.captures_at(source, offset) {
        offset = found.get(0).unwrap().end();
        if let Some(delimiter) = found.name("verbatim") {
            let rest = &source[offset..];
            let line_end = rest.find(['\r', '\n']).unwrap_or(rest.len());
            offset += rest[..line_end]
                .find(delimiter.as_str())
                .map_or(line_end, |index| index + delimiter.len());
        }
        if let Some(path) = found.name("path") {
            paths.insert(path.as_str().trim());
        }
    }
}

fn document(stem: &str, solutions: &[AnswerSection<'_>]) -> String {
    let mut answers = String::new();
    for solution in solutions {
        let title = escape_title(solution.title);
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
    format!(
        r"% !TeX program = xelatex
\documentclass[UTF8,fontset=fandol]{{ctexart}}
\usepackage{{amsmath,amssymb}}
\usepackage{{graphicx}}
\usepackage{{tikz}}
\usepackage[version=4]{{mhchem}}
\begin{{document}}

\section*{{题干}}
{stem}
{answers}

\end{{document}}
"
    )
}

pub(crate) fn escape_title(title: &str) -> String {
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
    use std::io::Read;

    use super::*;

    #[test]
    fn packages_original_source_and_referenced_file_once() {
        let path = "images/first.png";
        let second_path = "images/second.png";
        let stem = format!(r"题干 \includegraphics[width=.5\linewidth]{{{path}}}");
        let answer = format!(
            "$2$\n% \\includegraphics{{ignored.png}}\n\\verb|\\includegraphics{{ignored.png}}|\n\\includegraphics*{{{path}}}"
        );
        let another = format!(r"另一解法 \includegraphics{{{second_path}}}");
        let solutions = [
            AnswerSection {
                title: "默认解法",
                body: &answer,
                is_default: true,
            },
            AnswerSection {
                title: "方法_2",
                body: &another,
                is_default: false,
            },
        ];
        let references = image_references(&stem, &solutions);
        assert_eq!(references, BTreeSet::from([path, second_path]));
        let mut output = Archive::new(42, &stem, &solutions).unwrap();
        output.add_image("first.png", b"fixture-1").unwrap();
        output.add_image("second.png", b"fixture-2").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(output.finish().unwrap())).unwrap();
        assert_eq!(archive.len(), 3);
        let mut output = String::new();
        archive
            .by_name("question-42.tex")
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();
        assert!(output.contains(r"\usepackage{graphicx}"));
        assert!(output.contains(&stem) && output.contains(&answer));
        assert!(output.contains(r"\section*{方法\_2}"));
        assert!(output.contains("另一解法"));
        let mut data = Vec::new();
        archive
            .by_name(path)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"fixture-1");
        let mut data = Vec::new();
        archive
            .by_name(second_path)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"fixture-2");
    }
}
