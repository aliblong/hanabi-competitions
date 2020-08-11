use std::collections::HashMap;
use sqlx::{FromRow, Row};

use serde::{Serialize, Deserialize};
use chrono::{Weekday, Duration, Datelike};
use crate::{DbViewerPool, DbAdminPool, model::{Tx, UtcDateTime}};
use anyhow::Result;
use sqlx::postgres::PgRow;

//// This is a huge pain in the ass at the time I'm writing this code
//#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
//#[sqlx(rename = "scoring_type", rename_all = "lowercase")]
//enum ScoringType { Standard, Speedrun }

#[derive(Serialize, Deserialize)]
pub struct SeriesCompetitions {
    series_name: String,
    competitions: Vec<CompetitionWithDerivedQuantities>,
}

#[derive(Serialize, Debug)]
pub struct CompetitionNestedResults {
    pub competition_with_derived_quantities: CompetitionWithDerivedQuantities,
    pub team_results: Vec<TeamResults>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompetitionWithDerivedQuantities {
    pub competition: Competition,
    pub competition_name: String,
    pub create_table_urls: Vec<BaseSeedNameCreateTableUrlPair>,
    pub formatted_time_control: String,
}

impl CompetitionWithDerivedQuantities {
    fn new(competition: Competition, competition_name: String) -> Self {
        let seed_name_create_table_url_pairs = competition.generate_create_table_urls()
            .into_iter().zip(competition.base_seed_names.iter())
            .map(|(create_table_url, base_seed_name)| BaseSeedNameCreateTableUrlPair {
                base_seed_name: base_seed_name.clone(),
                create_table_url
            }).collect();
        let formatted_time_control = match &competition.ruleset.time_control {
            None => "".to_owned(),
            Some(time_control) => {
                let base_time_duration =
                    chrono::Duration::seconds(time_control.base_time_seconds as i64);
                let turn_time_duration =
                    chrono::Duration::seconds(time_control.turn_time_seconds as i64);
                format!(
                    "{}:{} + {}:{}",
                    base_time_duration.num_minutes(), 
                    base_time_duration.num_seconds() % 60, 
                    turn_time_duration.num_minutes(), 
                    turn_time_duration.num_seconds() % 60, 
                )
            }
        };
        Self {
            create_table_urls: seed_name_create_table_url_pairs,
            competition,
            competition_name,
            formatted_time_control,
        }
    }
}

// This bit of ugliness is due to needing to pipe this into handlebars.
// Even if I wanted to figure out how to properly write a helper function,
// afaict, there's no good way to zip together vectors.
// It's not impossible to have base_seed_name be a reference to another field in the
// parent struct, but it's more headache than it's worth given the small perf impact.
#[derive(Serialize, Deserialize, Debug)]
pub struct BaseSeedNameCreateTableUrlPair {
    base_seed_name: String,
    create_table_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct PartiallySpecifiedCompetition {
    pub num_players: i16,
    pub variant_name: String,
    pub end_datetime: Option<UtcDateTime>,
    pub deckplay_enabled: Option<bool>,
    pub empty_clues_enabled: Option<bool>,
    pub characters_enabled: Option<bool>,
    pub scoring_type: Option<String>,
    pub time_control: Option<TimeControl>,
    pub additional_rules: Option<String>,
    pub base_seed_names: Option<Vec<String>>,
    pub series_names: Option<Vec<String>>,
}

impl PartiallySpecifiedCompetition {
    pub fn fill_missing_values_with_defaults(mut self) -> Competition {
        if self.end_datetime.is_none() {
            let today = chrono::offset::Utc::today();
            // If I'm setting the competition on Mon-Wed, it's probably the current competition
            // Default period of 2 weeks, ending Monday @ 13:00 UTC, which is just after I wake up
            let days_to_add = match today.weekday() {
                Weekday::Mon => 14,
                Weekday::Tue => 13,
                Weekday::Wed => 12,
                Weekday::Thu => 18,
                Weekday::Fri => 17,
                Weekday::Sat => 16,
                Weekday::Sun => 15,
            };
            self.end_datetime = Some((today + Duration::days(days_to_add)).and_hms(13, 0, 0));
        }
        if self.deckplay_enabled.is_none() { self.deckplay_enabled = Some(true) }
        if self.empty_clues_enabled.is_none() { self.empty_clues_enabled = Some(false) }
        if self.characters_enabled.is_none() { self.characters_enabled = Some(false) }
        if self.scoring_type.is_none() {
            self.scoring_type = Some("standard".to_owned());
            //self.scoring_type = Some(ScoringType::Standard);
        }
        if self.base_seed_names.is_none() {
            let base_seed_prefix = format!(
                "hc-{}", self.end_datetime.unwrap().date().format("%Y-%m-%d")
            );
            self.base_seed_names = Some(vec![
                format!("{}-1", base_seed_prefix),
                format!("{}-2", base_seed_prefix),
                format!("{}-3", base_seed_prefix),
                format!("{}-4", base_seed_prefix),
            ]);
        }
        if self.series_names.is_none() {
            self.series_names = Some(Vec::new());
        }
        Competition {
            ruleset: CompetitionRuleset {
                num_players: self.num_players,
                variant_name: self.variant_name,
                end_datetime: self.end_datetime.unwrap(),
                deckplay_enabled: self.deckplay_enabled.unwrap(),
                empty_clues_enabled: self.empty_clues_enabled.unwrap(),
                characters_enabled: self.characters_enabled.unwrap(),
                additional_rules: self.additional_rules,
                scoring_type: self.scoring_type.unwrap(),
                time_control: self.time_control,
            },
            base_seed_names: self.base_seed_names.unwrap(),
            series_names: self.series_names.unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Competition {
    pub ruleset: CompetitionRuleset,
    pub base_seed_names: Vec<String>,
    pub series_names: Vec<String>,
}

impl Competition {
    fn generate_create_table_urls(&self) -> Vec<String> {
        let mut create_table_urls = Vec::new();
        let ruleset = &self.ruleset;
        let time_control_query_parameters_str = match &self.ruleset.time_control {
            None => "".to_owned(),
            Some(time_control) => format!("\
                &timed=true\
                &timeBase={}\
                &timePerTurn={}\
                ",
                time_control.base_time_seconds,
                time_control.turn_time_seconds,
            ),
        };
        for base_seed_name in &self.base_seed_names {
            create_table_urls.push(format!("https://hanab.live/create-table?\
                name=!seed%20{}\
                &variantName={}\
                &deckPlays={}\
                &emptyClues={}\
                &detrimentalCharacters={}\
                {}
                ",
                //&speedrun={}\
                urlencoding::encode(&base_seed_name),
                urlencoding::encode(&ruleset.variant_name),
                ruleset.deckplay_enabled,
                ruleset.empty_clues_enabled,
                // TODO: enable whatever feature we get for speedruns that don't end
                // when a critical card is lost
                // ruleset.scoring_type == "speedrun",
                ruleset.characters_enabled,
                time_control_query_parameters_str,
            ));
        }
        create_table_urls
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompetitionRuleset {
    pub num_players: i16,
    pub variant_name: String,
    pub end_datetime: UtcDateTime,
    pub deckplay_enabled: bool,
    pub empty_clues_enabled: bool,
    pub characters_enabled: bool,
    pub scoring_type: String,
    pub time_control: Option<TimeControl>,
    pub additional_rules: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimeControl {
    pub base_time_seconds: i16,
    pub turn_time_seconds: i16,
}

#[derive(Serialize, Debug)]
pub struct TeamResults {
    pub players: Vec<Option<String>>,
    pub final_rank: i64,
    pub fractional_mp: f64,
    pub sum_mp: i64,
    pub game_results: Vec<Option<GameResult>>,
}

#[derive(Serialize, Debug)]
pub struct GameResult {
    pub seed_matchpoints: i32,
    pub score: i16,
    pub turns: i16,
    pub site_game_id: i64,
    pub replay_url: String,
    pub game_duration_seconds: i32,
}

// This is essentially an intermediate record that gets passed around so we can join it to the
// `competitions` and `variants` tables more conveniently.
#[derive(FromRow)]
pub struct CompetitionRulesetWithIds {
    pub competition_id: i16,
    pub variant_id: i32,
    pub num_players: i16,
    pub variant_name: String,
    pub end_datetime: UtcDateTime,
    pub deckplay_enabled: bool,
    pub empty_clues_enabled: bool,
    pub characters_enabled: bool,
    pub scoring_type: String,
    pub base_time_seconds: Option<i16>,
    pub turn_time_seconds: Option<i16>,
    pub additional_rules: Option<String>,
}

#[derive(thiserror::Error, Debug)]
enum GetCompetitionError {
    #[error("No competition with that name was found")]
    NotFound,
}

// This is quite similar to model::result::CombinedResult, but this one is tailored to be a good
// default view of the results, intended to be nested, whereas the other is more of a raw, complete
// view, for analysis.
#[derive(FromRow, Serialize, Clone, Debug)]
struct CompetitionFlatResult {
    pub final_rank: i64,
    pub fractional_mp: f64,
    pub sum_mp: i64,
    pub player_name: String,
    pub base_seed_name: String,
    pub seed_matchpoints: i32,
    pub site_game_id: i64,
    pub replay_url: String,
    pub score: i16,
    pub turns: i16,
    // support for the INTERVAL type landed literally less than a week ago, so look out for a
    // release: https://github.com/launchbadge/sqlx/pull/271
    pub game_duration_seconds: i32,
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

pub async fn get_competition_and_nested_results(
    pool: &DbViewerPool,
    competition_name: &str
) -> Result<CompetitionNestedResults> {
    let competition_id = get_competition_id(pool, competition_name).await?;
    if competition_id.is_none() {
        return Err(GetCompetitionError::NotFound.into());
    }
    let competition_ruleset_with_ids = get_competition_with_ids(pool, competition_id.unwrap()).await?;
    let competition_with_derived_quantities =
        competition_with_derived_quantities_from_ruleset_with_ids(
            pool,
            competition_ruleset_with_ids,
        ).await?;
    let competition_flat_results = get_competition_flat_results(
        pool,
        competition_name
    ).await?;
    let mut nested_results = nest_competition_results(
        competition_with_derived_quantities,
        competition_flat_results
    );
    nested_results.team_results.sort_unstable_by_key(|record| record.final_rank);
    Ok(nested_results)
}

pub async fn get_active_competitions(
    pool: &DbViewerPool,
) -> Result<Vec<CompetitionWithDerivedQuantities>> {
    let active_competition_ids = get_active_competition_ids(pool).await?;
    let mut active_competitions_rulesets_with_ids = Vec::new();
    for id in active_competition_ids.into_iter() {
        active_competitions_rulesets_with_ids.push(get_competition_with_ids(pool, id).await?);
    }
    let mut competitions_with_derived_quantities = Vec::new();
    for ruleset_with_ids in active_competitions_rulesets_with_ids {
        competitions_with_derived_quantities.push(
            competition_with_derived_quantities_from_ruleset_with_ids(
                pool,
                ruleset_with_ids,
            ).await?
        );
    }
    Ok(competitions_with_derived_quantities)
}

async fn get_competition_id(
    pool: &DbViewerPool,
    competition_name: &str,
) -> Result<Option<i16>> {
    Ok(sqlx::query!(
        "select competition_id
        from competition_names
        where name = $1",
        competition_name,
    ).fetch_optional(&pool.0).await?.map(|record| record.competition_id.unwrap()))
}

pub async fn get_competition_with_ids(
    pool: &DbViewerPool,
    competition_id: i16,
) -> Result<CompetitionRulesetWithIds> {
    Ok(sqlx::query_as!(
        CompetitionRulesetWithIds,
        r#"select
            competitions.id competition_id
          , variant_id
          , num_players
          , variants.name variant_name
          , end_datetime
          , deckplay_enabled
          , empty_clues_enabled
          , characters_enabled
          , scoring_type::text
          , base_time_seconds
          , turn_time_seconds
          , additional_rules
        from competitions
        join variants on variant_id = variants.id
        where competitions.id = $1"#,
        // , scoring_type as "scoring_type: String"
        competition_id
    ).fetch_one(&pool.0).await?)
}

async fn get_active_competition_ids(
    pool: &DbViewerPool,
) -> Result<Vec<i16>> {
    Ok(sqlx::query!(
        r#"select competitions.id
        from competitions
        where end_datetime > now()"#
        // , scoring_type as "scoring_type: String"
    ).fetch_all(&pool.0).await?.into_iter().map(|record| record.id).collect())
}

// TODO: Most of the functions below could use a refactor
pub async fn competition_with_derived_quantities_from_ruleset_with_ids(
    pool: &DbViewerPool,
    competition_ruleset_with_ids: CompetitionRulesetWithIds,
) -> Result<CompetitionWithDerivedQuantities> {
    // logically guaranteed there will be a record
    let competition_name = sqlx::query!(
        "select name from competition_names where competition_id = $1",
        competition_ruleset_with_ids.competition_id
    ).fetch_one(&pool.0).await?.name.unwrap();
    let series_names = sqlx::query!(
        "select name
        from series_competitions
        join series on series_competitions.series_id = series.id
        where competition_id = $1",
        competition_ruleset_with_ids.competition_id
    ).fetch_all(&pool.0).await?.into_iter().map(|record| record.name).collect();
    let base_seed_name_records = sqlx::query!(
        "select base_name
        from competition_seeds
        where
            competition_id = $1
            and variant_id = $2
            and num_players = $3
        ",
        competition_ruleset_with_ids.competition_id,
        competition_ruleset_with_ids.variant_id,
        competition_ruleset_with_ids.num_players,
    ).fetch_all(&pool.0).await?;
    let competition = Competition {
        ruleset: CompetitionRuleset {
            num_players: competition_ruleset_with_ids.num_players,
            variant_name: competition_ruleset_with_ids.variant_name,
            end_datetime: competition_ruleset_with_ids.end_datetime,
            deckplay_enabled: competition_ruleset_with_ids.deckplay_enabled,
            empty_clues_enabled: competition_ruleset_with_ids.empty_clues_enabled,
            characters_enabled: competition_ruleset_with_ids.characters_enabled,
            scoring_type: competition_ruleset_with_ids.scoring_type,
            time_control: match (
                competition_ruleset_with_ids.base_time_seconds,
                competition_ruleset_with_ids.turn_time_seconds,
            ) {
                (None, None) => None,
                (Some(base_time_seconds), Some(turn_time_seconds)) => {
                    Some(TimeControl {
                        base_time_seconds,
                        turn_time_seconds,
                    })
                },
                _ => unreachable!(
                    "Base time and turn time have composite nullability, \
                    enforced by a db constraint"
                ),
            },
            additional_rules: competition_ruleset_with_ids.additional_rules,
        },
        base_seed_names: base_seed_name_records.into_iter().map(|record|
            record.base_name).collect(),
        series_names,
    };
    Ok(CompetitionWithDerivedQuantities::new(competition, competition_name))
}

async fn get_competition_flat_results(
    pool: &crate::DbViewerPool,
    competition_name: &str,
) -> Result<Vec<CompetitionFlatResult>> {
    let results = sqlx::query_as!(
        CompetitionFlatResult,
        "select
            final_rank
          , fractional_MP
          , sum_MP
          , player_name
          , base_seed_name
          , seed_matchpoints
          , replay_URL
          , site_game_id
          , score
          , turns
          , cast(round(extract(epoch from (datetime_game_ended - datetime_game_started))) as int)
                as game_duration_seconds
        from computed_competition_standings
        where competition_name = $1",
        competition_name,
    ).fetch_all(&pool.0).await?;
    Ok(results)
}

fn nest_competition_results(
    competition: CompetitionWithDerivedQuantities,
    flat_results: Vec<CompetitionFlatResult>,
) -> CompetitionNestedResults {
    let mut player_indexed_results = HashMap::new(); // ::<String, (Vec<i64>, Vec<CompetitionFlatResult>)>
    for result in flat_results.into_iter() {
        match player_indexed_results.get_mut(&result.player_name) {
            None => {
                player_indexed_results.insert(
                    result.player_name.clone(),
                    (
                        vec![result.site_game_id],
                        vec![Some(result)]
                    )
                );
            },
            Some((games, results)) => {
                games.push(result.site_game_id);
                results.push(Some(result));
            },
        }
    }
    let mut game_combination_indexed_results = HashMap::new(); // ::<String, TeamResults>
    for (player_name, (mut games, results)) in player_indexed_results.into_iter() {
        games.sort_unstable();
        let games_key = games.into_iter().map(|site_game_id| site_game_id.to_string())
            .collect::<Vec<String>>().join("-");
        match game_combination_indexed_results.get_mut(&games_key) {
            None => {
                game_combination_indexed_results.insert(
                    games_key,
                    (vec![Some(player_name)], results)
                );
            }
            Some((players, _)) => {
                players.push(Some(player_name));
            }
        }
    }
    let mut competition_nested_results = CompetitionNestedResults {
        competition_with_derived_quantities: competition, 
        team_results: Vec::new(),
    };
    for (_, (mut players, mut results)) in game_combination_indexed_results.into_iter() {
        players.sort_unstable();
        let competition =
            &competition_nested_results.competition_with_derived_quantities.competition;
        while players.len() < competition.ruleset.num_players as usize {
            players.push(None);
        }
        results.sort_unstable_by(|r1,  r2|
            r1.as_ref().unwrap().base_seed_name.cmp(&r2.as_ref().unwrap().base_seed_name));
        for (idx, base_seed_name) in competition.base_seed_names.iter().enumerate() {
            if results.iter().find(
                |&r| r.is_some() && &r.as_ref().unwrap().base_seed_name == base_seed_name
            ).is_none() {
                results.insert(idx, None);
            }
        }
        let first_result = results.iter().find(|&r| r.is_some()).unwrap().as_ref().unwrap();
        competition_nested_results.team_results.push(TeamResults{
            players,
            final_rank: first_result.final_rank, 
            fractional_mp: first_result.fractional_mp, 
            sum_mp: first_result.sum_mp, 
            game_results: results.into_iter().map(|optional_result| match optional_result {
                Some(result) => Some(GameResult{
                    seed_matchpoints: result.seed_matchpoints,
                    score: result.score,
                    turns: result.turns,
                    site_game_id: result.site_game_id,
                    replay_url: result.replay_url,
                    game_duration_seconds: result.game_duration_seconds,
                }),
                None => None,
            }).collect(),
        })
    }
    competition_nested_results
}

async fn add_competition(
    mut tx: Tx,
    partially_specified_competition: PartiallySpecifiedCompetition,
) -> Result<Tx> {
    let competition = partially_specified_competition.fill_missing_values_with_defaults();
    let ruleset = &competition.ruleset;
    let variant_id = sqlx::query!(
        "SELECT id from variants WHERE name = $1",
        ruleset.variant_name
    ).fetch_one(&mut tx).await?.id;

    let (base_time_seconds, turn_time_seconds) = match &ruleset.time_control {
        None => (None, None),
        Some(time_control) => (
            Some(time_control.base_time_seconds),
            Some(time_control.turn_time_seconds),
        ),
    };

    let competition_id = sqlx::query(
        r#"INSERT INTO competitions (
            end_datetime
          , num_players
          , variant_id
          , deckplay_enabled
          , empty_clues_enabled
          , characters_enabled
          , scoring_type
          , base_time_seconds
          , turn_time_seconds
          , additional_rules
        ) VALUES (
            $1
          , $2
          , $3
          , $4
          , $5
          , $6
          , cast($7 as scoring_type)
          , $8
          , $9
          , $10
        ) RETURNING id"#)
        .bind(ruleset.end_datetime)
        .bind(ruleset.num_players)
        .bind(variant_id)
        .bind(ruleset.deckplay_enabled)
        .bind(ruleset.empty_clues_enabled)
        .bind(ruleset.characters_enabled)
        .bind(&ruleset.scoring_type)
        .bind(base_time_seconds)
        .bind(turn_time_seconds)
        .bind(&ruleset.additional_rules)
        .map(|row: PgRow| row.get(0))
        .fetch_one(&mut tx).await?;

    for series_name in &competition.series_names {
        let series_id = sqlx::query!(
            "select id
            from series
            where name = $1",
            series_name,
        ).fetch_one(&mut tx).await?.id;
        sqlx::query!(
            "insert into series_competitions (
                series_id
              , competition_id
            ) values (
                $1
              , $2
            )",
            series_id,
            competition_id
        ).execute(&mut tx).await?;
    }

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
            ruleset.num_players,
            variant_id,
            base_seed_name,
        ).execute(&mut tx).await?;
    }
    Ok(tx)
}
