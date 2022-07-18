use crate::RPSGame;
use gstd::{exec, msg, ActorId};
use io::*;

const MILLISEC_IN_SEC: u64 = 1000;
const MIN_TIMEOUT: u64 = 5 * MILLISEC_IN_SEC;
const MIN_PLAYERS_COUNT: u8 = 2;

impl RPSGame {
    pub(crate) fn validate_there_is_place_for_player(&self) {
        if self.lobby.len() + 1 > self.game_config.players_count_limit as usize {
            panic!("There are enough players")
        }
    }

    pub(crate) fn validate_source_is_owner(&self) {
        if msg::source() != self.owner {
            panic!("Caller is not an owner")
        }
    }

    pub(crate) fn validate_there_is_no_such_player(&self, player: &ActorId) {
        if self.lobby.contains(player) {
            panic!("This player is already in lobby")
        }
    }

    pub(crate) fn validate_game_is_not_in_progress(&self) {
        if self.stage.game_is_in_progress() {
            panic!("Game is in progress")
        }
    }

    pub(crate) fn validate_game_is_in_progress(&self) {
        if !self.stage.game_is_in_progress() {
            panic!("Game is not in progress")
        }
    }

    pub(crate) fn validate_bet(&self, value: u128) {
        if self.game_config.bet_size > value {
            panic!("Not enough money for bet")
        }
    }

    pub(crate) fn validate_player_can_make_a_move(&self, player: &ActorId) {
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

    pub(crate) fn validate_player_can_reveal(&self, player: &ActorId) {
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

    pub(crate) fn validate_reveal(&self, player: &ActorId, real_move: &str) {
        let saved_move = self
            .encrypted_moves
            .get(player)
            .expect("Can't find a move of this player");

        let hash_bytes = sp_core_hashing::blake2_256(real_move.as_bytes());
        let hex_hash = Self::bytes_to_hex_string(hash_bytes);

        if &hex_hash != saved_move {
            panic!("Player tries to cheat")
        }
    }
}

pub(crate) fn validate_game_config(config: &GameConfig) {
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
