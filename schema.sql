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
  , end_time timestamp not null check(end_time > date('2020-05-01'))
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

create materialized view competition_names as (
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
)
