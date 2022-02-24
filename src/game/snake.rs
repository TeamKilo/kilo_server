use crate::game::adapter::{
    GameAdapter, GameAdapterError, GameAdapterErrorType, GenericGameMove, GenericGameState, Stage,
};
use crate::game::{GameId, GameType};
use crate::notify::Notifier;
use derive_more::Display;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Add;
use std::vec;
use std::vec::Vec;

const NUM_PLAYERS: usize = 4;

const BOARD_MIN_X: i32 = -100;
const BOARD_MAX_X: i32 = 100;
const BOARD_MIN_Y: i32 = -100;
const BOARD_MAX_Y: i32 = 100;

pub struct SnakeAdapter {
    game_id: GameId,
    players: Vec<String>,
    stage: Stage,
    notifier: Notifier,
    game: Snake,
}

#[derive(Deserialize)]
pub enum ConstSnake {
    #[serde(rename = "snake")]
    Snake,
}

#[derive(Deserialize, Debug, Copy, Clone, Eq, PartialEq, Display)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[display(fmt = "up")]
    Up,
    #[display(fmt = "down")]
    Down,
    #[display(fmt = "left")]
    Left,
    #[display(fmt = "right")]
    Right,
}

#[derive(Deserialize)]
struct SnakeRequestPayload {
    game_type: ConstSnake,
    direction: Direction,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Point2D {
    x: i32,
    y: i32,
}

impl<'a, 'b> Add<&'a Direction> for &'b Point2D {
    type Output = Point2D;

    fn add(self, rhs: &'a Direction) -> Self::Output {
        match rhs {
            Direction::Up => Point2D {
                x: self.x,
                y: self.y + 1,
            },
            Direction::Down => Point2D {
                x: self.x,
                y: self.y - 1,
            },
            Direction::Left => Point2D {
                x: self.x - 1,
                y: self.y,
            },
            Direction::Right => Point2D {
                x: self.x + 1,
                y: self.y,
            },
        }
    }
}

#[derive(Serialize)]
struct SnakeResponsePayload {
    players: HashMap<String, VecDeque<Point2D>>,
    fruits: HashSet<Point2D>,
}

struct Snake {
    game_id: GameId,
    moves: HashMap<String, Direction>,
    state: SnakeResponsePayload,
}

impl GameAdapter for SnakeAdapter {
    fn new(game_id: GameId) -> Self
    where
        Self: Sized,
    {
        SnakeAdapter {
            game_id,
            players: vec![],
            stage: Stage::Waiting,
            notifier: Notifier::new(),
            game: Snake {
                game_id,
                moves: HashMap::new(),
                state: SnakeResponsePayload {
                    players: HashMap::new(),
                    fruits: HashSet::new(),
                },
            },
        }
    }

    fn get_notifier(&self) -> &Notifier {
        &self.notifier
    }

    fn add_player(&mut self, username: String) -> actix_web::Result<()> {
        assert!(self.players.len() < NUM_PLAYERS);
        assert_eq!(self.stage, Stage::Waiting);

        self.players.push(username.clone());
        self.game.state.players.insert(
            username,
            VecDeque::from([Point2D {
                x: thread_rng().gen_range(BOARD_MIN_X..=BOARD_MAX_X),
                y: thread_rng().gen_range(BOARD_MIN_Y..=BOARD_MAX_Y),
            }]),
        );
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
        if self.stage == Stage::Waiting || self.stage == Stage::Ended {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidGameStage(self.stage),
            ));
        }

        let request_payload = serde_json::from_value::<SnakeRequestPayload>(game_move.payload)?;
        let user = game_move.player;

        if !self.game.state.players.contains_key(&user) || self.game.moves.contains_key(&user) {
            return Err(GameAdapterError::actix_err(
                self.game_id,
                GameAdapterErrorType::InvalidPlayer(user),
            ));
        }

        self.game.record_move(user, request_payload.direction)?;

        if self.game.state.players.len() == 1 {
            self.stage = Stage::Ended;
        }

        self.notifier.send();

        Ok(())
    }

    fn get_stage(&self) -> Stage {
        self.stage
    }

    fn get_encoded_state(&self) -> actix_web::Result<GenericGameState> {
        Ok(GenericGameState {
            players: self.players.clone(),
            stage: self.stage,
            can_move: self.game.state.players.keys().cloned().collect(),
            winners: if self.stage == Stage::Ended {
                self.game.state.players.keys().cloned().collect()
            } else {
                vec![]
            },
            payload: serde_json::to_value(&self.game.state)?,
        })
    }

    fn get_type(&self) -> GameType {
        GameType::Snake
    }
}

impl Snake {
    fn spawn_fruit(&mut self) {
        todo!()
    }

    fn time_step(&mut self) -> actix_web::Result<()> {
        let mut occupied: HashSet<Point2D> = HashSet::new();

        for (player, _) in self.moves.iter() {
            let deque = self.state.players.get(player).unwrap();
            occupied.extend(deque.iter());
        }

        for (player, dir) in self.moves.iter() {
            let deque = self.state.players.get_mut(player).unwrap();
            let new_point = deque.front().unwrap() + dir;

            if new_point.y > BOARD_MAX_Y
                || new_point.y < BOARD_MIN_Y
                || new_point.x > BOARD_MAX_X
                || new_point.x < BOARD_MIN_X
                || occupied.contains(&new_point)
            {
                self.state.players.remove(player);
            } else {
                deque.push_front(new_point);
                if self.state.fruits.contains(&new_point) {
                    // self.spawn_fruit();
                } else {
                    deque.pop_back();
                }
            }
        }

        self.moves.clear();

        Ok(())
    }

    fn record_move(&mut self, player: String, direction: Direction) -> actix_web::Result<()> {
        assert_eq!(self.moves.insert(player, direction), None);

        if self.moves.len() == self.state.players.len() {
            self.time_step()?
        }

        Ok(())
    }
}
