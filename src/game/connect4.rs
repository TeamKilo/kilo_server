use crate::game::adapter::{
    GameAdapter, GameAdapterError, GenericGameMove, GenericGameState, State,
};
use crate::game::{GameId, SessionId};

use serde::Deserialize;
use serde_json::Value;
use std::vec;
use std::vec::Vec;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

const NUM_PLAYERS: usize = 2;
const ROW_SIZE: usize = 6;
const COL_SIZE: usize = 7;
const CONNECT_FOUR: usize = 4;

pub struct Connect4Adapter {
    game_id: GameId,
    players: Vec<String>,
    state: State,
    next_move: String,
    notifier: broadcast::Sender<()>,
    game: Connect4,
    winner: Vec<String>,
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
            notifier: broadcast::channel(16).0,
            game: Connect4 {
                completed: false,
                turn: Token::Red,
                board: vec![vec![]; COL_SIZE],
            },
            winner: vec![],
        }
    }

    fn get_notifier(&self) -> &Sender<()> {
        &self.notifier
    }

    fn add_player(&mut self, username: String) -> actix_web::Result<()> {
        if self.players.len() < NUM_PLAYERS {
            self.players.push(username);
            if self.players.len() == NUM_PLAYERS {
                self.state = State::InProgress
            }
            self.notifier.send(());
            Ok(())
        } else {
            Err(actix_web::Error::from(
                GameAdapterError::PlayerLimitExceeded(NUM_PLAYERS),
            ))
        }
    }

    fn has_player(&self, username: &str) -> bool {
        self.players.iter().any(|s| s.eq(username))
    }

    fn play_move(&mut self, game_move: GenericGameMove) -> actix_web::Result<()> {
        if self.state != State::InProgress {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                self.state,
            )));
        }
        let request_payload = serde_json::from_value::<Connect4RequestPayload>(game_move.payload)?;
        let column = request_payload.column;
        let mut player = match self.game.turn {
            Token::Red => self.players.get(0).unwrap().clone(),
            Token::Blue => self.players.get(1).unwrap().clone(),
        };
        let mut user = game_move.player;
        if player == user {
            self.game.moves(column)?;
            let win = self.game.winning_move(column);
            let draw = self.game.draw_move();
            if win {
                let mut winner = match self.game.turn {
                    Token::Red => self.players.get(0).unwrap().clone(),
                    Token::Blue => self.players.get(1).unwrap().clone(),
                };
                self.winner.push(winner);
            }
            if win || draw {
                self.game.completed = true;
            } else {
                self.game.switch_token();
            }
            Ok(())
        } else {
            Err(actix_web::Error::from(
                GameAdapterError::WrongPlayerRequest(user), // return the one who made the request
            ))
        }
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
            game: "connect_4".to_string(),
            players: self.players.clone(),
            state: self.state,
            can_move: vec![self.next_move.clone()],
            winners: vec![],
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
        } else if self.board.get(column).unwrap().len() >= ROW_SIZE {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                State::InProgress,
            )));
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
        if (column < COL_SIZE) {
            let row = self.board.get(column).unwrap().len() - 1;
            // let player = self.turn;
            let mut col_aux = column;
            let mut row_aux = row;
            let mut lenl = 0;
            let mut lenr = 0;
            let mut ok = true;

            //1.) Down
            while (lenl < CONNECT_FOUR)
                && ok
                && (self.get_cell_at(row_aux, col_aux) == Some(self.turn))
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
            lenl + lenr >= CONNECT_FOUR
        } else {
            false
        }
    }
    fn draw_move(&self) -> bool {
        self.board.iter().all(|ref col| col.len() == ROW_SIZE)
    }

    fn moves(&mut self, column: usize) -> actix_web::Result<()> {
        if self.completed {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                State::Ended,
            )));
        }
        self.insert_move_if_legal(column)?;
        Ok(())
    }
    fn visualize(&mut self, column: usize) {
        let mut ct = 0;
        let mut tok_turn = "";

        if self.turn == Token::Red {
            tok_turn = "Red";
        }
        if self.turn == Token::Blue {
            tok_turn = "Blue";
        }
        println!("Adding on column {}: on token {}", column, tok_turn);
        self.insert_move_if_legal(column);
        println!("Board after inserting:");
        self.board.iter().for_each(|it| {
            println!("Col {}, {:#?}", ct, it);
            ct += 1;
        });
        let win = self.winning_move(column);
        let draw = self.draw_move();
        println!("Win:{}, Draw:{}", win, draw);

        if win || draw {
            self.completed = true;
        } else {
            self.switch_token();
        }
        println!("Completed: {};", self.completed);
    }
}

#[cfg(test)]
mod tests {

    use crate::game::connect4::Connect4;
    use crate::game::connect4::Token;
    use crate::game::connect4::COL_SIZE;
    #[test]

    fn add_() {
        let mut board1 = Connect4 {
            completed: false,
            turn: Token::Red,
            board: vec![vec![]; COL_SIZE], // vector of columns, each variable length.
        };
        board1.visualize(8);
        board1.visualize(2);
        board1.visualize(2);
        board1.visualize(2);
        board1.visualize(2);
        board1.visualize(2);
        board1.visualize(2);

        assert_eq!(2 + 2, 4);
    }
}
