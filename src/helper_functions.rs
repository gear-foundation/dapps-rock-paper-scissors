use crate::RPSGame;
use gstd::{exec, msg, prelude::*, ActorId};
use io::*;

impl RPSGame {
    pub(crate) fn change_stage_by_timeout_if_needed(&mut self) {
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

    pub(crate) fn bytes_to_hex_string(bytes: [u8; 32]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    pub(crate) fn transit_to_in_progress_stage_from_preparation(&mut self) {
        let progress_description = StageDescription {
            anticipated_players: self.lobby.clone(),
            finished_players: Default::default(),
        };

        self.stage = GameStage::InProgress(progress_description);
        self.update_timestamp();
    }

    pub(crate) fn try_to_transit_to_reveal_stage_after_move(&mut self) {
        if let GameStage::InProgress(description) = self.stage.clone() {
            if description.anticipated_players.is_empty() {
                self.transit_to_reveal_stage(&description);
            }
        }
    }

    pub(crate) fn handle_preparation_timout(&mut self) {
        match self.lobby.len() {
            0 | 1 => self.update_timestamp(),
            _ => self.transit_to_in_progress_stage_from_preparation(),
        }
    }

    pub(crate) fn handle_moves_timout(&mut self, progress_description: &StageDescription) {
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

    pub(crate) fn handle_reveal_timout(&mut self, progress_description: &StageDescription) {
        match progress_description.finished_players.len() {
            0 => self.update_timestamp(),
            _ => {
                self.end_round();
            }
        }
    }

    pub(crate) fn transit_to_reveal_stage(&mut self, progress_description: &StageDescription) {
        self.stage = GameStage::Reveal(StageDescription {
            anticipated_players: progress_description.finished_players.clone(),
            finished_players: Default::default(),
        });
        self.update_timestamp();
    }

    pub(crate) fn end_round_if_needed(&mut self) -> RevealResult {
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

    pub(crate) fn end_round(&mut self) -> RevealResult {
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
            msg::send(winner, "WIN", exec::value_available()).expect("Can't send reward");
            self.start_new_game();

            RevealResult::GameOver { winner }
        }
    }

    pub(crate) fn next_round_moves_set(&self, set_of_moves: BTreeSet<Move>) -> BTreeSet<Move> {
        'outer: for a_move in set_of_moves.iter() {
            for b_move in set_of_moves.iter() {
                if a_move != b_move && !a_move.wins(b_move) {
                    continue 'outer;
                }
            }

            return BTreeSet::from([a_move.clone()]);
        }

        set_of_moves
    }

    pub(crate) fn save_move(&mut self, player: &ActorId, move_hash: String) {
        if let GameStage::InProgress(progress_description) = &mut self.stage {
            self.encrypted_moves.insert(*player, move_hash);

            progress_description.anticipated_players.remove(player);
            progress_description.finished_players.insert(*player);
        }
    }

    pub(crate) fn save_real_move(&mut self, player: &ActorId, real_move: &str) {
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

    pub(crate) fn clear_moves(&mut self) {
        self.encrypted_moves.clear();
        self.player_moves.clear();
    }

    pub(crate) fn start_new_game(&mut self) {
        self.clear_for_new_game();
        self.stage = GameStage::Preparation;
        self.update_timestamp();
    }

    pub(crate) fn clear_for_new_game(&mut self) {
        self.clear_moves();
        self.lobby.clear();
        if let Some(config) = self.next_game_config.take() {
            self.game_config = config;
        }
    }

    pub(crate) fn update_timestamp(&mut self) {
        self.current_stage_start_timestamp = exec::block_timestamp();
    }
}
