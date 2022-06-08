use gtest::System;
use io::*;

mod routines;
pub use routines::*;

#[test]
fn check_all_users_bet() {
    let sys = System::new();

    let game = common_init(&sys);

    check_user_move(&game, USERS[0], Move::Spock, COMMON_BET);
    failure_user_move(&game, USERS[1], Move::Spock, COMMON_BET / 10);
    check_user_move(&game, USERS[1], Move::Lizard, COMMON_BET);
    check_user_move(&game, USERS[2], Move::Spock, COMMON_BET);
    failure_user_move(&game, USERS[3], Move::Spock, COMMON_BET);
}
