use crate::game::adapter::{
    GameAdapter, GameAdapterError, GenericGameMove, GenericGameState, State,
};
use crate::game::GameId;
use serde::{Deserialize, Serialize};
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
    notifier: broadcast::Sender<()>,
    game: Connect4,
    winner: Vec<String>,
}

#[derive(Deserialize)]
struct Connect4RequestPayload {
    column: usize,
}

#[derive(Serialize)]
struct Connect4ResponsePayload<'a> {
    cells: Vec<Vec<&'a String>>,
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
        if self.players.len() >= NUM_PLAYERS {
            return Err(actix_web::Error::from(
                GameAdapterError::PlayerLimitExceeded(NUM_PLAYERS),
            ));
        }
        self.players.push(username);
        self.notifier.send(());
        Ok(())
    }

    fn has_player(&self, username: &str) -> bool {
        self.players.iter().any(|s| s.eq(username))
    }

    fn get_user_from_token(&self) -> String {
        let user = match self.game.turn {
            Token::Red => self.players.get(0).unwrap().clone(),
            Token::Blue => self.players.get(1).unwrap().clone(),
        };
        user
    }

    fn play_move(&mut self, game_move: GenericGameMove) -> actix_web::Result<()> {
        if self.state == State::Waiting {
            return Err(actix_web::Error::from(GameAdapterError::InvalidGameState(
                self.state,
            )));
        }
        if self.state == State::Ended {
            return Err(actix_web::Error::from(GameAdapterError::GameEnded()));
        }
        let request_payload = serde_json::from_value::<Connect4RequestPayload>(game_move.payload);
        let column = request_payload.unwrap().column;
        let player = self.get_user_from_token();
        let user = game_move.player;

        if player != user {
            return Err(actix_web::Error::from(
                GameAdapterError::WrongPlayerRequest(user),
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
            self.state = State::Ended;
        } else {
            self.game.switch_token();
        }
        self.notifier.send(());
        Ok(())
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
            game: "connect_4".to_string(),
            players: self.players.clone(),
            state: self.state,
            can_move: if self.state == State::InProgress {
                vec![self.get_user_from_token()]
            } else {
                vec![]
            },
            winners: self.winner.clone(),
            payload: serde_json::to_value(&response_payload)?,
        })
    }
}

impl Connect4 {
    fn get_cell_at(&self, row: usize, col: usize) -> Option<Token> {
        Some(*self.board.get(col)?.get(row)?)
    }

    fn insert_move_if_legal(&mut self, column: usize) -> actix_web::Result<()> {
        if column >= COL_SIZE {
            return Err(actix_web::Error::from(GameAdapterError::WrongMoveRequest(
                column,
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
        if column >= COL_SIZE {
            return false;
        }
        let row = self.board.get(column).unwrap().len() - 1;
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
        lenl + lenr >= CONNECT_FOUR
    }
    fn is_game_drawn(&self) -> bool {
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
}
