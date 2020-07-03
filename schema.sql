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
  , end_time timestamptz not null check(end_time > date('2020-05-01'))
  , end_date date generated always as (date(end_time at time zone 'UTC')) stored
  , num_players smallint not null check(num_players >= 2)
  , variant_id int not null references variants(id)
  , deckplay_enabled boolean default true
  , empty_clues_enabled boolean default false
  , characters_enabled boolean default false
  , additional_rules text
  , unique (end_date, num_players, variant_id)
    -- putting a unique constraint on id and any other columns to be used as a foreign
    -- composite key is required, and tbh I don't fully understand why; if id is the primary
    -- key, its combination with other columns is necessarily unique
  , unique (id, num_players)
);

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

create materialized view if not exists competition_names as (
    select
        competitions.id
      , concat(
            to_char(competitions.end_time, 'YYYY-MM-DD')
          , ' '
          , cast(competitions.num_players as text)
          , 'p '
          , variants.name
        ) as name
    from competitions
    join variants on competitions.variant_id = variants.id
);

create or replace view computed_competition_standings as (
with base_cte as (
    select
        competitions.id competition_id
      , competition_seeds.id seed_id
      , competition_seeds.base_name base_seed_name
      , games.id game_id
      , games.score
      , games.turns
      , games.datetime_started datetime_game_started
      , games.datetime_ended datetime_game_ended
      , cast(
            rank() over(partition by competition_seeds.id order by games.score desc, games.turns)
        as int) seed_rank
      , cast(count(*) over(partition by competition_seeds.id) as int) num_seed_participants
      , cast(count(*) over(partition by competitions.id) as int) num_comp_participants
    from competitions
    join competition_seeds on competition_seeds.competition_id = competitions.id
    join games on competition_seeds.id = games.seed_id
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
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from base_cte
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
      , player_name
      , seed_id
      , base_seed_name
      , seed_matchpoints
      , max_MP
      , game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from computed_mp_with_primary_player_ids
)
select
    competition_names.name competition_name
  , rank() over(partition by competition_id order by sum_MP desc) final_rank
  , cast(sum_MP as real)/ max_MP as fractional_MP
  , sum_MP
  , player_name
  , base_seed_name
  , seed_matchpoints
  , game_id
  , score
  , turns
  , datetime_game_started
  , datetime_game_ended
  , characters.name character_name
from mp_agg
join competition_names on competition_id = competition_names.id
left join seed_characters on mp_agg.seed_id = seed_characters.character_id
left join characters on seed_characters.character_id = characters.id
--order by
--    competition_name desc
--  , sum_MP desc
--  , base_seed_name
--  , game_id
--  , player_name
)
