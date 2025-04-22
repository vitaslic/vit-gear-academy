#![no_std]
#![allow(warnings)]
use gstd::{debug, exec, msg};
use pebbles_game_io::*;

static mut PEBBLES_GAME: Option<GameState> = None;

#[no_mangle]
unsafe extern "C" fn init() {
    let pebbles_init: PebblesInit = msg::load().expect("Failed to decode PebblesInit");

    if pebbles_init.pebbles_count <= pebbles_init.max_pebbles_per_turn {
        panic!("Pebbles count must be greater than max pebbles per turn");
    }

    let first_player = choose_first_player();

    let game_state = GameState {
        pebbles_count: pebbles_init.pebbles_count,
        max_pebbles_per_turn: pebbles_init.max_pebbles_per_turn,
        pebbles_remaining: pebbles_init.pebbles_count,
        difficulty: pebbles_init.difficulty,
        first_player: first_player.clone(),
        winner: None,
    };

    PEBBLES_GAME = Some(game_state);

    if first_player == Player::Program {
        exec_program_turn();
    }
}

#[no_mangle]
unsafe extern "C" fn handle() {
    let game_state = PEBBLES_GAME.as_mut().expect("Failed to load game state");

    let action: PebblesAction = msg::load().expect("Failed to decode PebblesAction");

    let mut finished = false;

    let mut counter_turn = 0_u32;
    match action {
        PebblesAction::Turn(count) => match remove_pebbles(count) {
            Some(()) => {
                if game_state.pebbles_remaining != 0 {
                    counter_turn = exec_program_turn();
                    if game_state.pebbles_remaining == 0 {
                        game_state.winner = Some(Player::Program);
                        finished = true;
                    }
                } else {
                    game_state.winner = Some(Player::User);
                    finished = true;
                }
            }
            None => (),
        },
        PebblesAction::GiveUp => {
            debug!("The user leaves");
            game_state.winner = Some(Player::Program);
            finished = true;
        }
        PebblesAction::Restart {
            difficulty,
            pebbles_count,
            max_pebbles_per_turn,
        } => {
            debug!("The user restarts the gane");
            let pebbles_init = PebblesInit {
                difficulty,
                pebbles_count,
                max_pebbles_per_turn,
            };

            let first_player = choose_first_player();
            let game_state = GameState {
                pebbles_count: pebbles_init.pebbles_count,
                max_pebbles_per_turn: pebbles_init.max_pebbles_per_turn,
                pebbles_remaining: pebbles_init.pebbles_count,
                difficulty: pebbles_init.difficulty,
                first_player: first_player.clone(),
                winner: None,
            };
            PEBBLES_GAME = Some(game_state);
            if first_player == Player::Program {
                counter_turn = exec_program_turn();
            }
        }
    }
    if finished {
        debug!(
            "The game is over and the winner is: {:?}",
            game_state.winner.clone().unwrap()
        );
        msg::reply(PebblesEvent::Won(game_state.winner.clone().unwrap()), 0)
            .expect("Failed to send game over event");
    } else {
        msg::reply(PebblesEvent::CounterTurn(counter_turn), 0)
            .expect("Failed to send counter turn event");
    }
}

#[no_mangle]
unsafe extern "C" fn state() {
    let game_state = PEBBLES_GAME.take().expect("Game state is not initialized");
    msg::reply(game_state, 0).expect("Failed to reply from `state()`");
}

fn choose_first_player() -> Player {
    let r_num = get_random_u32();
    if r_num % 2 == 0 {
        debug!("First player Random generated : User");
        Player::User
    } else {
        debug!("First player Random generated: Program");
        Player::Program
    }
}

#[cfg(feature = "test_user_first")]
fn choose_first_player() -> Player {
    debug!("First player: User");
    Player::User
}

#[cfg(feature = "test_program_first")]
fn choose_first_player() -> Player {
    debug!("First player: Program");
    Player::Program
}
unsafe fn exec_program_turn() -> u32 {
    let game_state: &mut GameState = PEBBLES_GAME.as_mut().expect("Failed to load game state");
    let count: u32;
    match game_state.difficulty {
        DifficultyLevel::Easy => {
            count = take_easy_action();
        }
        DifficultyLevel::Hard => {
            count = take_hard_action();
        }
    }
    game_state.pebbles_remaining -= count;
    debug!("The number of stones removed by the program {:?}", count);
    debug!(
        "Number of stones remaining: {}",
        game_state.pebbles_remaining
    );
    count
}

unsafe fn take_easy_action() -> u32 {
    let game_state: &mut GameState = PEBBLES_GAME.as_mut().expect("Failed to load game state");
    let mut count = get_random_u32();

    if game_state.pebbles_remaining < game_state.max_pebbles_per_turn {
        count = count % game_state.pebbles_remaining;
        if count == 0 {
            count = game_state.pebbles_remaining;
        }
    } else {
        count = count % game_state.max_pebbles_per_turn;
        if count == 0 {
            count = game_state.max_pebbles_per_turn;
        }
    }
    count
}

unsafe fn take_hard_action() -> u32 {
    let game_state: &mut GameState = PEBBLES_GAME.as_mut().expect("Failed to load game state");

    let target = game_state.pebbles_remaining % (game_state.max_pebbles_per_turn + 1);

    let count = if target == 0 {
        game_state.max_pebbles_per_turn
    } else {
        target
    };
    count
}

unsafe fn remove_pebbles(count: u32) -> Option<()> {
    let game_state = PEBBLES_GAME.as_mut().expect("Failed to load game state");

    if count == 0 {
        debug!(
            "The number of stones entered by the user cannot be 0, and the user needs to re-enter"
        );
        return None;
    }

    if count > game_state.max_pebbles_per_turn {
        debug!("The number of stones entered by the user cannot be greater than the maximum number of stones that can be taken, and the user needs to re-enter");
        return None;
    }
    if count > game_state.pebbles_remaining {
        debug!("The number of stones entered by the user cannot be greater than the remaining number of stones, and the user needs to re-enter ");
        return None;
    }
    game_state.pebbles_remaining -= count;

    Some(())
}

fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}
