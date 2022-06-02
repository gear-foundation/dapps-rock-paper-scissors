use gstd::prelude::*;
use gtest::System;
use io::*;

mod routines;
pub use routines::*;

#[test]
fn check_common() {
    let sys = System::new();
    let game = common_init(&sys);

    check_change_lobby(&game, USERS[0], USERS);

    check_user_move(&game, USERS[2], Move::Rock, COMMON_BET);
    check_user_move(&game, USERS[3], Move::Rock, COMMON_BET);
    check_user_move(&game, USERS[0], Move::Rock, COMMON_BET);
    check_user_move(&game, USERS[1], Move::Rock, COMMON_BET);
}

#[test]
fn check_changing_twice() {
    let sys = System::new();
    let game = common_init(&sys);

    check_change_lobby(&game, USERS[0], USERS);
    check_change_lobby(&game, USERS[0], &USERS[0..2]);

    failure_user_move(&game, USERS[2], Move::Rock, COMMON_BET);
    failure_user_move(&game, USERS[3], Move::Rock, COMMON_BET);

    check_user_move(&game, USERS[0], Move::Rock, COMMON_BET);
    check_user_move(&game, USERS[1], Move::Rock, COMMON_BET);
}

#[test]
fn check_after_game_over() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Rock, Move::Lizard, Move::Lizard];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_change_lobby(&game, USERS[0], &USERS[0..3]);

    failure_user_move(&game, USERS[3], Move::Rock, COMMON_BET);
    check_user_move(&game, USERS[2], Move::Rock, COMMON_BET);
}

#[test]
fn check_after_stop_the_game() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Rock, Move::Spock, Move::Lizard];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_stop_the_game(&game, USERS[0], USERS);
    check_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_during_the_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = common_init(&sys);

    check_user_move(&game, USERS[0], moves[0].clone(), COMMON_BET);
    check_user_move(&game, USERS[2], moves[2].clone(), COMMON_BET);

    failure_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_before_first_reveal_in_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    failure_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_during_reveal_in_first_round_with_some_reveals() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    check_user_reveal_with_continue(&game, USERS[1], moves[1].clone());
    check_user_reveal_with_continue(&game, USERS[3], moves[3].clone());
    failure_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_in_start_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    failure_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_all_players_in_progress_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_user_move(&game, USERS[0], moves[0].clone(), 0);
    check_user_move(&game, USERS[2], moves[2].clone(), 0);
    failure_change_lobby(&game, USERS[0], &USERS[0..3]);
}

#[test]
fn check_not_owner() {
    let sys = System::new();
    let game = common_init(&sys);

    failure_change_lobby(&game, USERS[1], USERS);
}
