drop materialized view competition_names cascade;
create materialized view if not exists competition_names as (
    select
        competitions.id competition_id
      , concat(
            to_char(competitions.end_datetime, 'YYYY-MM-DD')
          , ' '
          , cast(competitions.num_players as text)
          , 'p '
          , variants.name
          , (case
                when scoring_type = 'speedrun'
                    then 'speedrun'
                -- constraint ensures turn_time_seconds is also not null
                when base_time_seconds is not null
                    then concat(
                        ' ['
                      , to_char(base_time_seconds * '1 second'::interval, 'MI:SS')
                      , ' + '
                      , to_char(turn_time_seconds * '1 second'::interval, 'MI:SS')
                      , ']'
                    )
                else ''
            end)
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
          , concat('https://hanab.live/replay/', games.site_game_id) replay_URL
          , games.site_game_id
          , games.score
          , games.turns
          , games.datetime_started datetime_game_started
          , games.datetime_ended datetime_game_ended
          , competitions.scoring_type
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
    selected_game_ids as (
        select distinct game_id
        from prioritized_games
        where priority = 1
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
          , cast(case
                when scoring_type = 'speedrun'
                    then rank() over(partition by seed_id order by
                        score desc,
                        datetime_game_ended - datetime_game_started
                    )
                else  -- standard
                    rank() over(partition by seed_id order by score desc, turns)
            end as int) as seed_rank
          , cast(count(*) over(partition by seed_id) as int) num_seed_participants
          , cast(count(*) over(partition by competition_id) as int) num_comp_participants
        from base_cte
        join selected_game_ids using(game_id)
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
                - (cast(count(*) over(partition by seed_id, seed_rank) as int) - 1)
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
);

create or replace view series_competition_results as (
    with first_n_competitions_by_series_and_player as (
        select
            series.id series_id
          , series.name series_name
          , series.first_n
          , series.top_n
          , player_name
          , competition_name
          , fractional_MP
            -- under the logic that competition name starts with deadline date, and that we
            -- won't have two competitions with the same date in the same series
          , row_number() over(partition by series.id, player_name order by competition_name)
                as nth_competition_by_series_and_player
        from computed_competition_standings
        join competition_names on competition_name = competition_names.name
        join series_competitions using(competition_id)
        join series on series_id = series.id
        group by
            series.id
          , series_name
          , first_n
          , top_n
          , player_name
          , competition_name
          , fractional_MP
    ),

    top_n_competitions_by_series_and_player as (
        select
            series_id
          , series_name
          , top_n
          , player_name
          , competition_name
          , fractional_MP
          , row_number() over(partition by series_id, player_name order by fractional_MP desc)
                as ranked_performance_by_series_and_player
        from first_n_competitions_by_series_and_player
        where (
            case
                when first_n is not null
                    then nth_competition_by_series_and_player <= first_n
                else true
            end
        )
        group by
            series_id
          , series_name
          , top_n
          , player_name
          , competition_name
          , fractional_MP
    )
    select
        series_name
      , player_name
      , competition_name
      , fractional_MP
    from top_n_competitions_by_series_and_player
    where (
        case
            when top_n is not null
                then ranked_performance_by_series_and_player <= top_n
            else true
        end
    )
);

create or replace view series_player_scores as (
    with base_view as (
        select
            player_name
          , series_name
          , case
                when series_name like 'All-time%'
                    then median(fractional_mp) * (1 + log(20, count(fractional_mp)))
                    -- use this factor if we want to stop inflating past 100 competitions
                    -- add an extra 1 to the competitions count so that a player with
                    -- 1 competition has nonzero score
                    --* greatest(log(100, count(fractional_mp) + 1), 1)
                else sum(fractional_mp)
            end as score
          , avg(fractional_mp) mean_frac_mp
        from series_competition_results
        group by player_name, series_name
    )
    select
        rank() over(partition by series_name order by score desc) rank
      , player_name
      , series_name
      , score
      , mean_frac_mp
    from base_view
);
