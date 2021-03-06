use crate::game::adapter::{
    GameAdapter, GameAdapterError, GameAdapterErrorType, GenericGameMove, GenericGameState, Stage,
};
use crate::game::{GameId, GameType};
use crate::notify::Notifier;
use actix_web::error::{InternalError, JsonPayloadError};
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use std::vec;
use std::vec::Vec;

const NUM_PLAYERS: usize = 2;
const ROW_SIZE: usize = 6;
const COL_SIZE: usize = 7;
const CONNECT_FOUR: usize = 4;

pub struct Connect4Adapter {
    game_id: GameId,
    players: Vec<String>,
    stage: Stage,
    notifier: Notifier,
    game: Connect4,
    winner: Vec<String>,
}

#[derive(Deserialize)]
pub enum ConstConnect4 {
    #[serde(rename = "connect_4")]
    Connect4,
}

#[derive(Deserialize)]
struct Connect4RequestPayload {
    game_type: ConstConnect4,
    column: usize,
}

#[derive(Serialize)]
struct Connect4ResponsePayload<'a> {
    cells: Vec<Vec<&'a String>>,
}

struct Connect4 {
    game_id: GameId,
    completed: bool,
    turn: Token,
    board: Vec<Vec<Token>>, // vector of columns, each variable length.
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Token {
    Red,
    Blue,
}

impl Connect4Adapter {
    fn get_user_from_token(&self) -> String {
        let user = match self.game.turn {
            Token::Red => self.players.get(0).unwrap().clone(),
            Token::Blue => self.players.get(1).unwrap().clone(),
        };
        user
    }
}

impl GameAdapter for Connect4Adapter {
    fn new(game_id: GameId) -> Self
    where
        Self: Sized,
    {
        Connect4Adapter {
            game_id,
            players: vec![],
            stage: Stage::Waiting,
            notifier: Notifier::new(),
            game: Connect4 {
                game_id,
                completed: false,
                turn: Token::Red,
                board: vec![vec![]; COL_SIZE],
            },
            winner: vec![],
        }
    }

    fn get_notifier(&self) -> &Notifier {
        &self.notifier
    }

    fn add_player(&mut self, username: String) -> actix_web::Result<()> {
        assert!(self.players.len() < NUM_PLAYERS);
        assert_eq!(self.stage, Stage::Waiting);

        self.players.push(username);
        if self.players.len() == NUM_PLAYERS {
            self.stage = Stage::InProgress;
        }
        self.notifier.send();
        Ok(())
    }

    fn has_player(&self, username: &str) -> bool {
        self.players.iter().any(|s| s.eq(username))
    }

    fn play_move(&mut self, game_move: GenericGameMove) -> actix_web::Result<()> {
        if self.stage == Stage::Waiting {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidGameStage(self.stage),
            ));
        }
        if self.stage == Stage::Ended {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidGameStage(self.stage),
            ));
        }

        let request_payload = serde_json::from_value::<Connect4RequestPayload>(game_move.payload)
            .map_err(|e| {
            InternalError::from_response(
                "",
                HttpResponse::BadRequest()
                    .content_type("text/plain")
                    .body(JsonPayloadError::Deserialize(e).to_string()),
            )
        })?;

        let column = request_payload.column;
        let player = self.get_user_from_token();
        let user = game_move.player;

        if player != user {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidPlayer(user),
            )); // return the one who made the request
        }
        self.game.moves(column)?;
        let win = self.game.winning_move(column);
        let draw = self.game.is_game_drawn();
        if win {
            self.winner.push(self.get_user_from_token());
        }
        if win || draw {
            self.game.completed = true;
            self.stage = Stage::Ended;
        } else {
            self.game.switch_token();
        }
        self.notifier.send();
        Ok(())
    }

    fn get_stage(&self) -> Stage {
        self.stage
    }

    fn get_encoded_state(&self) -> actix_web::Result<GenericGameState> {
        let encoded_board = self
            .game
            .board
            .iter()
            .map(|col| {
                col.iter()
                    .map(|&token| {
                        match token {
                            Token::Red => self.players.get(0),
                            Token::Blue => self.players.get(1),
                        }
                        .unwrap()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let response_payload = Connect4ResponsePayload {
            cells: encoded_board,
        };
        Ok(GenericGameState {
            players: self.players.clone(),
            stage: self.stage,
            can_move: if self.stage == Stage::InProgress {
                vec![self.get_user_from_token()]
            } else {
                vec![]
            },
            winners: self.winner.clone(),
            payload: serde_json::to_value(&response_payload)?,
        })
    }

    fn get_type(&self) -> GameType {
        GameType::Connect4
    }
}

impl Connect4 {
    fn get_cell_at(&self, row: isize, col: isize) -> Option<Token> {
        if row < 0 || col < 0 || row >= ROW_SIZE as isize || col >= COL_SIZE as isize {
            return None;
        }
        Some(*self.board.get(col as usize)?.get(row as usize)?)
    }

    fn insert_move_if_legal(&mut self, column: usize) -> actix_web::Result<()> {
        if column >= COL_SIZE {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidMove(format!("column {} does not exist", column)),
            ));
        } else if self.board.get(column).unwrap().len() >= ROW_SIZE {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidMove(format!("column {} is already full", column)),
            ));
        } else {
            self.board.get_mut(column).unwrap().push(self.turn);
        }
        Ok(())
    }

    fn switch_token(&mut self) {
        self.turn = match self.turn {
            Token::Red => Token::Blue,
            Token::Blue => Token::Red,
        };
    }
    fn winning_move(&mut self, column: usize) -> bool {
        if column >= COL_SIZE {
            return false;
        }
        let row = self.board.get(column).unwrap().len() - 1;
        let direction_col = vec![0, -1, 1, -1, 1, -1, 1]; // Down,Left,Right,LU ,RD, LD, RU
        let direction_row = vec![-1, 0, 0, 1, -1, -1, 1];
        let mut lengths = Vec::with_capacity(7);
        let mut col_parser;
        let mut row_parser;
        lengths.resize(7, 0);
        for counter in 0..7 {
            col_parser = column as isize; // usize
            row_parser = row as isize;
            while self.get_cell_at(row_parser, col_parser) == Some(self.turn) {
                lengths[counter] += 1;
                col_parser += direction_col[counter];
                row_parser += direction_row[counter];
            }
        }
        if lengths[0] >= CONNECT_FOUR as isize {
            return true;
        }
        for pair in 0..3 {
            if lengths[2 * pair + 1] + lengths[2 * pair + 2] > CONNECT_FOUR as isize {
                return true;
            }
        }
        false
    }

    fn is_game_drawn(&self) -> bool {
        self.board.iter().all(|ref col| col.len() == ROW_SIZE)
    }

    fn moves(&mut self, column: usize) -> actix_web::Result<()> {
        if self.completed {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidGameStage(Stage::Ended),
            ));
        }
        self.insert_move_if_legal(column)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_game() -> Connect4 {
        Connect4 {
            game_id: GameId::new(),
            completed: false,
            turn: Token::Red,
            board: vec![vec![]; COL_SIZE],
        }
    }

    fn display_board(board: &Vec<Vec<Token>>) {
        board.iter().for_each(|it| {
            println!("{:#?}", it);
        });
    }

    fn play_moves(game: &mut Connect4, moves: Vec<usize>) {
        moves.iter().for_each(|column| {
            game.insert_move_if_legal(*column).unwrap();
            game.switch_token();
        });
        game.switch_token();
    }

    #[test]
    fn down_win_detected() {
        let mut game = create_game();

        play_moves(&mut game, vec![0, 3, 0, 2, 0, 1, 0]);

        display_board(&game.board);
        assert!(game.winning_move(0));
    }

    #[test]
    fn left_right_win_detected() {
        let mut game = Connect4 {
            game_id: GameId::new(),
            completed: false,
            turn: Token::Red,
            board: vec![vec![]; COL_SIZE],
        };

        play_moves(&mut game, vec![3, 3, 2, 0, 1, 1, 4]);

        display_board(&game.board);
        assert!(game.winning_move(4));
    }

    #[test]
    fn left_up_and_right_down_win_detected() {
        let mut game = Connect4 {
            game_id: GameId::new(),
            completed: false,
            turn: Token::Red,
            board: vec![vec![]; COL_SIZE],
        };

        play_moves(&mut game, vec![2, 3, 1, 2, 1, 1, 0, 0, 0, 0]);

        display_board(&game.board);
        assert!(game.winning_move(0));
    }

    #[test]
    fn left_down_and_right_up_win_detected() {
        let mut game = Connect4 {
            game_id: GameId::new(),
            completed: false,
            turn: Token::Red,
            board: vec![vec![]; COL_SIZE],
        };

        play_moves(&mut game, vec![2, 3, 3, 4, 4, 5, 4, 5, 0, 5, 5]);

        display_board(&game.board);
        assert!(game.winning_move(5));
    }
}
