drop table if exists players cascade;
drop table if exists aliases cascade;
drop table if exists seeds cascade;
drop table if exists games cascade;
drop table if exists whitelisted_games;
drop table if exists blacklisted_games;
drop table if exists game_players cascade;
drop table if exists variants cascade;
drop table if exists competitions cascade;
drop table if exists competition_seeds cascade;
drop table if exists characters cascade;
drop table if exists seed_characters cascade;
drop materialized view if exists competition_names cascade;
drop view if exists computed_competition_standings cascade;
