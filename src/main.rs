use chess::*;
use chess_networking::{Move, Start};
use network::*;
use raylib::prelude::*;

mod network;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 1024;
const RECT_WIDTH: i32 = WINDOW_WIDTH / 8;

const COLOR_EVEN: u32 = 0xebecd0ff;
const COLOR_ODD: u32 = 0x779556ff;
const COLOR_MOVABLE: u32 = 0xcdcdb4ff;
const COLOR_WHITE_SELECTED: u32 = 0xf5f580ff;
const COLOR_BLACK_SELECTED: u32 = 0xb9ca42ff;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Chess")
        .msaa_4x()
        .log_level(TraceLogLevel::LOG_ERROR)
        .build();

    let audio = match RaylibAudio::init_audio_device() {
        Ok(audio) => audio,
        Err(e) => panic!("{}", e),
    };

    let move_sound = audio.new_sound("assets/move-self.mp3").unwrap();
    let capture_sound = audio.new_sound("assets/capture.mp3").unwrap();
    let promote_sound = audio.new_sound("assets/move-check.mp3").unwrap();

    let textures = load_textures(&mut rl, &thread);
    let args: Vec<String> = std::env::args().collect();

    let is_server = args[1] == "server";
    let mut network: Box<dyn ChessProtocol> = if is_server {
        Box::new(Server::new().unwrap())
    } else {
        Box::new(Client::new().unwrap())
    };

    let our_name = &args[1];
    let desired_start = Start {
        is_white: is_server,
        name: our_name.to_string(),
        fen: None,
        time: None,
        inc: None,
    };

    let start = network.handle_setup(desired_start).unwrap();
    let mut our_turn = start.is_white == false;
    println!("{:?}", start);
    network.set_blocking(false);

    let initial_board: [[char; 8]; 8] = [
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['.', '.', '.', '.', '.', '.', '.', '.'],
        ['.', '.', '.', '.', '.', '.', '.', '.'],
        ['.', '.', '.', '.', '.', '.', '.', '.'],
        ['.', '.', '.', '.', '.', '.', '.', '.'],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
    ];

    let mut board = ChessBoard::new();
    board.board = vec![initial_board];

    let mut move_selector = MoveSelector {
        moves: board.get_moves(),
        selected_square: None,
        promotion_prompt: None,
        promotion_move: None,
    };

    while !rl.window_should_close() {
        let game_state = board.current_gamestate();
        if let Some(m) = network.receive_move().unwrap() {
            println!("{:?}", m);
            let mut move_str = String::new();

            move_str.push(('a' as u8 + m.from.0 as u8) as char);
            move_str.push(('8' as u8 - m.from.1 as u8) as char);
            move_str.push(('a' as u8 + m.to.0 as u8) as char);
            move_str.push(('8' as u8 - m.to.1 as u8) as char);

            board.make_move(move_str);
            move_selector.moves = board.get_moves();
            our_turn = !our_turn;
        }

        if our_turn {
            if let Some(m) = move_selector.on_update(&mut rl) {
                let (from, to) = move_squares(&m);
                let is_capture = board.piece_on(to).is_some();
                let is_promotion = is_promotion(&m);
                let is_quiet = !is_capture && !is_promotion;

                if is_capture {
                    capture_sound.play();
                }
                if is_promotion {
                    promote_sound.play();
                }
                if is_quiet {
                    move_sound.play();
                }

                network
                    .send_move(Move {
                        from: (from as u8 & 7, from as u8 / 8),
                        to: (to as u8 & 7, to as u8 / 8),
                        promotion: None,
                        forfeit: false,
                        ofer_draw: false,
                    })
                    .unwrap();

                board.make_move(m);
                move_selector.moves = board.get_moves();
                our_turn = !our_turn;
            }

            if game_state == GameState::Checkmate || game_state == GameState::Draw {
                if let Some(restart) = Menu::update(&mut rl) {
                    if restart {
                        board = ChessBoard::new();
                        board.board = vec![initial_board];
                        move_selector.moves = board.get_moves();
                    } else {
                        break;
                    }
                }
            }
        }

        let mut d = rl.begin_drawing(&thread);

        draw_board(&mut d);
        match game_state {
            GameState::InProgress => {
                if let Some(s) = move_selector.selected_square {
                    hightlight_current_piece(&mut d, &board, s);
                }
                draw_pieces(&mut d, &board, &textures);

                if let Some(s) = move_selector.selected_square {
                    highlight_movable_squares(&mut d, &move_selector.moves, s);
                }

                if let Some(p) = &move_selector.promotion_prompt {
                    p.draw(&mut d, &textures, board.current_side());
                }
            }
            GameState::Checkmate => Menu::draw(&mut d, &board, &textures, "Checkmate"),
            GameState::Draw => Menu::draw(&mut d, &board, &textures, "Draw"),
        };

        d.draw_text(&our_name, 10, 10, 48, Color::CORNFLOWERBLUE);
        d.draw_text(
            &start.name,
            10,
            WINDOW_HEIGHT - 10 - 48,
            48,
            Color::CORNFLOWERBLUE,
        );
    }
}

struct Menu;

impl Menu {
    const BUTTON_WIDTH: f32 = 300.0;
    const BUTTON_HEIGHT: f32 = 100.0;
    const BUTTON_PAD: f32 = 50.0;
    const BUTTON_X: f32 = WINDOW_WIDTH as f32 / 2.0 - Self::BUTTON_WIDTH / 2.0;
    const BUTTON_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;
    const BUTTON_DIFF: f32 = Self::BUTTON_HEIGHT - Self::BUTTON_PAD / 2.0;

    fn update(rl: &mut RaylibHandle) -> Option<bool> {
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            for mul in [-1, 1] {
                let r = Rectangle::new(
                    Self::BUTTON_X,
                    Self::BUTTON_Y + mul as f32 * Self::BUTTON_DIFF,
                    Self::BUTTON_WIDTH,
                    Self::BUTTON_HEIGHT,
                );

                if r.check_collision_point_rec(rl.get_mouse_position()) {
                    if mul == -1 {
                        return Some(true);
                    } else {
                        return Some(false);
                    }
                }
            }
        }

        None
    }

    fn draw(
        d: &mut RaylibDrawHandle,
        board: &ChessBoard,
        textures: &[Texture2D],
        result_text: &str,
    ) {
        draw_pieces(d, &board, &textures);
        d.draw_rectangle(
            0,
            0,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Color::get_color(0x00_00_00_55),
        );

        let x = Self::BUTTON_X;
        let y = Self::BUTTON_Y;
        let diff = Self::BUTTON_DIFF;

        d.draw_rectangle_rounded(
            Rectangle::new(x, y - diff, Self::BUTTON_WIDTH, Self::BUTTON_HEIGHT),
            0.5,
            15,
            Color::RAYWHITE,
        );

        d.draw_rectangle_rounded(
            Rectangle::new(x, y + diff, Self::BUTTON_WIDTH, Self::BUTTON_HEIGHT),
            0.5,
            15,
            Color::RAYWHITE,
        );

        let length = d.measure_text("Restart", 48);
        let x_offset = (Self::BUTTON_WIDTH - length as f32) / 2.0;
        // Hardcoded since the bindings don't support MeasureTextEx which also returns height.
        let y_offset = 25.0;

        d.draw_text(
            "Restart",
            (x + x_offset) as i32,
            (y - diff + y_offset) as i32,
            48,
            Color::BLACK,
        );

        let length = d.measure_text("Quit", 48);
        let x_offset = (Self::BUTTON_WIDTH - length as f32) / 2.0;
        d.draw_text(
            "Quit",
            (x + x_offset) as i32,
            (y + diff + y_offset) as i32,
            48,
            Color::BLACK,
        );

        let length = d.measure_text(result_text, 72);
        let x = WINDOW_WIDTH as f32 / 2.0 - length as f32 / 2.0;
        d.draw_text(result_text, x as i32, y as i32 - 300, 72, Color::PURPLE);
    }
}

struct MoveSelector {
    selected_square: Option<u32>,
    moves: Vec<String>,
    promotion_prompt: Option<PromotionUI>,
    promotion_move: Option<String>,
}

impl MoveSelector {
    fn on_update(&mut self, rl: &mut RaylibHandle) -> Option<String> {
        let x = rl.get_mouse_x();
        let y = rl.get_mouse_y();
        let clicked_square = ((y / RECT_WIDTH) * 8 + x / RECT_WIDTH) as u32;

        if let Some(m) = &self.promotion_move {
            if let Some(c) = self
                .promotion_prompt
                .as_mut()
                .unwrap()
                .update(rl, x as f32, y as f32)
            {
                let mut clone = m.clone();
                clone.pop();
                clone.push(c);

                self.promotion_move = None;
                self.promotion_prompt = None;
                self.selected_square = None;

                return Some(clone);
            }

            return None;
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if self.selected_square.is_none() {
                self.selected_square = Some(clicked_square);
                return None;
            }

            let from = self.selected_square.unwrap();
            let to = clicked_square;

            let valid_move = self.moves.iter().find(|s| move_squares(s) == (from, to));
            if valid_move.is_none() {
                self.selected_square = None;
                return None;
            }

            let m = valid_move.unwrap().clone();
            if is_promotion(&m) {
                self.promotion_move = Some(m);
                self.promotion_prompt = Some(PromotionUI::new(x as f32, y as f32));
                return None;
            }

            self.selected_square = None;

            return Some(m);
        }

        None
    }
}

struct PromotionUI {
    x: f32,
    y: f32,

    piece_rects: [Rectangle; 4],
}

impl PromotionUI {
    const HEIGHT_PAD: f32 = 10.0;
    const EDGE_PAD: f32 = 25.0;
    const PIECE_RECT_SIZE: f32 = RECT_WIDTH as f32;
    const WIDTH: f32 = Self::PIECE_RECT_SIZE * 4.0 + 2.0 * Self::EDGE_PAD;
    const HEIGHT: f32 = Self::PIECE_RECT_SIZE + 2.0 * Self::HEIGHT_PAD;

    fn new(mut x: f32, mut y: f32) -> PromotionUI {
        if y + Self::HEIGHT > WINDOW_HEIGHT as f32 {
            y -= (y + Self::HEIGHT) - WINDOW_HEIGHT as f32;
        }

        if x + Self::WIDTH > WINDOW_WIDTH as f32 {
            x -= (x + Self::WIDTH) - WINDOW_WIDTH as f32;
        }

        let mut piece_rects = [Rectangle::EMPTY; 4];

        for idx in 0..4 {
            let x = x + idx as f32 * Self::PIECE_RECT_SIZE + Self::EDGE_PAD;
            let y = y + Self::HEIGHT_PAD;

            piece_rects[idx] = Rectangle::new(x, y, RECT_WIDTH as f32, RECT_WIDTH as f32);
        }

        PromotionUI { x, y, piece_rects }
    }

    fn update(&self, rl: &mut RaylibHandle, x: f32, y: f32) -> Option<char> {
        if !rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            return None;
        };

        for idx in 0..4 {
            if self.piece_rects[idx].check_collision_point_rec(Vector2::new(x, y)) {
                return Some(match idx {
                    0 => 'n',
                    1 => 'b',
                    2 => 'r',
                    3 => 'q',
                    _ => unreachable!(),
                });
            }
        }

        None
    }

    fn draw(&self, d: &mut impl RaylibDraw, textures: &[Texture2D], color: ChessColor) {
        d.draw_rectangle_rounded(
            Rectangle::new(self.x, self.y, Self::WIDTH, Self::HEIGHT),
            0.5,
            15,
            Color::RAYWHITE,
        );

        for idx in 0..4 {
            let texture_index = idx + 6 * color as usize;
            let texture = &textures[texture_index + 1];
            let x = self.x + idx as f32 * Self::PIECE_RECT_SIZE + Self::EDGE_PAD;
            let y = self.y + Self::HEIGHT_PAD;

            d.draw_texture_pro(
                texture,
                Rectangle::new(0.0, 0.0, texture.width() as f32, texture.height() as f32),
                Rectangle::new(x, y, RECT_WIDTH as f32, RECT_WIDTH as f32),
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }
    }
}

fn is_promotion(m: &str) -> bool {
    m.len() > 4 && m.chars().nth(4).unwrap() != 'e'
}

fn move_squares(s: &str) -> (u32, u32) {
    (square(&s[0..2]), square(&s[2..4]))
}

fn square(s: &str) -> u32 {
    let mut chars = s.chars();
    let x0 = chars.next().unwrap() as u32 - 97;
    let y0 = 8 - chars.next().unwrap().to_digit(10).unwrap();

    y0 * 8 + x0
}

fn hightlight_current_piece(d: &mut impl RaylibDraw, board: &ChessBoard, square: u32) {
    let color = if board.current_side() == ChessColor::White {
        Color::get_color(COLOR_WHITE_SELECTED)
    } else {
        Color::get_color(COLOR_BLACK_SELECTED)
    };

    let x = (square & 7) as i32;
    let y = (square / 8) as i32;

    d.draw_rectangle(
        x * RECT_WIDTH,
        y * RECT_WIDTH,
        RECT_WIDTH,
        RECT_WIDTH,
        color,
    );
}

fn highlight_movable_squares(d: &mut impl RaylibDraw, moves: &[String], selected_square: u32) {
    for m in moves {
        let (from, to) = move_squares(m);
        if from == selected_square {
            let x = to % 8;
            let y = to / 8;
            let center_x = x as i32 * RECT_WIDTH + RECT_WIDTH / 2;
            let center_y = y as i32 * RECT_WIDTH + RECT_WIDTH / 2;

            d.draw_circle(center_x, center_y, 24.0, Color::get_color(COLOR_MOVABLE));
        }
    }
}

fn draw_pieces(d: &mut impl RaylibDraw, board: &ChessBoard, textures: &[Texture2D]) {
    for y in 0..8 {
        for x in 0..8 {
            let curr_piece = board.board[board.board.len() - 1][y][x];
            if curr_piece != '.' {
                let color = !curr_piece.is_uppercase();
                let piece_type = match curr_piece.to_ascii_lowercase() {
                    'p' => 0,
                    'n' => 1,
                    'b' => 2,
                    'r' => 3,
                    'q' => 4,
                    'k' => 5,
                    _ => panic!("Invalid piece: {}", curr_piece.to_ascii_lowercase() as u8),
                };

                let texture_index = piece_type + 6 * color as usize;
                let texture = &textures[texture_index];

                let x = x as i32 * RECT_WIDTH;
                let y = y as i32 * RECT_WIDTH;

                d.draw_texture_pro(
                    texture,
                    Rectangle::new(0.0, 0.0, texture.width() as f32, texture.height() as f32),
                    Rectangle::new(x as f32, y as f32, RECT_WIDTH as f32, RECT_WIDTH as f32),
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            }
        }
    }
}

fn draw_board(d: &mut impl RaylibDraw) {
    for y in 0..8 {
        for x in 0..8 {
            let color = if (x + y) % 2 == 0 {
                Color::get_color(COLOR_EVEN)
            } else {
                Color::get_color(COLOR_ODD)
            };

            d.draw_rectangle(
                x * RECT_WIDTH,
                y * RECT_WIDTH,
                RECT_WIDTH,
                RECT_WIDTH,
                color,
            );
        }
    }
}

fn load_textures(rl: &mut RaylibHandle, thread: &RaylibThread) -> Vec<Texture2D> {
    let mut textures = Vec::new();
    const NUM_PIECES: u32 = 6;

    for idx in 0..NUM_PIECES * 2 {
        let texture = match rl.load_texture(&thread, format!("assets/{}.png", idx).as_str()) {
            Ok(texture) => texture,
            Err(msg) => panic!("{}", msg),
        };

        textures.push(texture);
    }

    textures
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChessColor {
    White,
    Black,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    t: PieceType,
    color: ChessColor,
}

trait BoardExtensions {
    fn piece_on(&self, square: u32) -> Option<Piece>;
    fn current_side(&self) -> ChessColor;
}

impl BoardExtensions for ChessBoard {
    fn piece_on(&self, square: u32) -> Option<Piece> {
        Piece::from(self.board[self.board.len() - 1][square as usize / 8][square as usize & 7])
    }

    fn current_side(&self) -> ChessColor {
        if self.white_move {
            ChessColor::White
        } else {
            ChessColor::Black
        }
    }
}

impl Piece {
    fn from(s: char) -> Option<Piece> {
        let white = s.is_uppercase();
        let piece_type = match s.to_ascii_lowercase() {
            'p' => PieceType::Pawn,
            'n' => PieceType::Knight,
            'b' => PieceType::Bishop,
            'r' => PieceType::Rook,
            'q' => PieceType::Queen,
            'k' => PieceType::King,
            '.' => return None,
            _ => {
                println!("Should panic");
                return None;
            }
        };

        Some(Piece {
            t: piece_type,
            color: if white {
                ChessColor::White
            } else {
                ChessColor::Black
            },
        })
    }
}
