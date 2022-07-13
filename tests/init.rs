use gtest::{Program, System};
use io::*;

mod routines;
pub use routines::*;

pub fn init(
    sys: &System,
    owner_user: u64,
    bet_size: u128,
    players_count_limit: u8,
    entry_timeout: u64,
    move_timeout: u64,
    reveal_timeout: u64,
) -> Program {
    sys.init_logger();
    USERS
        .iter()
        .copied()
        .for_each(|id| sys.mint_to(id, 1_000_000_000));

    let program = Program::current(sys);
    let result = program.send(
        owner_user,
        GameConfig {
            bet_size,
            players_count_limit,
            entry_timeout,
            move_timeout,
            reveal_timeout,
        },
    );

    assert!(!result.main_failed());
    assert!(result.log().is_empty());

    program
}

#[test]
fn check_all_users_bet() {
    let sys = System::new();
    let entry_timout = COMMON_TIMOUT;
    let move_timout = COMMON_TIMOUT + 1;
    let reveal_timout = COMMON_TIMOUT + 2;

    let game = init(
        &sys,
        USERS[0],
        COMMON_BET,
        COMMON_PLAYERS_COUNT_LIMIT,
        entry_timout,
        move_timout,
        reveal_timout,
    );

    register_players(&game, &USERS[0..3], COMMON_BET);
    failure_register_player(&game, USERS[3], COMMON_BET - 1);
    failure_user_move(&game, USERS[0], Move::Spock);

    sys.spend_blocks(blocks_count(entry_timout));
    failure_user_move(&game, USERS[0], Move::Spock);
    sys.spend_blocks(1);
    check_user_move(&game, USERS[0], Move::Spock);
    check_user_move(&game, USERS[1], Move::Spock);
    failure_user_move(&game, USERS[1], Move::Lizard);
    failure_user_move(&game, USERS[3], Move::Spock);

    failure_user_reveal(&game, USERS[0], Move::Spock);
    sys.spend_blocks(blocks_count(move_timout));
    failure_user_reveal(&game, USERS[0], Move::Spock);
    sys.spend_blocks(1);
    check_user_reveal_with_continue(&game, USERS[0], Move::Spock);
    failure_user_reveal(&game, USERS[2], Move::Lizard);
    failure_user_reveal(&game, USERS[1], Move::Lizard);
    sys.spend_blocks(blocks_count(reveal_timout));
    sys.spend_blocks(1);

    register_players(&game, &USERS[0..3], COMMON_BET);
}
