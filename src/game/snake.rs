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

const BOARD_MIN_X: i32 = -5;
const BOARD_MAX_X: i32 = 5;
const BOARD_MIN_Y: i32 = -5;
const BOARD_MAX_Y: i32 = 5;

const STARTS: [[Point2D; 3]; NUM_PLAYERS] = [
    [Point2D::new(-3, -3), Point2D::new(-2, -3), Point2D::new(-1, -3)],
    [Point2D::new(-3, 3), Point2D::new(-3, 2), Point2D::new(-3, 1)],
    [Point2D::new(3, -3), Point2D::new(3, -2), Point2D::new(3, -1)],
    [Point2D::new(3, 3), Point2D::new(2, 3), Point2D::new(1, 3)],
];

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

impl Point2D {
    fn random() -> Self {
        Point2D {
            x: thread_rng().gen_range(BOARD_MIN_X..=BOARD_MAX_X),
            y: thread_rng().gen_range(BOARD_MIN_Y..=BOARD_MAX_Y),
        }
    }

    const fn new(x: i32, y: i32) -> Self {
        Point2D { x, y }
    }
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
    world_min: Point2D,
    world_max: Point2D,
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
                    world_min: Point2D {
                        x: BOARD_MIN_X,
                        y: BOARD_MIN_Y,
                    },
                    world_max: Point2D {
                        x: BOARD_MAX_X,
                        y: BOARD_MAX_Y,
                    },
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

        let start = VecDeque::from(STARTS[self.players.len()]);
        self.players.push(username.clone());
        self.game.state.players.insert(username, start);
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

        if self.game.state.players.len() <= 1 {
            self.stage = Stage::Ended;
        }

        self.notifier.send();

        Ok(())
    }

    fn get_stage(&self) -> Stage {
        self.stage
    }

    fn get_encoded_state(&self) -> actix_web::Result<GenericGameState> {
        let all_players = self.game.state.players.keys();
        let can_move = all_players
            .filter(|&x| !self.game.moves.contains_key(x))
            .cloned()
            .collect();

        Ok(GenericGameState {
            players: self.players.clone(),
            stage: self.stage,
            can_move,
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
    fn time_step(&mut self) -> actix_web::Result<()> {
        let mut occupied: HashSet<Point2D> = HashSet::new();

        for (player, _) in self.moves.iter() {
            let deque = self.state.players.get(player).unwrap();
            occupied.extend(deque.iter());
        }

        let mut newly_occupied: HashMap<Point2D, &String> = HashMap::new();
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
                if let Some(collided) = newly_occupied.insert(new_point, player) {
                    self.state.players.remove(collided);
                    self.state.players.remove(player);
                } else {
                    deque.push_front(new_point);
                    if self.state.fruits.contains(&new_point) {
                        self.state.fruits.remove(&new_point);
                    } else {
                        deque.pop_back();
                    }
                }
            }
        }
        occupied.extend(newly_occupied.keys());
        self.moves.clear();

        let fruit_prob = if self.state.fruits.is_empty() {
            0.5
        } else {
            0.15
        };
        if rand::thread_rng().gen_bool(fruit_prob) {
            // Attempt to spawn a single new fruit
            for _ in 0..10 {
                let fruit_pos = Point2D::random();
                if !occupied.contains(&fruit_pos) && !self.state.fruits.contains(&fruit_pos) {
                    self.state.fruits.insert(fruit_pos);
                    break;
                }
            }
        }

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
