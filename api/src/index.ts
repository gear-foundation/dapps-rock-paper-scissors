import { GearApi, getWasmMetadata, Hex, Metadata, PayloadType } from '@gear-js/api';
import { UploadProgramModel, UploadProgram, ProgramState, MessageModel, Action, Account } from "gear-program-interface-core";
import { payloads } from './payloads';
import * as Blake2b from 'blake2b';

export enum Move {
  ROCK,
  PAPER,
  SCISSORS,
  LIZARD,
  SPOCK,
}

export async function deploy(
  gearApi: GearApi,
  code: File,
  account: Account,
  bet_size: number,
  players: [Hex],
  programIdHandler: (id: Hex) => void,
  metaBuffer?: Buffer,
) {
    const meta =  await getWasmMetadata(metaBuffer);
    const initPayload = payloads.init(bet_size, players);

    const programOptions: UploadProgramModel = {
      meta,
      value: 0,
      initPayload,
    };
  
    UploadProgram(gearApi, account, code, programOptions, programIdHandler)
}

async function sendAction(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  payload: PayloadType,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  const meta = (metaBuffer !== null && metaBuffer !== undefined) ? await getWasmMetadata(metaBuffer) : null;
  let messageModel: MessageModel = {
    destination: programId,
    payload: payload,
  };

  Action(gearApi, account, messageModel, eventHandler, meta,);
}

export async function addPlayerInLobby(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  player: Hex,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.addPlayerInLobby(player),
    eventHandler,
    metaBuffer,
  )
}

export async function removePlayerFromLobby(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  player: Hex,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.removePlayerFromLobby(player),
    eventHandler,
    metaBuffer,
  )
}

export async function setLobbyPlayersList(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  players_list: [Hex],
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.setLobbyPlayersList(players_list),
    eventHandler,
    metaBuffer,
  )
}

export async function setBetSize(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  betSize: number,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.setBetSize(betSize),
    eventHandler,
    metaBuffer,
  )
}

export async function makeMove(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  move: Move,
  password: string,
  bet: number,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  const moveWithPass = move.toString() + password;

  const output = new Uint8Array(32);
  const input = Buffer.from(moveWithPass);
  const result = Blake2b(output.length).update(input).digest('hex');

  sendAction(
    gearApi,
    programId,
    account,
    payloads.makeMove(result),
    eventHandler,
    metaBuffer,
  )
}

export async function reveal(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  move: Move,
  password: string,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.reveal(move.toString() + password),
    eventHandler,
    metaBuffer,
  )
}

export async function stopGame(
  gearApi: GearApi,
  programId: Hex,
  account: Account,
  eventHandler: (event: any) => void,
  metaBuffer?: Buffer,
) {
  sendAction(
    gearApi,
    programId,
    account,
    payloads.stopGame,
    eventHandler,
    metaBuffer,
  )
}

export async function currentBetSize(
  gearApi: GearApi,
  programId: Hex,
  metaBuffer?: Buffer,
) {
  return await ProgramState(gearApi, programId, payloads.betSizeState, metaBuffer);
}

export async function currentLobbyList(
  gearApi: GearApi,
  programId: Hex,
  metaBuffer?: Buffer,
) {
  return await ProgramState(gearApi, programId, payloads.lobbyListState, metaBuffer);
}

export async function currentGameState(
  gearApi: GearApi,
  programId: Hex,
  metaBuffer?: Buffer,
) {
  return await ProgramState(gearApi, programId, payloads.gameState, metaBuffer);
}
