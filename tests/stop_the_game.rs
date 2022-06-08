use gstd::prelude::*;
use gtest::System;
use io::*;

mod routines;
pub use routines::*;

#[test]
fn check_during_the_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];

    let game = init_with_users(&sys, USERS);

    check_user_move(&game, USERS[0], moves[0].clone(), COMMON_BET);
    check_user_move(&game, USERS[2], moves[2].clone(), COMMON_BET);

    let rewarding_users = [USERS[0], USERS[2]];

    check_stop_the_game(&game, USERS[0], &rewarding_users);
}

#[test]
fn check_during_reveal_in_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];

    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    check_stop_the_game(&game, USERS[0], USERS);
}

#[test]
fn check_during_reveal_in_first_round_with_some_reveals() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);
    check_user_reveal_with_continue(&game, USERS[1], moves[1].clone());
    check_user_reveal_with_continue(&game, USERS[3], moves[3].clone());

    check_stop_the_game(&game, USERS[0], USERS);
}

#[test]
fn check_all_players_in_start_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);
    play_round(&game, USERS, &moves);

    check_stop_the_game(&game, USERS[0], USERS);
}

#[test]
fn check_all_players_in_progress_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);
    play_round(&game, USERS, &moves);
    check_user_move(&game, USERS[0], moves[0].clone(), 0);
    check_user_move(&game, USERS[2], moves[2].clone(), 0);

    check_stop_the_game(&game, USERS[0], USERS);
}

#[test]
fn check_not_all_players_in_progress_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Lizard, Move::Lizard];
    let game = init_with_users(&sys, USERS);
    play_round(&game, USERS, &moves);
    check_user_move(&game, USERS[0], moves[0].clone(), 0);
    check_user_move(&game, USERS[2], moves[2].clone(), 0);

    let rewarding_users = [USERS[0], USERS[2], USERS[3]];
    check_stop_the_game(&game, USERS[0], &rewarding_users);
}

#[test]
fn check_game_is_not_in_progress() {
    let sys = System::new();
    let game = init_with_users(&sys, USERS);

    failure_stop_the_game(&game, USERS[0]);
}

#[test]
fn check_not_owner_stop() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Lizard, Move::Lizard];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    failure_stop_the_game(&game, USERS[1]);
}
