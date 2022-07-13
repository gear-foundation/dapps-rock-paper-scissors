#![no_std]

use gstd::{debug, exec, msg, prelude::*, ActorId};
use io::*;

static mut RPS_GAME: Option<RPSGame> = None;

const MILLISEC_IN_SEC: u64 = 1000;
const MIN_TIMEOUT: u64 = 5 * MILLISEC_IN_SEC;
const MIN_PLAYERS_COUNT: u8 = 2;

#[derive(Debug, Default)]
struct RPSGame {
    owner: ActorId,
    lobby: BTreeSet<ActorId>,
    game_config: GameConfig,
    stage: GameStage,
    encrypted_moves: BTreeMap<ActorId, String>,
    player_moves: BTreeMap<ActorId, Move>,
    next_game_config: Option<GameConfig>,
    current_stage_start_timestamp: u64,
}

impl RPSGame {
    fn validate_game_config(config: &GameConfig) {
        if config.players_count_limit < MIN_PLAYERS_COUNT {
            panic!("Players count limit is too low")
        }

        if config.entry_timeout < MIN_TIMEOUT {
            panic!("Entry timeout is too low")
        }

        if config.move_timeout < MIN_TIMEOUT {
            panic!("Move timeout is too low")
        }

        if config.reveal_timeout < MIN_TIMEOUT {
            panic!("Reveal timeout is too low")
        }
    }

    fn validate_there_is_place_for_player(&self) {
        if self.lobby.len() + 1 > self.game_config.players_count_limit as usize {
            panic!("There are enough players")
        }
    }

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

    fn validate_bet(&self, value: u128) {
        if self.game_config.bet_size > value {
            panic!("Not enough money for bet")
        }
    }

    fn validate_player_can_make_a_move(&self, player: &ActorId) {
        match &self.stage {
            GameStage::InProgress(description) => {
                if !description.anticipated_players.contains(player) {
                    panic!("There is no such player in game right now, may be he got out of the game or he is not in the lobby")
                }
            }
            GameStage::Reveal(_) | GameStage::Preparation => {
                panic!(
                    "It's not time to make a move, {:?}, {:?}, {:?}",
                    self.stage,
                    exec::block_timestamp(),
                    self.current_stage_start_timestamp,
                );
            }
        };
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
            .encrypted_moves
            .get(player)
            .expect("Can't find a move of this player");

        let hash_bytes = sp_core_hashing::blake2_256(real_move.as_bytes());
        let hex_hash = Self::to_hex_string(hash_bytes);

        if &hex_hash != saved_move {
            panic!("Player tries to cheat")
        }
    }

    fn register(&mut self) {
        self.validate_game_is_not_in_progress();
        self.validate_bet(msg::value());
        self.validate_there_is_no_such_player(&msg::source());
        self.validate_there_is_place_for_player();

        let change = msg::value() - self.game_config.bet_size;
        self.lobby.insert(msg::source());

        msg::reply(Event::PlayerRegistred, change).expect("Can't send reply");
    }

    fn make_move(&mut self, move_hash: String) {
        let player_id = &msg::source();
        self.validate_player_can_make_a_move(player_id);

        self.save_move(&msg::source(), move_hash);
        self.try_to_transit_to_reveal_stage_after_move();

        msg::reply(Event::SuccessfulMove(*player_id), 0).expect("Reply error");
    }

    fn reveal(&mut self, real_move: &str) {
        let player = &msg::source();

        self.validate_player_can_reveal(player);
        self.validate_reveal(player, real_move);

        self.save_real_move(player, real_move);
        let result = self.end_round_if_needed();

        msg::reply(Event::SuccessfulReveal(result), 0).expect("Reply error");
    }

    fn set_next_game_config(&mut self, config: GameConfig) {
        Self::validate_game_config(&config);
        self.validate_source_is_owner();

        self.next_game_config = Some(config);

        msg::reply(Event::GameConfigChanged, 0).expect("Reply error");
    }

    fn stop_the_game(&mut self) {
        self.validate_source_is_owner();
        self.validate_game_is_in_progress();

        let players = if matches!(self.stage, GameStage::Preparation) {
            self.lobby.iter().for_each(|player| {
                msg::send(*player, "", self.game_config.bet_size).expect("Can't send reward");
            });

            self.lobby.clone()
        } else {
            let players = self.stage.current_players().expect("Game is not started");

            let part = exec::value_available() / players.len() as u128;

            for player in players.iter() {
                msg::send(*player, "", part).expect("Can't send reward");
            }

            players
        };

        msg::reply(Event::GameWasStopped(players), 0).expect("Reply error");

        self.start_new_game();
    }

    fn change_stage_by_timeout_if_needed(&mut self) {
        let end_time = self.current_stage_start_timestamp
            + match self.stage {
                GameStage::Preparation => self.game_config.entry_timeout,
                GameStage::InProgress(_) => self.game_config.move_timeout,
                GameStage::Reveal(_) => self.game_config.reveal_timeout,
            };

        if end_time < exec::block_timestamp() {
            match self.stage.clone() {
                GameStage::Preparation => self.handle_preparation_timout(),
                GameStage::InProgress(desctription) => self.handle_moves_timout(&desctription),
                GameStage::Reveal(desctription) => self.handle_reveal_timout(&desctription),
            }
        }
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

        self.stage = GameStage::InProgress(progress_description);
        self.update_timestamp();
    }

    fn try_to_transit_to_reveal_stage_after_move(&mut self) {
        if let GameStage::InProgress(description) = self.stage.clone() {
            if description.anticipated_players.is_empty() {
                self.transit_to_reveal_stage(&description);
            }
        }
    }

    fn handle_preparation_timout(&mut self) {
        match self.lobby.len() {
            0 | 1 => self.update_timestamp(),
            _ => self.transit_to_in_progress_stage_from_preparation(),
        }
    }

    fn handle_moves_timout(&mut self, progress_description: &StageDescription) {
        match progress_description.finished_players.len() {
            0 => self.update_timestamp(),
            1 => {
                let winner = progress_description
                    .finished_players
                    .clone()
                    .into_iter()
                    .last()
                    .expect("Unknown winner");
                msg::send(winner, "", exec::value_available()).expect("Can't send reward");
                self.start_new_game();
            }
            _ => self.transit_to_reveal_stage(progress_description),
        }
    }

    fn handle_reveal_timout(&mut self, progress_description: &StageDescription) {
        match progress_description.finished_players.len() {
            0 => self.update_timestamp(),
            _ => {
                self.end_round();
            }
        }
    }

    fn transit_to_reveal_stage(&mut self, progress_description: &StageDescription) {
        self.stage = GameStage::Reveal(StageDescription {
            anticipated_players: progress_description.finished_players.clone(),
            finished_players: Default::default(),
        });
        self.update_timestamp();
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
        let set_of_moves = BTreeSet::from_iter(self.player_moves.values().cloned());
        let next_round_players: BTreeSet<ActorId> = match set_of_moves.len() {
            1 | 4 | 5 => self.player_moves.keys().cloned().collect(),
            2 | 3 => {
                let winners = self.next_round_moves_set(set_of_moves);
                self.player_moves
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
            self.update_timestamp();
            self.clear_moves();

            RevealResult::NextRoundStarted {
                players: next_round_players,
            }
        } else {
            let winner = next_round_players
                .into_iter()
                .last()
                .expect("Unknown winner");
            msg::send(winner, "", exec::value_available()).expect("Can't send reward");
            self.start_new_game();

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
            self.encrypted_moves.insert(*player, move_hash);

            progress_description.anticipated_players.remove(player);
            progress_description.finished_players.insert(*player);
        }
    }

    fn save_real_move(&mut self, player: &ActorId, real_move: &str) {
        let users_move = real_move.chars().next().expect("Move is empty");
        let users_move = Move::new(users_move);

        self.player_moves.insert(*player, users_move);

        match &mut self.stage {
            GameStage::Preparation | GameStage::InProgress(_) => {}
            GameStage::Reveal(description) => {
                description.anticipated_players.remove(player);
                description.finished_players.insert(*player);
            }
        }
    }

    fn clear_moves(&mut self) {
        self.encrypted_moves.clear();
        self.player_moves.clear();
    }

    fn start_new_game(&mut self) {
        self.clear_for_new_game();
        self.stage = GameStage::Preparation;
        self.update_timestamp();
    }

    fn clear_for_new_game(&mut self) {
        self.clear_moves();
        self.lobby.clear();
        if let Some(config) = self.next_game_config.take() {
            self.game_config = config;
        }
    }

    fn update_timestamp(&mut self) {
        self.current_stage_start_timestamp = exec::block_timestamp();
    }
}

#[no_mangle]
unsafe extern "C" fn init() {
    let config: GameConfig = msg::load().expect("Could not load Action");
    debug!("init(): {:?}", config);

    RPSGame::validate_game_config(&config);

    let game = RPSGame {
        owner: msg::source(),
        game_config: config,
        current_stage_start_timestamp: exec::block_timestamp(),
        ..RPSGame::default()
    };

    RPS_GAME = Some(game);
}

#[no_mangle]
unsafe extern "C" fn handle() {
    let action: Action = msg::load().expect("Could not load Action");
    let game: &mut RPSGame = RPS_GAME.get_or_insert(RPSGame::default());

    game.change_stage_by_timeout_if_needed();

    match action {
        Action::Register => game.register(),
        Action::MakeMove(hashed_move) => game.make_move(hashed_move),
        Action::Reveal(real_move) => game.reveal(real_move.as_str()),
        Action::ChangeNextGameConfig(config) => game.set_next_game_config(config),
        Action::StopGame => game.stop_the_game(),
    }
}

#[no_mangle]
unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: State = msg::load().expect("failed to decode input argument");
    let game: &RPSGame = RPS_GAME.get_or_insert(RPSGame::default());

    let encoded = match query {
        State::Config => StateReply::Config(game.game_config.clone()),
        State::LobbyList => StateReply::LobbyList(game.lobby.clone().into_iter().collect()),
        State::GameState => StateReply::GameStage(game.stage.clone()),
    }
    .encode();

    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "RockPaperScissors",
    init:
        input : GameConfig,
    handle:
        input: Action,
        output: Event,
    state:
        input: State,
        output: StateReply,
}
