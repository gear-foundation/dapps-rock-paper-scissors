#![no_std]
use gstd::{msg, prelude::*, prog::ProgramGenerator, ActorId, CodeHash};

pub mod entities;
use entities::*;

#[derive(Default)]
pub struct Aggregator {
    generated_programs: Vec<ActorId>,
    code_hash: CodeHash,
}

impl Aggregator {
    fn create(&mut self, init_payload: Vec<u8>) {
        let address = ProgramGenerator::create_program(self.code_hash, init_payload, 0)
            .expect("Error in creating program");
        self.generated_programs.push(address);

        msg::reply(AggregatorEvent::Created { address }, 0).expect("Error in sending reply");
    }

    fn get_generated_programs(&self) -> Vec<ActorId> {
        self.generated_programs.clone()
    }
}

static mut AGGREGATOR: Option<Aggregator> = None;

#[gstd::async_main]
async fn main() {
    let action: AggregatorAction = msg::load().expect("Unable to decode `LoanFactoryAction");
    let aggregator: &mut Aggregator = unsafe { AGGREGATOR.get_or_insert(Default::default()) };
    match action {
        AggregatorAction::Create { init_payload } => aggregator.create(init_payload),
    }
}

#[no_mangle]
unsafe extern "C" fn init() {
    let code_hash: CodeHash = msg::load().expect("Unable to decode CodeHash of Loan program");
    let aggregator = Aggregator {
        code_hash,
        ..Default::default()
    };
    AGGREGATOR = Some(aggregator);
}

#[no_mangle]
unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: AggregatorState = msg::load().expect("failed to decode input argument");
    let aggregator: &mut Aggregator = AGGREGATOR.get_or_insert(Default::default());

    let encoded = match query {
        AggregatorState::GeneratedPrograms => {
            AggregatorStateReply::GeneratedPrograms(aggregator.get_generated_programs())
        }
    }
    .encode();

    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "Aggregator",
    init:
        input: CodeHash,
    handle:
        input: AggregatorAction,
        output: AggregatorEvent,
    state:
        input: AggregatorState,
        output: AggregatorStateReply,
}
