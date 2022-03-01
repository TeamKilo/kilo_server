use crate::game::adapter::Stage;
use crate::game::{GameId, GameManagerError, GameType};
use actix_web::Result;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const LIST_GAME_SUMMARY_COUNT: usize = 20;

#[derive(Serialize)]
pub struct GameSummary {
    pub game_id: GameId,
    pub game_type: GameType,
    pub players: Vec<String>,
    pub stage: Stage,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    GameType,
    Players,
    Stage,
    LastUpdated,
}

pub struct SearchOptions {
    pub page: usize,
    pub sort_order: SortOrder,
    pub sort_key: SortKey,
    pub game_type: Option<GameType>,
    pub players: Option<usize>,
    pub stage: Option<Stage>,
}

pub struct SearchEngine;

impl SearchEngine {
    pub fn apply<I>(summaries: I, options: &SearchOptions) -> Result<Vec<GameSummary>>
    where
        I: Iterator<Item = GameSummary>,
    {
        let SearchOptions {
            page,
            sort_order,
            sort_key,
            game_type,
            players,
            stage,
        } = options;

        let page = *page;
        let sort_order = *sort_order;
        let sort_key = *sort_key;

        if page == 0 {
            return Err(actix_web::Error::from(GameManagerError::InvalidPage));
        }

        let skip = (page - 1) * LIST_GAME_SUMMARY_COUNT;

        Ok(summaries
            .sorted_by(|a, b| SearchEngine::compare_summaries(a, b, sort_key, sort_order))
            .skip(skip)
            .filter(|s| game_type.map_or(true, |x| s.game_type == x))
            .filter(|s| players.map_or(true, |x| s.players.len() == x))
            .filter(|s| stage.map_or(true, |x| s.stage == x))
            .take(LIST_GAME_SUMMARY_COUNT)
            .collect())
    }

    fn next_sort_key(sort_key: SortKey) -> SortKey {
        match sort_key {
            SortKey::GameType => SortKey::Players,
            SortKey::Players => SortKey::Stage,
            SortKey::Stage => SortKey::LastUpdated,
            SortKey::LastUpdated => SortKey::GameType,
        }
    }

    fn compare_summaries(
        a: &GameSummary,
        b: &GameSummary,
        sort_key: SortKey,
        sort_order: SortOrder,
    ) -> Ordering {
        let mut current_sort_key = sort_key;
        let mut ordering = Ordering::Equal;

        for _ in 0..4 {
            ordering = match current_sort_key {
                SortKey::GameType => Ord::cmp(&a.game_type, &b.game_type),
                SortKey::Players => Ord::cmp(&a.players.len(), &b.players.len()),
                SortKey::Stage => Ord::cmp(&a.stage, &b.stage),
                SortKey::LastUpdated => Ord::cmp(&a.last_updated, &b.last_updated),
            };
            match ordering {
                Ordering::Equal => current_sort_key = SearchEngine::next_sort_key(current_sort_key),
                _ => break,
            }
        }

        if sort_order == SortOrder::Asc {
            ordering
        } else {
            ordering.reverse()
        }
    }
}
