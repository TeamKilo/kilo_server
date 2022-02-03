use crate::game::adapter::{
    GameAdapter, GameAdapterError, GenericGameMove, GenericGameState, State,
};
use crate::game::{GameId, SessionId};
use std::vec;
use std::vec::Vec;

const NUM_PLAYERS: usize = 2;

pub struct Connect4Adapter<'a> {
    game_id: GameId,
    players: Vec<String>,
    state: State,
    next_move: String,
    game: Connect4<'a>,
}

struct Connect4<'a> {
    board: Vec<Vec<&'a String>>, // vector of columns, each variable length.
}

impl GameAdapter for Connect4Adapter<'_> {
    fn new(game_id: GameId) -> Self
    where
        Self: Sized,
    {
        Connect4Adapter {
            game_id,
            players: vec![],
            state: State::Waiting,
            next_move: "".parse().unwrap(),
            game: Connect4 { board: vec![] },
        }
    }

    fn add_player(&mut self, username: String) -> actix_web::Result<()> {
        if self.players.len() < NUM_PLAYERS {
            self.players.push(username);
            if self.players.len() == NUM_PLAYERS {
                self.state = State::InProgress
            }
            Ok(())
        } else {
            Err(actix_web::Error::from(
                GameAdapterError::PlayerLimitExceeded(NUM_PLAYERS),
            ))
        }
    }

    fn play_move(&mut self, game_move: GenericGameMove) -> actix_web::Result<()> {
        if self.state != State::InProgress {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                self.state,
            )));
        };
        todo!()
    }

    fn get_encoded_state(&self) -> actix_web::Result<GenericGameState> {
        Ok(GenericGameState {
            game: "connect_4".to_string(),
            players: self.players.clone(),
            state: self.state,
            can_move: vec![self.next_move.clone()],
            winners: vec![],
            payload: serde_json::to_value(&self.game.board)?,
        })
    }
}

impl Connect4<'_> {}
