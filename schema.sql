create table if not exists players (
    id int primary key generated always as identity
  , name text not null unique check(length(name) > 0)
);

create table if not exists aliases (
    alias_id int primary key references players(id)
  , primary_id int not null references players(id)
);

create table if not exists variants (
    id int primary key generated always as identity
    -- https://github.com/Zamiell/hanabi-live/blob/master/data/variants.json
  , site_variant_id int not null check(id >= 0)
  , name text not null check(length(name) > 0)
);

create table if not exists competitions (
    id smallint primary key generated always as identity
  , end_datetime timestamptz not null check(end_datetime > date('2020-05-01'))
  , end_date date generated always as (date(end_datetime at time zone 'UTC')) stored
  , num_players smallint not null check(num_players >= 2)
  , variant_id int not null references variants(id)
  , deckplay_enabled boolean not null default true
  , empty_clues_enabled boolean not null default false
  , characters_enabled boolean not null default false
  , additional_rules text
  , unique (end_date, num_players, variant_id)
    -- putting a unique constraint on id and any other columns to be used as a foreign
    -- composite key is required, and tbh I don't fully understand why; if id is the primary
    -- key, its combination with other columns is necessarily unique
  , unique (id, num_players)
);

--create or replace view competition_ruleset (
--    select
--      --  competitions.id
--        competition_names.name competition_name
--      , num_players
--      , variants.name variant_name
--      , end_datetime
--      , deckplay_enabled
--      , empty_clues_enabled
--      , characters_enabled
--      , additional_rules
--    from competitions
--    join variants on variant_id = variants.id
--    join competition_names on competitions.id = competition_names.competition_id
--)

create table if not exists competition_seeds (
    id smallint primary key generated always as identity
  , competition_id smallint not null
  , num_players smallint not null
  , foreign key (competition_id, num_players) references competitions (id, num_players) on delete cascade
  , variant_id int not null references variants(id)
  , base_name text not null check(length(base_name) > 0)
  , unique (num_players, variant_id, base_name)
);

create table if not exists games (
    id int primary key generated always as identity
  , site_game_id bigint check (id > 0)
  , seed_id smallint not null references competition_seeds(id) on delete cascade
  , score smallint not null check(score >= 0)
  , turns smallint not null check(turns >= 0)
  , datetime_started timestamptz
  , datetime_ended timestamptz
);

create table if not exists whitelisted_games (
    game_id int primary key references games(id) on delete cascade
  , reason text
);

create table if not exists blacklisted_games (
    game_id int primary key references games(id) on delete cascade
  , reason text
);

create table if not exists game_players (
    game_id int not null references games(id) on delete cascade
  , player_id int not null references players(id) on delete cascade
  , primary key (game_id, player_id)
);

create table if not exists characters (
    id smallint primary key check(id >= 0)
  , name text not null check(length(name) > 0)
);

create table if not exists seed_characters (
    seed_id smallint not null references competition_seeds(id) on delete cascade
  , character_id smallint not null references characters(id)
  , primary key (seed_id, character_id)
);

create table if not exists series (
    id smallint primary key generated always as identity
  , name text not null check(length(name) > 0)
  , first_n smallint check(coalesce(first_n, 1) > 0)
  , top_n smallint check(coalesce(top_n, 0) <= first_n and coalesce(top_n, 1) > 0)
);

create table if not exists series_competitions (
    series_id smallint not null references seriess(id) on delete cascade
  , competition_id smallint not null references competitions(id) on delete cascade
  , primary key (series_id, competition_id)
);

-- even though "name" is logically non-null, it's not possible to explicitly constain it as such,
-- except with a hack-y solution like in https://stackoverflow.com/a/47245081
create materialized view if not exists competition_names as (
    select
        competitions.id competition_id
      , concat(
            to_char(competitions.end_datetime, 'YYYY-MM-DD')
          , ' '
          , cast(competitions.num_players as text)
          , 'p '
          , variants.name
        ) as name
    from competitions
    join variants on competitions.variant_id = variants.id
);

create materialized view if not exists computed_competition_standings as (
with base_cte as (
    select
        competitions.id competition_id
      , competition_seeds.id seed_id
      , competition_seeds.base_name base_seed_name
      , games.id game_id
        -- if we start allowing play on different sites, revisit this
      , concat('https://hanabi.live/replay/', games.site_game_id) replay_URL
      , games.site_game_id
      , games.score
      , games.turns
      , games.datetime_started datetime_game_started
      , games.datetime_ended datetime_game_ended
    from competitions
    join competition_seeds on competition_seeds.competition_id = competitions.id
    join games on competition_seeds.id = games.seed_id
    where games.datetime_ended < competitions.end_datetime
),
game_participation as (
    select
        seed_id
      , game_id
      , datetime_game_started
      , coalesce(primary_accounts.id, actual_accounts.id) player_id
      , case 
            when whitelisted_games.game_id is not null
                then 1
            else 0
        end as is_whitelisted_game
    from base_cte
    join game_players using(game_id)
    join players actual_accounts on game_players.player_id = actual_accounts.id
    left join aliases on actual_accounts.id = aliases.alias_id
    left join players primary_accounts on aliases.primary_id = primary_accounts.id
    left join whitelisted_games using(game_id)
    where not exists (
        select b.game_id
        from blacklisted_games b
        where b.game_id = base_cte.game_id
    )
),
prioritized_games as (
    select
        game_id
      , row_number() over(
            partition by seed_id, player_id
            order by is_whitelisted_game desc, datetime_game_started
        ) priority
    from game_participation
),
games_selected as (
    select
        competition_id
      , seed_id
      , base_seed_name
      , game_id
      , replay_URL
      , site_game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
      , cast(
            rank() over(partition by seed_id order by score desc, turns)
        as int) seed_rank
      , cast(count(*) over(partition by seed_id) as int) num_seed_participants
      , cast(count(*) over(partition by competition_id) as int) num_comp_participants
    from base_cte
    where not exists (
            select p.game_id
            from prioritized_games p
            where priority > 1 and p.game_id = base_cte.game_id
        )
),
competition_num_unique_seeds as (
    select competitions.id, count(distinct competition_seeds.id) num_seeds
    from competitions
    join competition_seeds on competition_seeds.competition_id = competitions.id
    group by competitions.id
),
computed_mp as (
    select
        competition_id
      , seed_id
      , base_seed_name
      , (
            2 * num_seed_participants
            - (cast(count(*) over(partition by base_seed_name, seed_rank) as int) - 1)
            - 2 * seed_rank
        ) as seed_matchpoints
      , 2 * (num_comp_participants - num_seeds) as max_MP
      , game_id
      , replay_URL
      , site_game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from games_selected
    join competition_num_unique_seeds on competition_id = competition_num_unique_seeds.id
),
computed_mp_with_primary_player_ids as (
    select
        competition_id
      , seed_id
      , base_seed_name
      , seed_matchpoints
      , max_MP
      , game_id
      , replay_URL
      , site_game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
      , coalesce(primary_accounts.id, actual_accounts.id) player_id
      , coalesce(primary_accounts.name, actual_accounts.name) player_name
    from computed_mp
    join game_players using(game_id)
    join players actual_accounts on game_players.player_id = actual_accounts.id
    left join aliases on actual_accounts.id = aliases.alias_id
    left join players primary_accounts on aliases.primary_id = primary_accounts.id
),
mp_agg as (
    select
        competition_id
      , sum(seed_matchpoints) over(partition by competition_id, player_id) as sum_MP
      , player_id
      , player_name
      , seed_id
      , base_seed_name
      , seed_matchpoints
      , max_MP
      , replay_URL
      , site_game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from computed_mp_with_primary_player_ids
),
competition_player_sum_MP as (
    select distinct
        competition_id
      , player_id
      , sum_MP
    from mp_agg
),
competition_player_ranks as (
    select
        competition_id
      , player_id 
      , rank() over(partition by competition_id order by sum_MP desc) final_rank
    from competition_player_sum_MP
)
select
    competition_names.name competition_name
  , final_rank
  , case
        when max_MP = 0
            then null
        else
            cast(sum_MP as real)/ max_MP
    end as fractional_MP
  , sum_MP
  , player_name
  , base_seed_name
  , seed_matchpoints
  , replay_URL
  , site_game_id
  , score
  , turns
  , datetime_game_started
  , datetime_game_ended
  , characters.name character_name
from mp_agg
join competition_names using(competition_id)
join competition_player_ranks cpr using(competition_id, player_id)
left join seed_characters on mp_agg.seed_id = seed_characters.character_id
left join characters on seed_characters.character_id = characters.id
--order by
--    competition_name desc
--  , sum_MP desc
--  , base_seed_name
--  , game_id
--  , player_name
);

create or replace function update_computed_competition_standings()
--returns trigger language plpgsql
returns void
-- executes as superuser (postgres); only creator of matview can refresh it
security definer
as $$ begin
    refresh materialized view computed_competition_standings;
    return;
end $$
language plpgsql;

create or replace function update_competition_names()
--returns trigger language plpgsql
returns void
security definer
as $$ begin
    refresh materialized view competition_names;
    return;
end $$
language plpgsql;

-- Either I have a logic error, or it just takes a shitload of memory to keep refreshing the
-- matview below
--
---- postgres only allows deferral (e.g. until end of txn) for constraint triggers, and only
---- allows constraint triggers to be applied on a per-row basis
---- so this compromise solution will refresh the matview redundantly in the middle of a txn
---- oh, also there's no syntax for conditional creation of triggers... but there is so for
---- conditional deletion?
---- finally, we need to create the trigger separately for every table, afaik
--drop trigger if exists update_players_trigger_update_comp_standings on players;
--create trigger update_players_trigger_update_comp_standings
--after insert or update or delete or truncate on players
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_aliases_trigger_update_comp_standings on aliases;
--create trigger update_aliases_trigger_update_comp_standings
--after insert or update or delete or truncate on aliases
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_competitions_trigger_update_comp_standings on competitions;
--create trigger update_competitions_trigger_update_comp_standings
--after insert or update or delete or truncate on competitions
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_competition_seeds_trigger_update_comp_standings on competition_seeds;
--create trigger update_competition_seeds_trigger_update_comp_standings
--after insert or update or delete or truncate on competition_seeds
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_games_trigger_update_comp_standings on games;
--create trigger update_games_trigger_update_comp_standings
--after insert or update or delete or truncate on games
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_whitelisted_games_trigger_update_comp_standings on whitelisted_games;
--create trigger update_whitelisted_games_trigger_update_comp_standings
--after insert or update or delete or truncate on whitelisted_games
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_game_players_trigger_update_comp_standings on game_players;
--create trigger update_game_players_trigger_update_comp_standings
--after insert or update or delete or truncate on game_players
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_characters_trigger_update_comp_standings on characters;
--create trigger update_characters_trigger_update_comp_standings
--after insert or update or delete or truncate on characters
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_seed_characters_trigger_update_comp_standings on seed_characters;
--create trigger update_seed_characters_trigger_update_comp_standings
--after insert or update or delete or truncate on seed_characters
--for each statement
--execute function update_computed_competition_standings();
--
--drop trigger if exists update_competitions_trigger_update_competition_names on competitions;
--create trigger update_competitions_trigger_update_competition_names
--after insert or update or delete or truncate on competitions
--for each statement
--execute function update_competition_names();
--
--drop trigger if exists update_variants_trigger_update_competition_names on variants;
--create trigger update_variants_trigger_update_competition_names
--after insert or update or delete or truncate on variants
--for each statement
--execute function update_competition_names();
