use crate::game::adapter::{
    GameAdapter, GameAdapterError, GenericGameMove, GenericGameState, State,
};
use crate::game::{GameId, SessionId};
use serde::Deserialize;
use serde_json::Value;
use std::vec;
use std::vec::Vec;

const NUM_PLAYERS: usize = 2;
const ROW_SIZE: usize = 6;
const COL_SIZE: usize = 7;
const CONNECT_FOUR: usize = 4;
struct Connect4Adapter {
    game_id: GameId,
    players: Vec<String>,
    state: State,
    next_move: String,
    game: Connect4,
    //winner: Vec<String>
}

#[derive(Deserialize)]
struct Connect4RequestPayload {
    column: usize,
}
struct Connect4 {
    completed: bool,
    turn: Token,
    board: Vec<Vec<Token>>, // vector of columns, each variable length.
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Token {
    Red,
    Blue,
}

impl GameAdapter for Connect4Adapter {
    fn new(game_id: GameId) -> Self
    where
        Self: Sized,
    {
        Connect4Adapter {
            game_id,
            players: vec![],
            state: State::Waiting,
            next_move: "".parse().unwrap(),
            game: Connect4 {
                completed: false,
                turn: Token::Red,
                board: vec![vec![]; COL_SIZE],
            },
        }
    }

    fn add_player(&mut self, username: String) -> actix_web::Result<()> {
        if self.players.len() >= NUM_PLAYERS {
            return Err(actix_web::Error::from(
                GameAdapterError::PlayerLimitExceeded(NUM_PLAYERS),
            ));
        }

        self.players.push(username);
        if self.players.len() == NUM_PLAYERS {
            self.state = State::InProgress
        }
        Ok(())
    }

    fn play_move(&mut self, game_move: GenericGameMove) -> actix_web::Result<()> {
        if self.state != State::InProgress {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                self.state,
            )));
        }
        let request_payload = serde_json::from_value::<Connect4RequestPayload>(game_move.payload)?; //serde_json::from_value(GenericGameMove);
        let column = request_payload.column;
        self.game.moves(column)?;
        Ok(())
    }

    fn get_encoded_state(&self) -> actix_web::Result<GenericGameState> {
        let encoded_board = self
            .game
            .board
            .iter()
            .map(|col| {
                col.iter()
                    .map(|&token| match token {
                        Token::Red => self.players.get(0),
                        Token::Blue => self.players.get(1),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(GenericGameState {
            players: self.players.clone(),
            state: self.state,
            can_move: vec![self.next_move.clone()],
            payload: serde_json::to_value(&encoded_board)?,
        })
    }
}

impl Connect4 {
    fn get_cell_at(&self, row: usize, col: usize) -> Option<Token> {
        Some(*self.board.get(col)?.get(row)?)
    }

    fn insert_move_if_legal(&mut self, column: usize) -> actix_web::Result<()> {
        if column >= COL_SIZE {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                State::InProgress,
            )));
        } else if self.board.get(column).iter().len() >= ROW_SIZE {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                State::InProgress,
            )));
        }
        self.board.get_mut(column - 1).unwrap().push(self.turn);
        Ok(())
    }

    fn switch_token(&mut self) {
        self.turn = match self.turn {
            Token::Red => Token::Blue,
            Token::Blue => Token::Red,
        };
    }

    fn winning_move(&mut self, column: usize) -> bool {
        let row = self.board.get(column).iter().len() - 1;
        // let player = self.turn;
        let mut col_aux = column;
        let mut row_aux = row;
        let mut lenl = 0;
        let mut lenr = 0;
        let mut ok = true;

        //1.) Down
        while (lenl < CONNECT_FOUR) && ok && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            if row_aux > 0 {
                row_aux -= 1;
            } else {
                ok = false;
            }
        }
        if lenl >= CONNECT_FOUR {
            return true;
        }

        //2.) Left + Right
        lenl = 0;
        row_aux = row;
        col_aux = column;
        ok = true;
        while (lenl < CONNECT_FOUR)
            && (ok)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            if col_aux > 0 {
                col_aux -= 1;
            } else {
                ok = false;
            }
        }
        while (lenr < CONNECT_FOUR)
            && (col_aux < COL_SIZE)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            col_aux += 1;
        }
        if lenl + lenr >= CONNECT_FOUR {
            return true;
        }

        //3.) LeftUp + RightDown
        lenl = 0;
        lenr = 0;
        row_aux = row;
        col_aux = column;
        ok = true;
        while (lenl < CONNECT_FOUR)
            && (ok)
            && (row_aux < ROW_SIZE)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            if col_aux > 0 {
                col_aux -= 1;
            } else {
                ok = false;
            }
            row_aux += 1;
        }
        ok = true;
        while (lenr < CONNECT_FOUR)
            && (col_aux < COL_SIZE)
            && (ok)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            if row_aux > 0 {
                row_aux -= 1;
            } else {
                ok = false;
            }
            col_aux += 1;
        }
        if lenl + lenr >= CONNECT_FOUR {
            return true;
        }

        //4.) LeftDomn + RightUp
        lenl = 0;
        lenr = 0;
        row_aux = row;
        col_aux = column;
        ok = true;
        while (lenl < CONNECT_FOUR)
            && (ok)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            if row_aux > 0 {
                row_aux -= 1;
            } else {
                ok = false;
            }
            if col_aux > 0 {
                col_aux -= 1;
            } else {
                ok = false;
            }
        }
        while (lenr < CONNECT_FOUR)
            && (col_aux < COL_SIZE)
            && (row_aux < ROW_SIZE)
            && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
        {
            lenl += 1;
            col_aux += 1;
            row_aux += 1;
        }
        if lenl + lenr >= CONNECT_FOUR {
            return true;
        }

        false
    }
    fn draw_move(&self) -> bool {
        let full_table = self.board.iter().all(|ref col| col.len() == ROW_SIZE);
        return full_table;
    }

    fn moves(&mut self, column: usize) -> actix_web::Result<()> {
        if self.completed {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                State::Ended,
            )));
        }
        self.insert_move_if_legal(column)?;
        let win = self.winning_move(column);
        let draw = self.draw_move();
        if win || draw {
            self.completed = true;
        } else {
            self.switch_token();
        }
        Ok(())
    }
}
