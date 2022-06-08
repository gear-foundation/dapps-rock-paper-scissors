#![no_std]

use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

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

    pub fn current_players(&self) -> Option<BTreeSet<&ActorId>> {
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
            // .cloned()
            .collect();
        Some(players)
    }
}

#[derive(Debug, Decode, Encode, TypeInfo)]
pub enum Action {
    AddPlayerInLobby(ActorId),
    RemovePlayerFromLobby(ActorId),
    SetLobbyPlayersList(Vec<ActorId>),
    SetBetSize(u128),
    MakeMove(String),
    Reveal(String),
    StopGame,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum Event {
    PlayerWasAdded(ActorId),
    PlayerWasRemoved(ActorId),
    LobbyPlayersListUpdated,
    BetSizeWasChanged(u128),
    SuccessfulMove(ActorId),
    SuccessfulReveal(ActorId),
    GameWasStopped,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum State {
    BetSize,
    LobbyList,
    // RemainingPlayersList,
    // MovedPlayersList,
    // RevealedPlayersList,
    GameState,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum StateReply {
    BetSize(u128),
    LobbyList(Vec<ActorId>),
    GameStage(GameStage),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct InitConfig {
    pub bet_size: u128,
    pub lobby_players: Vec<ActorId>,
}
