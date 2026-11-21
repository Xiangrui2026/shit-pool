use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use super::{TagMatch, not_found, valid_id, validated_tags};
use crate::{
    http::{AppState, error::ApiError},
    questions::{self, QuestionScore, RankOrder, RankedQuestion, Vote},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rating {
    vote: Vote,
}

#[derive(Deserialize)]
pub(super) struct RankingQuery {
    order: RankOrder,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    tag_match: TagMatch,
}

pub(super) async fn rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(rating): Json<Rating>,
) -> Result<Json<QuestionScore>, ApiError> {
    let id = valid_id(id)?;
    questions::rate(state.pool()?, id, rating.vote)
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

pub(super) async fn ranking(
    State(state): State<AppState>,
    Query(query): Query<RankingQuery>,
) -> Result<Json<Vec<RankedQuestion>>, ApiError> {
    let tags = validated_tags(&query.tags)?;
    let items = questions::ranking(
        state.pool()?,
        query.order,
        &tags,
        matches!(query.tag_match, TagMatch::All),
    )
    .await?;
    Ok(Json(items))
}
