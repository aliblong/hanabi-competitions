pub mod results;

use serde::{Serialize, Deserialize};
use chrono::{Weekday, Duration, Datelike};
use crate::{DbViewerPool, DbAdminPool, model::{Tx, UtcDateTime}};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct PartiallySpecifiedCompetition {
    pub num_players: i16,
    pub variant: String,
    pub end_datetime: Option<UtcDateTime>,
    pub deckplay_enabled: Option<bool>,
    pub empty_clues_enabled: Option<bool>,
    pub characters_enabled: Option<bool>,
    pub additional_rules: Option<String>,
    pub base_seed_names: Option<Vec<String>>,
}
#[derive(Serialize, Deserialize)]
pub struct Competition {
    pub num_players: i16,
    pub variant: String,
    pub end_datetime: UtcDateTime,
    pub deckplay_enabled: bool,
    pub empty_clues_enabled: bool,
    pub characters_enabled: bool,
    pub additional_rules: Option<String>,
    pub base_seed_names: Vec<String>,
}
impl PartiallySpecifiedCompetition {
    pub fn fill_missing_values_with_defaults(mut self) -> Competition {
        if self.end_datetime.is_none() {
            let today = chrono::offset::Utc::today();
            // If I'm setting the competition on Mon-Wed, it's probably the current competition
            let days_to_add = match today.weekday() {
                Weekday::Mon => 14,
                Weekday::Tue => 13,
                Weekday::Wed => 12,
                Weekday::Thu => 18,
                Weekday::Fri => 17,
                Weekday::Sat => 16,
                Weekday::Sun => 15,
            };
            // Default competition end time: Monday @ 13:00 UTC, which is just after I wake up
            self.end_datetime = Some((today + Duration::days(days_to_add)).and_hms(13, 0, 0));
        }
        if self.deckplay_enabled.is_none() { self.deckplay_enabled = Some(true) }
        if self.empty_clues_enabled.is_none() { self.empty_clues_enabled = Some(true) }
        if self.characters_enabled.is_none() { self.characters_enabled = Some(true) }
        if self.base_seed_names.is_none() {
            let base_seed_prefix = format!(
                "hl-comp-{}", self.end_datetime.unwrap().date().format("%Y-%m-%d")
            );
            self.base_seed_names = Some(vec![
                format!("{}-1", base_seed_prefix),
                format!("{}-2", base_seed_prefix),
                format!("{}-3", base_seed_prefix),
                format!("{}-4", base_seed_prefix),
            ]);
        }
        Competition {
            num_players: self.num_players,
            variant: self.variant,
            end_datetime: self.end_datetime.unwrap(),
            deckplay_enabled: self.deckplay_enabled.unwrap(),
            empty_clues_enabled: self.empty_clues_enabled.unwrap(),
            characters_enabled: self.characters_enabled.unwrap(),
            additional_rules: self.additional_rules,
            base_seed_names: self.base_seed_names.unwrap(),
        }
    }
}

pub async fn get_competition_names(
    pool: &DbViewerPool,
) -> Result<Vec<String>> {
    let competition_name_records = sqlx::query!(
        "select name
        from competition_names
        order by name desc",
    ).fetch_all(&pool.0).await?;
    let competition_names = competition_name_records.into_iter().map(
        |record| record.name.unwrap()).collect();
    Ok(competition_names)
}

pub async fn get_active_competitions(
    pool: &DbViewerPool,
) -> Result<Vec<Competition>> {
    let active_competitions_without_seeds = sqlx::query!(
        "select
            competitions.id
          , num_players
          , variant_id
          , variants.name variant_name
          , end_datetime
          , deckplay_enabled
          , empty_clues_enabled
          , characters_enabled
          , additional_rules
        from competitions
        join variants on variant_id = variants.id
        join competition_names on competitions.id = competition_names.competition_id
        where end_datetime > now()",
    ).fetch_all(&pool.0).await?;
    let mut active_competitions = Vec::new();
    for active_competition_without_seeds in active_competitions_without_seeds {
        let base_seed_name_records = sqlx::query!(
            "select base_name
            from competition_seeds
            where
                competition_id = $1
                and variant_id = $2
                and num_players = $3
            ",
            active_competition_without_seeds.id,
            active_competition_without_seeds.variant_id,
            active_competition_without_seeds.num_players,
        ).fetch_all(&pool.0).await?;
        active_competitions.push(Competition{
            num_players: active_competition_without_seeds.num_players,
            variant: active_competition_without_seeds.variant_name,
            end_datetime: active_competition_without_seeds.end_datetime,
            deckplay_enabled: active_competition_without_seeds.deckplay_enabled,
            empty_clues_enabled: active_competition_without_seeds.empty_clues_enabled,
            characters_enabled: active_competition_without_seeds.characters_enabled,
            additional_rules: active_competition_without_seeds.additional_rules,
            base_seed_names: base_seed_name_records.into_iter().map(|record|
                record.base_name).collect(),
        })
    }
    Ok(active_competitions)
}

pub async fn add_competitions(
    pool: &DbAdminPool,
    partially_specified_competitions: Vec<PartiallySpecifiedCompetition>,
) -> Result<()> {
    // if a single competition causes an error, don't commit any
    let mut tx = pool.0.begin().await?;
    for competition in partially_specified_competitions {
        // I'd prefer to use references throughout, but I don't know a better pattern that would
        // allow me to pass the same mutable borrow to multiple functions.
        tx = add_competition(tx, competition).await?;
    }
    sqlx::query("select update_competition_names()").execute(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}


pub async fn add_competition(
    mut tx: Tx,
    partially_specified_competition: PartiallySpecifiedCompetition,
) -> Result<Tx> {
    let competition = partially_specified_competition.fill_missing_values_with_defaults();
    let variant_id = sqlx::query!(
        "SELECT id from variants WHERE name = $1",
        competition.variant
    ).fetch_one(&mut tx).await?.id;

    let competition_id = sqlx::query!(
        "INSERT INTO competitions (
            end_datetime
          , num_players
          , variant_id
          , deckplay_enabled
          , empty_clues_enabled
          , characters_enabled
          , additional_rules
        ) VALUES (
            $1
          , $2
          , $3
          , $4
          , $5
          , $6
          , $7
        ) RETURNING id",
        competition.end_datetime,
        competition.num_players,
        variant_id,
        competition.deckplay_enabled,
        competition.empty_clues_enabled,
        competition.characters_enabled,
        competition.additional_rules,
    ).fetch_one(&mut tx).await?.id;

    for base_seed_name in competition.base_seed_names {
        sqlx::query!(
            "INSERT INTO competition_seeds (
                competition_id
              , num_players
              , variant_id
              , base_name
            ) VALUES (
                $1
              , $2
              , $3
              , $4
            )",
            competition_id,
            competition.num_players,
            variant_id,
            base_seed_name,
        ).execute(&mut tx).await?;
    }
    Ok(tx)
}
