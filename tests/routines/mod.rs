use gstd::{prelude::*, ActorId, Encode};
use gtest::{Program, RunResult, System};
use io::*;

pub const USERS: &[u64] = &[3, 4, 5, 6];
pub const DEFAULT_PASSWORD: &str = "pass12";
pub const COMMON_BET: u128 = 1_000_000;

pub fn common_init(sys: &System) -> Program {
    init_with_users(sys, &USERS[0..3])
}

pub fn init_with_users<'a>(sys: &'a System, users: &[u64]) -> Program<'a> {
    init(sys, USERS[0], users, COMMON_BET)
}

fn init<'a>(sys: &'a System, owner_user: u64, players: &[u64], bet_size: u128) -> Program<'a> {
    sys.init_logger();

    let program = Program::current(sys);
    let result = program.send(
        owner_user,
        InitConfig {
            bet_size,
            lobby_players: players.iter().map(|user| (*user).into()).collect(),
        },
    );

    assert!(!result.main_failed());
    assert!(result.log().is_empty());

    program
}

pub fn reach_reveal_stage_with_init<'a>(
    sys: &'a System,
    users: &[u64],
    moves: &[Move],
) -> Program<'a> {
    let game = init_with_users(sys, users);
    reach_reveal_stage(&game, users, moves);

    game
}

pub fn reach_reveal_stage(game: &Program, users: &[u64], moves: &[Move]) {
    assert_eq!(users.len(), moves.len());

    users
        .iter()
        .copied()
        .zip(moves.iter().cloned())
        .for_each(|(user, users_move)| check_user_move(game, user, users_move, COMMON_BET));
}

pub fn play_round(game: &Program, users: &[u64], moves: &[Move]) -> RunResult {
    reach_reveal_stage(game, users, moves);

    for (user, users_move) in users
        .iter()
        .take(users.len() - 1)
        .zip(moves.iter().take(users.len() - 1))
    {
        check_user_reveal_with_continue(game, *user, users_move.clone());
    }

    try_to_reveal(game, *users.last().unwrap(), moves.last().cloned().unwrap())
}

pub fn check_user_move(program: &Program, player: u64, users_move: Move, bet: u128) {
    let result = try_to_move(program, player, users_move, bet);

    assert!(result.contains(&(player, Event::SuccessfulMove(player.into()).encode())));
}

pub fn failure_user_move(program: &Program, player: u64, users_move: Move, bet: u128) {
    let result = try_to_move(program, player, users_move, bet);

    assert!(result.main_failed());
}

pub fn try_to_move(program: &Program, player: u64, users_move: Move, bet: u128) -> RunResult {
    let move_with_pass = users_move.number().to_string() + DEFAULT_PASSWORD;
    let hash_bytes = sp_core_hashing::blake2_256(move_with_pass.as_bytes());
    let hex_hash = to_hex_string(hash_bytes);
    program.send_with_value(player, Action::MakeMove(hex_hash), bet)
}

fn to_hex_string(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub fn check_user_reveal_with_continue(program: &Program, player: u64, users_move: Move) {
    let result = try_to_reveal(program, player, users_move);

    assert!(result.contains(&(
        player,
        Event::SuccessfulReveal(RevealResult::Continue).encode()
    )));
}

pub fn check_user_reveal_with_next_round(
    program: &Program,
    player: u64,
    users_move: Move,
    next_round_players: BTreeSet<ActorId>,
) {
    let result = try_to_reveal(program, player, users_move);

    assert!(result.contains(&(
        player,
        Event::SuccessfulReveal(RevealResult::NextRoundStarted {
            players: next_round_players
        })
        .encode()
    )));
}

pub fn check_user_reveal_with_game_over(
    program: &Program,
    player: u64,
    users_move: Move,
    winner: ActorId,
) {
    let result = try_to_reveal(program, player, users_move);

    assert!(result.contains(&(
        player,
        Event::SuccessfulReveal(RevealResult::GameOver { winner }).encode()
    )));
}

pub fn failure_user_reveal(program: &Program, player: u64, users_move: Move) {
    let result = try_to_reveal(program, player, users_move);

    assert!(result.main_failed());
}

pub fn failure_user_reveal_with_password(
    program: &Program,
    player: u64,
    users_move: Move,
    password: &str,
) {
    let result = try_to_reveal_with_password(program, player, users_move, password);

    assert!(result.main_failed());
}

fn try_to_reveal(program: &Program, player: u64, users_move: Move) -> RunResult {
    try_to_reveal_with_password(program, player, users_move, DEFAULT_PASSWORD)
}

fn try_to_reveal_with_password(
    program: &Program,
    player: u64,
    users_move: Move,
    password: &str,
) -> RunResult {
    let move_with_pass = users_move.number().to_string() + password;

    program.send(player, Action::Reveal(move_with_pass))
}

pub fn check_remove_player(program: &Program, from: u64, removing_player: u64) {
    let result = program.send(from, Action::RemovePlayerFromLobby(removing_player.into()));

    assert!(result.contains(&(
        from,
        Event::PlayerWasRemoved(removing_player.into()).encode()
    )));
}

pub fn failure_remove_player(program: &Program, from: u64, removing_player: u64) {
    let result = program.send(from, Action::RemovePlayerFromLobby(removing_player.into()));

    assert!(result.main_failed());
}

pub fn check_change_lobby(program: &Program, from: u64, players: &[u64]) {
    let result = program.send(
        from,
        Action::SetLobbyPlayersList(players.iter().cloned().map(|x| x.into()).collect()),
    );

    assert!(result.contains(&(from, Event::LobbyPlayersListUpdated.encode())));
}

pub fn failure_change_lobby(program: &Program, from: u64, players: &[u64]) {
    let result = program.send(
        from,
        Action::SetLobbyPlayersList(players.iter().cloned().map(|x| x.into()).collect()),
    );

    assert!(result.main_failed());
}

pub fn check_add_player(program: &Program, from: u64, adding_player: u64) {
    let result = program.send(from, Action::AddPlayerInLobby(adding_player.into()));

    assert!(result.contains(&(from, Event::PlayerWasAdded(adding_player.into()).encode())));
}

pub fn failure_add_player(program: &Program, from: u64, adding_player: u64) {
    let result = program.send(from, Action::AddPlayerInLobby(adding_player.into()));

    assert!(result.main_failed());
}

pub fn check_stop_the_game(program: &Program, from: u64, rewarded_users: &[u64]) {
    let result = program.send(from, Action::StopGame);
    let rewarded_users = rewarded_users.iter().cloned().map(Into::into).collect();
    assert!(result.contains(&(from, Event::GameWasStopped(rewarded_users).encode())));
}

pub fn failure_stop_the_game(program: &Program, from: u64) {
    let result = program.send(from, Action::StopGame);

    assert!(result.main_failed());
}
