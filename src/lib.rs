#![no_std]

use gstd::{debug, exec, msg, prelude::*, ActorId};
use io::*;

static mut RPS_GAME: Option<RPSGame> = None;

#[derive(Debug, Default)]
struct RPSGame {
    owner: ActorId,
    lobby: BTreeSet<ActorId>,
    bet_size: u128,
    stage: GameStage,
    moves: BTreeMap<ActorId, String>,
    player_real_moves: BTreeMap<ActorId, Move>,
    bettors: BTreeSet<ActorId>,
}

impl RPSGame {
    fn validate_source_is_owner(&self) {
        if msg::source() != self.owner {
            panic!("Caller is not an owner")
        }
    }

    fn validate_there_is_no_such_player(&self, player: &ActorId) {
        if self.lobby.contains(player) {
            panic!("This player is already in lobby")
        }
    }

    fn validate_there_is_such_player(&self, player: &ActorId) {
        if !self.lobby.contains(player) {
            panic!("This player is not in lobby")
        }
    }

    fn validate_game_is_not_in_progress(&self) {
        if self.stage.game_is_in_progress() {
            panic!("Game is in progress")
        }
    }

    fn validate_game_is_in_progress(&self) {
        if !self.stage.game_is_in_progress() {
            panic!("Game is not in progress")
        }
    }

    fn validate_bet(&self, player: &ActorId, value: u128) {
        if !self.bettors.contains(player) && self.bet_size > value {
            panic!("Not enough money for bet")
        }
    }

    fn validate_player_can_make_a_move(&self, player: &ActorId) {
        let can_make_a_move = match &self.stage {
            GameStage::Preparation => self.lobby.contains(player),
            GameStage::InProgress(description) => description.anticipated_players.contains(player),
            GameStage::Reveal(_) => false,
        };

        if !can_make_a_move {
            panic!("There is no such player in game right now, may be he got out of the game or he is not in the lobby")
        }
    }

    fn validate_player_can_reveal(&self, player: &ActorId) {
        match &self.stage {
            GameStage::Preparation | GameStage::InProgress(_) => panic!("It's not reveal stage!"),
            GameStage::Reveal(description) => {
                if !description.anticipated_players.contains(player) {
                    if description.finished_players.contains(player) {
                        panic!("Player has already revealed")
                    } else {
                        panic!("There is no such player at the reveal stage")
                    }
                }
            }
        };
    }

    fn validate_reveal(&self, player: &ActorId, real_move: &str) {
        let saved_move = self
            .moves
            .get(player)
            .expect("Can't find a move of this player");

        let hash_bytes = sp_core_hashing::blake2_256(real_move.as_bytes());
        let hex_hash = Self::to_hex_string(hash_bytes);

        if &hex_hash != saved_move {
            panic!("Player tries to cheat")
        }
    }

    fn add_player_in_lobby(&mut self, player: &ActorId) {
        self.validate_source_is_owner();
        self.validate_game_is_not_in_progress();
        self.validate_there_is_no_such_player(player);

        self.lobby.insert(*player);

        msg::reply(Event::PlayerWasAdded(*player), 0).expect("Can't send reply");
    }

    fn remove_player_in_lobby(&mut self, player: &ActorId) {
        self.validate_source_is_owner();
        self.validate_there_is_such_player(player);
        self.validate_game_is_not_in_progress();

        self.lobby.remove(player);
        msg::reply(Event::PlayerWasRemoved(*player), 0).expect("Can't send reply");
    }

    fn set_lobby_players_list(&mut self, new_list: Vec<ActorId>) {
        self.validate_source_is_owner();
        self.validate_game_is_not_in_progress();

        self.lobby = BTreeSet::from_iter(new_list.into_iter());

        msg::reply(Event::LobbyPlayersListUpdated, 0).expect("Can't send reply");
    }

    fn set_bet_size(&mut self, new_size: u128) {
        self.validate_source_is_owner();
        self.validate_game_is_not_in_progress();

        self.bet_size = new_size;

        msg::reply(Event::BetSizeWasChanged(new_size), 0).expect("Can't send reply");
    }

    fn make_move(&mut self, move_hash: String) {
        let player_id = &msg::source();
        self.validate_player_can_make_a_move(player_id);
        self.validate_bet(player_id, msg::value());

        self.clear_history_if_needed();

        match self.stage {
            GameStage::Preparation => self.transit_to_in_progress_stage_from_preparation(),
            GameStage::InProgress(_) => {}
            GameStage::Reveal(_) => panic!("It's reveal time"),
        }

        self.save_move(&msg::source(), move_hash);
        self.transit_to_reveal_stage_if_needed();

        let change = self.place_bet_if_needed(player_id, msg::value());

        msg::reply(Event::SuccessfulMove(*player_id), change).expect("Reply error");
    }

    fn reveal(&mut self, real_move: &str) {
        let player = &msg::source();

        self.validate_player_can_reveal(player);
        self.validate_reveal(player, real_move);

        self.save_real_move(player, real_move);
        let result = self.end_round_if_needed();

        msg::reply(Event::SuccessfulReveal(result), 0).expect("Reply error");
    }

    fn stop_the_game(&mut self) {
        self.validate_source_is_owner();
        self.validate_game_is_in_progress();

        let players = if self.bettors.len() < self.lobby.len() {
            self.bettors.iter().for_each(|player| {
                msg::send(*player, "", self.bet_size).expect("Can't send reward");
            });

            self.bettors.clone()
        } else {
            let players = self.stage.current_players().expect("game is not started");

            let part = exec::value_available() / players.len() as u128;

            for player in players.iter() {
                msg::send(*player, "", part).expect("Can't send reward");
            }

            players
        };

        self.stage = GameStage::Preparation;

        msg::reply(Event::GameWasStopped(players), 0).expect("Reply error");
    }

    fn to_hex_string(bytes: [u8; 32]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    fn transit_to_in_progress_stage_from_preparation(&mut self) {
        let progress_description = StageDescription {
            anticipated_players: self.lobby.clone(),
            finished_players: Default::default(),
        };

        self.stage = GameStage::InProgress(progress_description)
    }

    fn transit_to_reveal_stage_if_needed(&mut self) {
        if let GameStage::InProgress(description) = &self.stage {
            if description.anticipated_players.is_empty() {
                self.stage = GameStage::Reveal(StageDescription {
                    anticipated_players: description.finished_players.clone(),
                    finished_players: Default::default(),
                })
            }
        }
    }

    fn end_round_if_needed(&mut self) -> RevealResult {
        if let GameStage::Reveal(reveal_description) = &self.stage {
            if reveal_description.anticipated_players.is_empty() {
                self.end_round()
            } else {
                RevealResult::Continue
            }
        } else {
            panic!("It's not reveal stage")
        }
    }

    fn end_round(&mut self) -> RevealResult {
        let set_of_moves = BTreeSet::from_iter(self.player_real_moves.values().cloned());
        let next_round_players: BTreeSet<ActorId> = match set_of_moves.len() {
            1 | 4 | 5 => self.player_real_moves.keys().cloned().collect(),
            2 | 3 => {
                let winners = self.next_round_moves_set(set_of_moves);
                self.player_real_moves
                    .iter()
                    .filter(|(_, users_move)| winners.contains(users_move))
                    .map(|(player, _)| player)
                    .copied()
                    .collect()
            }
            _ => panic!("Unknown result"),
        };

        if next_round_players.len() > 1 {
            self.stage = GameStage::InProgress(StageDescription {
                anticipated_players: next_round_players.clone(),
                finished_players: BTreeSet::new(),
            });
            RevealResult::NextRoundStarted {
                players: next_round_players,
            }
        } else {
            let winner = next_round_players
                .into_iter()
                .last()
                .expect("Unknown winner");
            msg::send(winner, "", exec::value_available()).expect("Can't send reward");
            self.stage = GameStage::Preparation;
            RevealResult::GameOver { winner }
        }
    }

    fn next_round_moves_set(&self, set_of_moves: BTreeSet<Move>) -> BTreeSet<Move> {
        let mut wins_loses_map = BTreeMap::from_iter(
            set_of_moves
                .iter()
                .cloned()
                .map(|users_move| (users_move, (0, 0))),
        );

        let mut iterator = set_of_moves.iter();

        while let Some(a_move) = iterator.next() {
            let cloned_iterator = iterator.clone();

            for b_move in cloned_iterator {
                if a_move.wins(b_move) {
                    wins_loses_map.get_mut(a_move).unwrap().0 += 1;
                    wins_loses_map.get_mut(b_move).unwrap().1 += 1;
                } else {
                    wins_loses_map.get_mut(a_move).unwrap().1 += 1;
                    wins_loses_map.get_mut(b_move).unwrap().0 += 1;
                }
            }
        }

        let (only_wins, only_loses) = wins_loses_map.into_iter().fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut only_wins, mut only_loses), (users_move, (wins, loses))| {
                if loses == 0 {
                    only_wins.insert(users_move);
                } else if wins == 0 {
                    only_loses.insert(users_move);
                };

                (only_wins, only_loses)
            },
        );

        if !only_wins.is_empty() {
            only_wins
        } else if !only_loses.is_empty() {
            set_of_moves.difference(&only_loses).cloned().collect()
        } else {
            set_of_moves
        }
    }

    fn save_move(&mut self, player: &ActorId, move_hash: String) {
        if let GameStage::InProgress(progress_description) = &mut self.stage {
            self.moves.insert(*player, move_hash);

            progress_description.anticipated_players.remove(player);
            progress_description.finished_players.insert(*player);
        }
    }

    fn save_real_move(&mut self, player: &ActorId, real_move: &str) {
        let users_move = real_move.chars().next().expect("Move is empty");
        let users_move = Move::new(users_move);

        self.player_real_moves.insert(*player, users_move);

        match &mut self.stage {
            GameStage::Preparation | GameStage::InProgress(_) => {}
            GameStage::Reveal(description) => {
                description.anticipated_players.remove(player);
                description.finished_players.insert(*player);
            }
        }
    }

    fn place_bet_if_needed(&mut self, player_id: &ActorId, bet: u128) -> u128 {
        if self.bettors.contains(player_id) {
            bet
        } else {
            self.bettors.insert(*player_id);
            bet.checked_sub(self.bet_size).expect("Bet is too small")
        }
    }

    fn clear_history_if_needed(&mut self) {
        let mut clear_moves = || {
            self.moves.clear();
            self.player_real_moves.clear();
        };

        match self.stage.clone() {
            GameStage::Preparation => {
                clear_moves();
                self.bettors.clear();
            }
            GameStage::InProgress(description) => {
                if description.finished_players.is_empty() {
                    clear_moves();
                }
            }
            GameStage::Reveal(_) => {}
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn init() {
    let config: InitConfig = msg::load().expect("Could not load Action");

    debug!("init(): {:?}", config);

    let game = RPSGame {
        owner: msg::source(),
        lobby: BTreeSet::from_iter(config.lobby_players.into_iter()),
        bet_size: config.bet_size,
        ..RPSGame::default()
    };

    RPS_GAME = Some(game);
}

#[no_mangle]
pub unsafe extern "C" fn handle() {
    let action: Action = msg::load().expect("Could not load Action");
    let game: &mut RPSGame = RPS_GAME.get_or_insert(RPSGame::default());

    match action {
        Action::AddPlayerInLobby(player) => game.add_player_in_lobby(&player),
        Action::RemovePlayerFromLobby(player) => game.remove_player_in_lobby(&player),
        Action::SetLobbyPlayersList(players_list) => game.set_lobby_players_list(players_list),
        Action::SetBetSize(bet_size) => game.set_bet_size(bet_size),
        Action::MakeMove(hashed_move) => game.make_move(hashed_move),
        Action::Reveal(real_move) => game.reveal(real_move.as_str()),
        Action::StopGame => game.stop_the_game(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: State = msg::load().expect("failed to decode input argument");
    let game: &RPSGame = RPS_GAME.get_or_insert(RPSGame::default());

    let encoded = match query {
        State::BetSize => StateReply::BetSize(game.bet_size),
        State::LobbyList => StateReply::LobbyList(game.lobby.clone().into_iter().collect()),
        State::GameState => StateReply::GameStage(game.stage.clone()),
    }
    .encode();

    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "RockPaperScissors",
    init:
        input : InitConfig,
    handle:
        input: Action,
        output: Event,
    state:
        input: State,
        output: StateReply,
}
