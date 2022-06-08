#![no_std]

extern crate alloc;

use crate::Answer::{Lizard, Paper, Rock, Scissors, Spock};
use gstd::{debug, exec, msg, prelude::*, ActorId};
use io::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Answer {
    Rock,
    Paper,
    Scissors,
    Lizard,
    Spock,
}

impl Answer {
    fn new(number: char) -> Answer {
        match number {
            '0' => Rock,
            '1' => Paper,
            '2' => Scissors,
            '3' => Lizard,
            '4' => Spock,
            _ => panic!("Unknown symbol in answer"),
        }
    }

    fn wins(&self, other: &Answer) -> bool {
        match self {
            Rock => match other {
                Rock | Paper | Spock => false,
                Scissors | Lizard => true,
            },
            Paper => match other {
                Paper | Scissors | Lizard => false,
                Rock | Spock => true,
            },
            Scissors => match other {
                Rock | Scissors | Spock => false,
                Paper | Lizard => true,
            },
            Lizard => match other {
                Rock | Scissors | Lizard => false,
                Paper | Spock => true,
            },
            Spock => match other {
                Paper | Lizard | Spock => false,
                Rock | Scissors => true,
            },
        }
    }
}

static mut RPS_GAME: Option<RPSGame> = None;

#[derive(Debug, Default)]
struct RPSGame {
    owner: ActorId,
    lobby: BTreeSet<ActorId>,
    bet_size: u128,
    stage: GameStage,
    moves: BTreeMap<ActorId, String>,
    player_throws: BTreeMap<ActorId, Answer>,
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

    fn validate_is_not_playing_right_now(&self, player: &ActorId) {
        if self.stage.is_player_in_game(player) {
            panic!("This player is in game right now")
        }
    }

    fn validate_game_is_not_in_progress(&self) {
        if self.stage.game_is_in_progress() {
            panic!("Game is in progress")
        }
    }

    fn validate_enough_value(&self, value: u128) {
        if self.bet_size > value {
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

    fn validate_throw(&self, player: &ActorId, real_move: &str) {
        let saved_move = self
            .moves
            .get(player)
            .expect("Can't find a move of this player");

        let hash_bytes = sp_core_hashing::blake2_256(real_move.as_bytes());
        let hash = String::from_utf8(hash_bytes.to_vec()).unwrap();

        if &hash != saved_move {
            panic!("Player tries to cheat")
        }
    }

    fn add_player_in_lobby(&mut self, player: &ActorId) {
        self.validate_source_is_owner();
        self.validate_there_is_no_such_player(player);

        self.lobby.insert(*player);

        msg::reply(Event::PlayerWasAdded(*player), 0).unwrap();
    }

    fn remove_player_in_lobby(&mut self, player: &ActorId) {
        self.validate_source_is_owner();
        self.validate_there_is_such_player(player);
        self.validate_is_not_playing_right_now(player);

        self.lobby.remove(player);
        msg::reply(Event::PlayerWasRemoved(*player), 0).unwrap();
    }

    fn set_lobby_players_list(&mut self, new_list: Vec<ActorId>) {
        self.validate_source_is_owner();
        self.validate_game_is_not_in_progress();

        self.lobby = BTreeSet::from_iter(new_list.into_iter())
    }

    fn set_bet_size(&mut self, new_size: u128) {
        self.validate_source_is_owner();
        self.validate_game_is_not_in_progress();

        self.bet_size = new_size;
    }

    fn make_move(&mut self, move_hash: String) {
        self.validate_player_can_make_a_move(&msg::source());
        self.validate_enough_value(msg::value());

        match self.stage {
            GameStage::Preparation => self.transit_to_in_progress_stage_from_preparation(),
            GameStage::InProgress(_) => {}
            GameStage::Reveal(_) => panic!("It's reveal time"),
        }

        self.save_move(&msg::source(), move_hash);
        self.transit_to_reveal_stage_if_needed()
    }

    fn reveal(&mut self, real_move: &str) {
        let player = &msg::source();

        self.validate_player_can_reveal(player);
        self.validate_throw(player, real_move);

        self.save_throw(player, real_move);
        self.end_round_if_needed();
    }

    fn stop_the_game(&mut self) {
        self.validate_source_is_owner();

        let players = self
            .stage
            .current_players()
            .unwrap_or_else(|| self.lobby.iter().collect());
        let part = exec::value_available() / players.len() as u128;

        for player in players {
            msg::send(*player, "", part).unwrap();
        }
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

    fn end_round_if_needed(&mut self) {
        if let GameStage::Reveal(reveal_description) = &self.stage {
            if reveal_description.anticipated_players.is_empty() {
                self.end_round()
            }
        }
    }

    fn end_round(&mut self) {
        let set_of_answers = BTreeSet::from_iter(self.player_throws.values().cloned());
        let next_round_players: BTreeSet<ActorId> = match set_of_answers.len() {
            1 | 4 | 5 => self.player_throws.keys().cloned().collect(),
            2 | 3 => {
                let winners = self.next_round_answers_set(set_of_answers);
                self.player_throws
                    .iter()
                    .filter(|(_, answer)| winners.contains(answer))
                    .map(|(player, _)| player)
                    .copied()
                    .collect()
            }
            _ => panic!("Unknown result"),
        };

        if next_round_players.len() > 1 {
            self.stage = GameStage::InProgress(StageDescription {
                anticipated_players: next_round_players,
                finished_players: BTreeSet::new(),
            })
        } else {
            let winner = next_round_players.into_iter().last().unwrap();
            msg::send(winner, "", exec::value_available()).unwrap();
            self.stage = GameStage::Preparation
        }
    }

    fn next_round_answers_set(&self, set_of_answers: BTreeSet<Answer>) -> BTreeSet<Answer> {
        let mut wins_loses_map = BTreeMap::from_iter(
            set_of_answers
                .iter()
                .cloned()
                .map(|answer| (answer, (0, 0))),
        );

        let mut iterator = set_of_answers.iter();

        while let Some(a_answer) = iterator.next() {
            let cloned_iterator = iterator.clone();

            for b_answer in cloned_iterator {
                if a_answer.wins(b_answer) {
                    wins_loses_map.get_mut(a_answer).unwrap().0 += 1;
                    wins_loses_map.get_mut(b_answer).unwrap().1 += 1;
                } else {
                    wins_loses_map.get_mut(a_answer).unwrap().1 += 1;
                    wins_loses_map.get_mut(b_answer).unwrap().0 += 1;
                }
            }
        }

        let (only_wins, only_loses) = wins_loses_map.into_iter().fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut only_wins, mut only_loses), (answer, (wins, loses))| {
                if loses == 0 {
                    only_wins.insert(answer);
                } else if wins == 0 {
                    only_loses.insert(answer);
                };

                (only_wins, only_loses)
            },
        );

        if !only_wins.is_empty() {
            only_wins
        } else if !only_loses.is_empty() {
            set_of_answers.difference(&only_loses).cloned().collect()
        } else {
            set_of_answers
        }
    }

    fn save_move(&mut self, player: &ActorId, move_hash: String) {
        if let GameStage::InProgress(progress_description) = &mut self.stage {
            self.moves.insert(*player, move_hash);

            progress_description.anticipated_players.remove(player);
            progress_description.finished_players.insert(*player);
        }
    }

    fn save_throw(&mut self, player: &ActorId, real_move: &str) {
        let char_answer = real_move.chars().next().expect("Answer is empty");
        let answer = Answer::new(char_answer);

        self.player_throws.insert(*player, answer);

        match &mut self.stage {
            GameStage::Preparation | GameStage::InProgress(_) => {}
            GameStage::Reveal(description) => {
                description.anticipated_players.remove(player);
                description.finished_players.insert(*player);
            }
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
        Action::Reveal(throw) => game.reveal(throw.as_str()),
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
