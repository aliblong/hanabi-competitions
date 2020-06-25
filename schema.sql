create table if not exists players (
    id serial primary key
  , name text not null unique check(length(name) > 0)
);

create table if not exists seeds (
    id smallserial primary key
  , name text not null check (length(name) > 0)
);

create table if not exists games (
    id bigint primary key check (id > 0)
  , seed_id smallint not null references seeds(id)
  , score smallint not null check(score >= 0)
  , turns smallint not null check(turns >= 0)
  , datetime_started timestamptz
  , datetime_ended timestamptz
);

create table if not exists game_players (
    game_id bigint primary key references games(id)
  , player_id int not null references players(id)
);

create table if not exists variants (
    -- https://github.com/Zamiell/hanabi-live/blob/master/data/variants.txt
    id int primary key check(id >= 0)
  , name text not null check(length(name) > 0)
);

create table if not exists competitions (
    id smallserial primary key
  , end_time timestamptz not null check(end_time > date('2020-05-01'))
  , n_players smallint not null check(n_players >= 2)
  , variant_id int not null references variants(id)
  , deckplay_enabled boolean default true
  , empty_clues_enabled boolean default false
  , characters_enabled boolean default false
  , additional_rules text
);

create table if not exists competition_seeds (
    competition_id smallint not null references competitions(id)
  , seed_id smallint not null references seeds(id)
);

create table if not exists characters (
    id smallint primary key check(id >= 0)
  , name text not null check(length(name) > 0)
);

create table if not exists seed_characters (
    seed_id smallint not null references seeds(id)
  , character_id smallint not null references characters(id)
);

create materialized view if not exists competition_names as (
    select
        competitions.id
      , concat(
            to_char(competitions.end_time, 'YYYY-MM-DD')
          , ' '
          , cast(competitions.n_players as text)
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
      , seeds.id seed_id
      , seeds.name seed_name
      , games.id game_id
      , games.score
      , games.turns
      , games.datetime_started datetime_game_started
      , games.datetime_ended datetime_game_ended
      , cast(
            rank() over(partition by seeds.id order by games.score desc, games.turns)
        as int) seed_rank
      , cast(count(*) over(partition by seeds.id) as int) num_seed_participants
    from competitions
    join competition_seeds on competition_seeds.competition_id = competitions.id
    join seeds on competition_seeds.seed_id = seeds.id
    join games on seeds.id = games.seed_id
),
mp_computed as (
    select
        competition_id
      , seed_id
      , seed_name
      , (
            2 * num_seed_participants
            - (cast(count(*) over(partition by seed_name, seed_rank) as int) - 1)
            - 2 * seed_rank
        ) as seed_matchpoints
      , num_seed_participants
      , game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from base_cte
),
mp_agg as (
    select
        competition_id
      , sum(seed_matchpoints) over(partition by competition_id, players.id) as sum_MP
      , 2 * (
            sum(num_seed_participants) over(partition by competition_id)
            - count(seed_id) over(partition by competition_id)
        ) as max_MP
      , players.name player_name
      , seed_id
      , seed_name
      , seed_matchpoints
      , game_id
      , score
      , turns
      , datetime_game_started
      , datetime_game_ended
    from mp_computed
    join game_players using(game_id)
    join players on game_players.player_id = players.id
)
select
    competition_names.name competition_name
  , rank() over(partition by competition_id order by sum_MP) final_rank
  , cast(sum_MP as real)/ max_MP as fractional_MP
  , sum_MP
  , player_name
  , seed_name
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
--  , seed_name
--  , game_id
--  , player_name
)
