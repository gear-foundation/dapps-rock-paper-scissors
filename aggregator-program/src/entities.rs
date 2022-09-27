use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

#[derive(Encode, Decode, TypeInfo)]
pub enum AggregatorAction {
    Create { init_payload: Vec<u8> },
}

#[derive(Encode, Decode, TypeInfo)]
pub enum AggregatorEvent {
    Created { address: ActorId },
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum AggregatorState {
    GeneratedPrograms,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum AggregatorStateReply {
    GeneratedPrograms(Vec<ActorId>),
}
