mod rating;
mod repository;
mod solutions;

pub(crate) use rating::{QuestionScore, RankOrder, RankedQuestion, Vote, ranking, rate};
pub(crate) use repository::{
    categories, find, insert, paper_questions, papers, render_selection, search, soft_delete, tags,
    update,
};
pub(crate) use solutions::{
    NewSolution, SolutionPatch, add_solution, delete_solution, update_solution,
};

use serde::{Deserialize, Serialize};
use sqlx::types::Json;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewQuestion {
    pub category: String,
    pub source: String,
    pub original_number: Option<i32>,
    pub stem: String,
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct QuestionPatch {
    #[serde(deserialize_with = "present_string")]
    pub category: Option<String>,
    #[serde(deserialize_with = "present_string")]
    pub source: Option<String>,
    #[serde(deserialize_with = "present_number")]
    pub original_number: Option<Option<i32>>,
    #[serde(deserialize_with = "present_string")]
    pub stem: Option<String>,
    #[serde(deserialize_with = "present_string")]
    pub answer: Option<String>,
    #[serde(deserialize_with = "present_string")]
    pub notes: Option<String>,
    #[serde(deserialize_with = "present_string")]
    pub tags: Option<String>,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub(crate) struct Question {
    pub id: i32,
    pub category: String,
    pub source: String,
    pub original_number: Option<i32>,
    pub paper_source: String,
    pub stem: String,
    pub answer: String,
    pub notes: String,
    pub tags: String,
    pub score: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct QuestionDetail {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub question: Question,
    pub solutions: Json<Vec<Solution>>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Solution {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub is_default: bool,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub(crate) struct SelectedQuestion {
    pub id: i32,
    pub source: String,
    pub original_number: Option<i32>,
    pub stem: String,
    pub answer: String,
}

impl QuestionPatch {
    pub(crate) fn validated(self) -> Result<Self, &'static str> {
        if self.category.is_none()
            && self.source.is_none()
            && self.original_number.is_none()
            && self.stem.is_none()
            && self.answer.is_none()
            && self.notes.is_none()
            && self.tags.is_none()
        {
            return Err("empty_patch");
        }
        if let Some(value) = self.original_number {
            validate_original_number(value)?;
        }
        Ok(Self {
            category: checked(
                self.category,
                |value| validate(value, 120, false),
                "invalid_category",
            )?,
            source: checked(
                self.source,
                |value| validate(value, 300, false),
                "invalid_source",
            )?,
            original_number: self.original_number,
            stem: checked(self.stem, |value| latex_source(value, 6000), "invalid_stem")?,
            answer: checked(self.answer, optional_answer, "invalid_answer")?,
            notes: checked(
                self.notes,
                |value| validate(value, 2000, true),
                "invalid_notes",
            )?,
            tags: checked(self.tags, normalize_tags, "invalid_tags")?,
        })
    }
}

fn present_string<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<String>, D::Error> {
    String::deserialize(input).map(Some)
}

fn present_number<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<Option<Option<i32>>, D::Error> {
    Option::<i32>::deserialize(input).map(Some)
}

fn validate_original_number(number: Option<i32>) -> Result<Option<i32>, &'static str> {
    if number.is_some_and(|number| number <= 0) {
        return Err("invalid_original_number");
    }
    Ok(number)
}

fn checked(
    field: Option<String>,
    validate_field: impl FnOnce(String) -> Option<String>,
    error: &'static str,
) -> Result<Option<String>, &'static str> {
    field
        .map(|value| validate_field(value).ok_or(error))
        .transpose()
}
#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct Category {
    pub id: i32,
    pub name: String,
    pub question_count: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct Tag {
    pub name: String,
    pub question_count: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct Paper {
    pub source: String,
    pub question_count: i64,
}

impl NewQuestion {
    pub(crate) fn validated(self) -> Result<Self, &'static str> {
        Ok(Self {
            category: validate(self.category, 120, false).ok_or("invalid_category")?,
            source: validate(self.source, 300, false).ok_or("invalid_source")?,
            original_number: validate_original_number(self.original_number)?,
            stem: latex_source(self.stem, 6000).ok_or("invalid_stem")?,
            answer: optional_answer(self.answer).ok_or("invalid_answer")?,
            notes: validate(self.notes, 2000, true).ok_or("invalid_notes")?,
            tags: normalize_tags(self.tags).ok_or("invalid_tags")?,
        })
    }
}

fn optional_answer(value: String) -> Option<String> {
    if value.chars().count() > 8000 || value.contains('\0') {
        return None;
    }
    Some(if value.trim().is_empty() {
        String::new()
    } else {
        value
    })
}

fn latex_source(value: String, max_chars: usize) -> Option<String> {
    (!value.trim().is_empty() && !value.contains('\0') && value.chars().count() <= max_chars)
        .then_some(value)
}

fn normalize_tags(value: String) -> Option<String> {
    let value = validate(value, 500, true)?;
    let mut tags = Vec::new();
    for tag in value.split([',', '，']).map(str::trim) {
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    Some(tags.join(","))
}

fn validate(value: String, max_chars: usize, allow_empty: bool) -> Option<String> {
    let trimmed = value.trim();
    ((!trimmed.is_empty() || allow_empty)
        && !trimmed.contains('\0')
        && trimmed.chars().count() <= max_chars)
        .then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{NewQuestion, QuestionPatch, normalize_tags};

    #[test]
    fn answers_can_be_omitted_or_cleared() {
        let value: NewQuestion =
            deserialize(r#"{"category":"化学","source":"原卷","stem":"题干"}"#);
        assert_eq!(value.validated().unwrap().answer, "");
        let patch: QuestionPatch = deserialize(r#"{"answer":" "}"#);
        assert_eq!(patch.validated().unwrap().answer.as_deref(), Some(""));
    }

    #[test]
    fn original_number_patch_distinguishes_omission_and_clear() {
        let omitted: QuestionPatch = deserialize(r#"{"source":"试卷"}"#);
        assert_eq!(omitted.validated().unwrap().original_number, None);
        let cleared: QuestionPatch = deserialize(r#"{"original_number":null}"#);
        assert_eq!(cleared.validated().unwrap().original_number, Some(None));
        let numbered: QuestionPatch = deserialize(r#"{"original_number":10}"#);
        assert_eq!(
            numbered.validated().unwrap().original_number,
            Some(Some(10))
        );
        let invalid: QuestionPatch = deserialize(r#"{"original_number":0}"#);
        assert_eq!(invalid.validated().unwrap_err(), "invalid_original_number");
    }

    fn deserialize<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn normalizes_separators_whitespace_and_duplicate_tags() {
        assert_eq!(
            normalize_tags(" 代数，计算, 代数 ,, 三角函数 ".into()).as_deref(),
            Some("代数,计算,三角函数")
        );
    }
}
