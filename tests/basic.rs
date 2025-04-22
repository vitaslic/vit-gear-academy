use gtest::{Program, System};
use pebbles_game_io::*;

static SENDER_ID: u64 = 16;
static PEBBLES_COUNT: u32 = 72;
static MAX_PEBBLES_PER_TURN: u32 = 5;

#[test]
fn test_init_success() {
    let sys = System::new();
    sys.init_logger();
    init_game_success(&sys, DifficultyLevel::Easy);
}

fn init_game_success(sys: &System, difficulty: DifficultyLevel) -> Program {
    let pebbles_init = PebblesInit {
        difficulty: difficulty,
        pebbles_count: PEBBLES_COUNT,
        max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
    };

    let pebbles_game = Program::current(&sys);
    sys.mint_to(SENDER_ID, 100000000000000000);
    pebbles_game.send(SENDER_ID, pebbles_init);
    sys.run_next_block();
    pebbles_game
}

fn read_state(pebbles_game: &Program) -> GameState {
    pebbles_game.read_state(b"").unwrap()
}

fn create_user_turns(game_state: &GameState) -> Vec<u32> {
    let mut user_turns: Vec<u32> = Vec::new();
    let mut count = 0;
    for _ in 0..game_state.pebbles_count {
        let mut turn_num = (count + 31) % MAX_PEBBLES_PER_TURN;
        if turn_num == 0 {
            if count % 2 == 0 {
                turn_num = MAX_PEBBLES_PER_TURN;
            } else {
                turn_num = 1;
            }
        }
        user_turns.push(turn_num);
        count += 1;
    }
    user_turns
}

#[test]
fn test_state() {
    let sys = System::new();
    sys.init_logger();

    let pebbles_game = init_game_success(&sys, DifficultyLevel::Easy);

    let game_state = read_state(&pebbles_game);

    assert_eq!(game_state.pebbles_count, PEBBLES_COUNT);
    assert_eq!(game_state.max_pebbles_per_turn, MAX_PEBBLES_PER_TURN);
}

#[test]
fn test_handle_give_up() {
    let sys = System::new();
    sys.init_logger();

    let pebbles_game = init_game_success(&sys, DifficultyLevel::Hard);
    let mut game_state = read_state(&pebbles_game);

    let user_turns = create_user_turns(&game_state);

    let mut count = 0;
    loop {
        if game_state.pebbles_remaining <= game_state.pebbles_count / 2 {
            pebbles_game.send(SENDER_ID, PebblesAction::GiveUp);
            sys.run_next_block();
            break;
        }
        pebbles_game.send(SENDER_ID, PebblesAction::Turn(user_turns[count]));
        sys.run_next_block();
        game_state = read_state(&pebbles_game);
        count += 1;
    }
    game_state = read_state(&pebbles_game);

    assert_eq!(game_state.winner, Some(Player::Program));
}

#[test]
fn test_handle_restart() {
    let sys = System::new();
    sys.init_logger();

    let pebbles_game = init_game_success(&sys, DifficultyLevel::Hard);
    let mut game_state = read_state(&pebbles_game);

    let user_turns = create_user_turns(&game_state);

    let mut count = 0;
    loop {
        if game_state.pebbles_remaining <= game_state.pebbles_count / 2 {
            pebbles_game.send(
                SENDER_ID,
                PebblesAction::Restart {
                    difficulty: DifficultyLevel::Hard,
                    pebbles_count: PEBBLES_COUNT,
                    max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
                },
            );
            sys.run_next_block();
            break;
        }
        pebbles_game.send(SENDER_ID, PebblesAction::Turn(user_turns[count]));
        sys.run_next_block();
        game_state = read_state(&pebbles_game);
        count += 1;
    }

    game_state = read_state(&pebbles_game);
    assert_eq!(game_state.pebbles_count, PEBBLES_COUNT);
    if game_state.first_player == Player::Program {
        assert_ne!(game_state.pebbles_count, game_state.pebbles_remaining);
    } else {
        assert_eq!(game_state.pebbles_count, game_state.pebbles_remaining);
    }
}
