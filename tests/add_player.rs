use gstd::prelude::*;
use gtest::System;
use io::*;

mod routines;
pub use routines::*;

#[test]
fn check_common() {
    let sys = System::new();
    let game = common_init(&sys);

    check_add_player(&game, USERS[0], USERS[3]);
    failure_add_player(&game, USERS[0], USERS[1]);
}

#[test]
fn check_move_after_adding() {
    let sys = System::new();
    let game = common_init(&sys);

    check_add_player(&game, USERS[0], USERS[3]);
    check_user_move(&game, USERS[3], Move::Rock, COMMON_BET);
}

#[test]
fn check_add_twice_the_same() {
    let sys = System::new();
    let game = common_init(&sys);

    check_add_player(&game, USERS[0], USERS[3]);
    failure_add_player(&game, USERS[0], USERS[3]);
}

#[test]
fn check_add_after_game_over() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Rock, Move::Lizard, Move::Lizard];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_add_after_stop_the_game() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Rock, Move::Spock, Move::Lizard];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_stop_the_game(&game, USERS[0], USERS);
    check_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_during_the_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = common_init(&sys);

    check_user_move(&game, USERS[0], moves[0].clone(), COMMON_BET);
    check_user_move(&game, USERS[2], moves[2].clone(), COMMON_BET);

    failure_add_player(&game, USERS[0], USERS[3]);
}

#[test]
fn check_before_first_reveal_in_first_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    failure_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_during_reveal_in_first_round_with_some_reveals() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = reach_reveal_stage_with_init(&sys, USERS, &moves);

    check_user_reveal_with_continue(&game, USERS[1], moves[1].clone());
    check_user_reveal_with_continue(&game, USERS[3], moves[3].clone());
    failure_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_in_start_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    failure_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_all_players_in_progress_of_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Scissors, Move::Rock];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    check_user_move(&game, USERS[0], moves[0].clone(), 0);
    check_user_move(&game, USERS[2], moves[2].clone(), 0);
    failure_add_player(&game, USERS[0], USERS[3] + 1);
}

#[test]
fn check_add_retired_player_in_second_round() {
    let sys = System::new();
    let moves = [Move::Lizard, Move::Paper, Move::Lizard, Move::Lizard];
    let game = init_with_users(&sys, USERS);

    play_round(&game, USERS, &moves);
    failure_add_player(&game, USERS[0], USERS[1]);
}

#[test]
fn check_not_owner_add() {
    let sys = System::new();
    let game = common_init(&sys);

    failure_add_player(&game, USERS[1], USERS[3]);
}
