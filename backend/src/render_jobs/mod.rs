mod repository;
mod template;
mod worker;

use std::{collections::BTreeSet, io, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    questions::SelectedQuestion,
    storage::{FileKey, FileStore},
};

pub(crate) use repository::{JobStatus, enqueue, status};
pub(crate) use worker::run;

pub(crate) const MAX_QUESTIONS: usize = 40;
const MAX_PDF_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Template {
    #[default]
    Ccho,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Submission {
    pub title: String,
    pub question_ids: Vec<i32>,
    #[serde(default)]
    pub template: Template,
}

impl Submission {
    pub(crate) fn validated(mut self) -> Result<Self, &'static str> {
        let title = self.title.trim();
        if title.is_empty() || title.chars().count() > 120 || title.chars().any(char::is_control) {
            return Err("invalid_paper_title");
        }
        let unique: BTreeSet<_> = self.question_ids.iter().collect();
        if self.question_ids.is_empty()
            || self.question_ids.len() > MAX_QUESTIONS
            || self.question_ids.iter().any(|id| *id <= 0)
            || unique.len() != self.question_ids.len()
        {
            return Err("invalid_paper_questions");
        }
        self.title = title.into();
        Ok(self)
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Snapshot {
    pub title: String,
    pub template: Template,
    pub questions: Vec<SelectedQuestion>,
}

impl Snapshot {
    pub(crate) fn has_answers(&self) -> bool {
        self.questions
            .iter()
            .any(|question| !question.answer.trim().is_empty())
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Artifact {
    Questions,
    Answers,
}

impl Artifact {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Questions => "questions",
            Self::Answers => "answers",
        }
    }
}

#[derive(Clone)]
pub(crate) struct Artifacts(Arc<dyn FileStore>);

impl Artifacts {
    pub(crate) fn new(store: impl FileStore + 'static) -> Self {
        Self(Arc::new(store))
    }

    pub(crate) fn read(&self, id: &str, artifact: Artifact) -> io::Result<Vec<u8>> {
        let name = artifact_key(id, artifact);
        self.0.read(key(&name)?, MAX_PDF_BYTES)
    }

    fn save(&self, id: &str, artifact: Artifact, bytes: &[u8]) -> io::Result<()> {
        let name = artifact_key(id, artifact);
        self.0.put_if_absent(key(&name)?, bytes)
    }

    fn remove(&self, id: &str) -> io::Result<()> {
        for artifact in [Artifact::Questions, Artifact::Answers] {
            let name = artifact_key(id, artifact);
            self.0.remove(key(&name)?)?;
        }
        Ok(())
    }
}

fn artifact_key(id: &str, artifact: Artifact) -> String {
    format!("{id}-{}.pdf", artifact.name())
}

fn key(name: &str) -> io::Result<FileKey<'_>> {
    FileKey::new(name).ok_or_else(|| io::ErrorKind::InvalidInput.into())
}

pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub(crate) fn new_id() -> Result<String, getrandom::Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes)?;
    Ok(format!("{:032x}", u128::from_be_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_titles_and_ordered_question_ids() {
        let submit = |title: &str, ids| {
            Submission {
                title: title.into(),
                question_ids: ids,
                template: Template::Ccho,
            }
            .validated()
        };
        let valid = submit(" 练习卷 ", vec![3, 1, 2]).unwrap();
        assert_eq!(valid.title, "练习卷");
        assert_eq!(valid.question_ids, [3, 1, 2]);
        assert_eq!(submit("A\nB", vec![1]).unwrap_err(), "invalid_paper_title");
        for ids in [vec![], vec![0], vec![2, 2], (1..=41).collect()] {
            assert_eq!(
                submit("练习卷", ids).unwrap_err(),
                "invalid_paper_questions"
            );
        }
    }

    #[test]
    fn only_accepts_canonical_random_task_ids() {
        let first = new_id().unwrap();
        let second = new_id().unwrap();
        assert_ne!(first, second);
        assert!(valid_id(&first));
        assert!(!valid_id("ABCDEF00000000000000000000000000"));
        assert!(!valid_id("../questions"));
    }
}
