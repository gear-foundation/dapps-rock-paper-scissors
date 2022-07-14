#![no_std]

use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
    Lizard,
    Spock,
}

impl Move {
    pub fn number(&self) -> char {
        match self {
            Move::Rock => '0',
            Move::Paper => '1',
            Move::Scissors => '2',
            Move::Lizard => '3',
            Move::Spock => '4',
        }
    }

    pub fn new(number: char) -> Move {
        match number {
            '0' => Move::Rock,
            '1' => Move::Paper,
            '2' => Move::Scissors,
            '3' => Move::Lizard,
            '4' => Move::Spock,
            _ => panic!("Unknown symbol in move"),
        }
    }

    pub fn wins(&self, other: &Move) -> bool {
        match self {
            Move::Rock => match other {
                Move::Rock | Move::Paper | Move::Spock => false,
                Move::Scissors | Move::Lizard => true,
            },
            Move::Paper => match other {
                Move::Paper | Move::Scissors | Move::Lizard => false,
                Move::Rock | Move::Spock => true,
            },
            Move::Scissors => match other {
                Move::Rock | Move::Scissors | Move::Spock => false,
                Move::Paper | Move::Lizard => true,
            },
            Move::Lizard => match other {
                Move::Rock | Move::Scissors | Move::Lizard => false,
                Move::Paper | Move::Spock => true,
            },
            Move::Spock => match other {
                Move::Paper | Move::Lizard | Move::Spock => false,
                Move::Rock | Move::Scissors => true,
            },
        }
    }
}

#[derive(Debug, Default, Encode, Decode, TypeInfo, Clone)]
pub struct StageDescription {
    pub anticipated_players: BTreeSet<ActorId>,
    pub finished_players: BTreeSet<ActorId>,
}

#[derive(Debug, Default, Encode, Decode, TypeInfo, Clone)]
pub enum GameStage {
    #[default]
    Preparation,
    InProgress(StageDescription),
    Reveal(StageDescription),
}

impl GameStage {
    pub fn game_is_in_progress(&self) -> bool {
        match self {
            GameStage::Preparation => false,
            GameStage::InProgress(_) | GameStage::Reveal(_) => true,
        }
    }

    pub fn move_can_be_made(&self) -> bool {
        match self {
            GameStage::Preparation | GameStage::InProgress(_) => true,
            GameStage::Reveal(_) => false,
        }
    }

    pub fn is_player_in_game(&self, player: &ActorId) -> bool {
        match self {
            GameStage::Preparation => false,
            GameStage::InProgress(description) => {
                description.anticipated_players.contains(player)
                    || description.finished_players.contains(player)
            }
            GameStage::Reveal(description) => {
                description.anticipated_players.contains(player)
                    || description.finished_players.contains(player)
            }
        }
    }

    pub fn current_players(&self) -> Option<BTreeSet<ActorId>> {
        let description: &StageDescription;

        match self {
            GameStage::Preparation => return None,
            GameStage::InProgress(progress_description) => {
                description = progress_description;
            }
            GameStage::Reveal(reveal_description) => {
                description = reveal_description;
            }
        }

        let players = description
            .anticipated_players
            .union(&description.finished_players)
            .copied()
            .collect();
        Some(players)
    }
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct Duration {
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum RevealResult {
    Continue,
    NextRoundStarted { players: BTreeSet<ActorId> },
    GameOver { winner: ActorId },
}

#[derive(Debug, Decode, Encode, TypeInfo)]
pub enum Action {
    Register,
    MakeMove(String),
    Reveal(String),
    ChangeNextGameConfig(GameConfig),
    StopGame,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum Event {
    PlayerRegistred,
    SuccessfulMove(ActorId),
    SuccessfulReveal(RevealResult),
    GameConfigChanged,
    GameWasStopped(BTreeSet<ActorId>),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum State {
    Config,
    LobbyList,
    GameState,
    CurrentStageTimestamp,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum StateReply {
    Config(GameConfig),
    LobbyList(Vec<ActorId>),
    GameStage(GameStage),
    CurrentStageTimestamp(u64),
}

#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub struct GameConfig {
    pub bet_size: u128,
    pub players_count_limit: u8,
    pub entry_timeout: u64,
    pub move_timeout: u64,
    pub reveal_timeout: u64,
}
