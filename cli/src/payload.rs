use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::{CliError, Result};

const MAX_BYTES: u64 = 256 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewQuestion {
    category: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_number: Option<i32>,
    stem: String,
    #[serde(default)]
    answer: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    tags: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewSolution {
    title: String,
    body: String,
    #[serde(default)]
    make_default: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct SolutionPatch {
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    title: Option<String>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    body: Option<String>,
    make_default: bool,
}

impl SolutionPatch {
    pub(crate) fn is_empty(&self) -> bool {
        self.title.is_none() && self.body.is_none() && !self.make_default
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct QuestionPatch {
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    category: Option<String>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    source: Option<String>,
    #[serde(
        deserialize_with = "present_number",
        skip_serializing_if = "Option::is_none"
    )]
    original_number: Option<Option<i32>>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    stem: Option<String>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    answer: Option<String>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    notes: Option<String>,
    #[serde(
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    tags: Option<String>,
}

impl QuestionPatch {
    pub(crate) fn is_empty(&self) -> bool {
        self.category.is_none()
            && self.source.is_none()
            && self.original_number.is_none()
            && self.stem.is_none()
            && self.answer.is_none()
            && self.notes.is_none()
            && self.tags.is_none()
    }
}

fn present_string<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> std::result::Result<Option<String>, D::Error> {
    String::deserialize(input).map(Some)
}

fn present_number<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> std::result::Result<Option<Option<i32>>, D::Error> {
    Option::<i32>::deserialize(input).map(Some)
}

pub(crate) fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    if path == Path::new("-") {
        return parse(io::stdin().lock());
    }
    let file = File::open(path).map_err(|_| CliError::io("Could not open the JSON input file."))?;
    parse(file)
}

fn parse<T: DeserializeOwned>(input: impl Read) -> Result<T> {
    let mut bytes = Vec::new();
    input
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| CliError::io("Could not read JSON input."))?;
    if bytes.len() > MAX_BYTES as usize {
        return Err(CliError::usage("JSON input exceeds 256 KiB."));
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|error| CliError::usage(format!(
        "Invalid UTF-8 JSON or invalid fields at line {}, column {}. See the command documentation.",
        error.line(), error.column()
    )))
}

#[cfg(test)]
mod tests {
    use super::{NewQuestion, QuestionPatch, parse};

    #[test]
    fn update_distinguishes_omitted_empty_and_null() {
        let patch = parse::<QuestionPatch>(&br#"{"notes":""}"#[..]).unwrap();
        assert!(!patch.is_empty());
        assert_eq!(serde_json::to_string(&patch).unwrap(), r#"{"notes":""}"#);
        assert!(parse::<QuestionPatch>(&br#"{"notes":null}"#[..]).is_err());
        assert!(parse::<QuestionPatch>(&br#"{}"#[..]).unwrap().is_empty());
        let clear_number = parse::<QuestionPatch>(&br#"{"original_number":null}"#[..]).unwrap();
        assert!(!clear_number.is_empty());
        assert_eq!(
            serde_json::to_string(&clear_number).unwrap(),
            r#"{"original_number":null}"#
        );
    }

    #[test]
    fn add_reads_bom_and_preserves_latex() {
        let raw =
            br#"{"category":"Algebra","source":"Worksheet","stem":"\\(x=1\\)","answer":"$1$"}"#;
        let value =
            parse::<NewQuestion>([b"\xef\xbb\xbf", raw.as_slice()].concat().as_slice()).unwrap();
        assert_eq!(value.stem, r"\(x=1\)");
        assert!(value.notes.is_empty());
    }
}
