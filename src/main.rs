use raylib::prelude::*;
use chess::*;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 1024;
const RECT_WIDTH: i32 = WINDOW_WIDTH / 8;

const COLOR_EVEN: u32 = 0xebecd0ff;
const COLOR_ODD: u32 = 0x779556ff;
const COLOR_MOVABLE: u32 = 0xcdcdb4ff;
const COLOR_WHITE_SELECTED: u32 = 0xf5f580ff;
const COLOR_BLACK_SELECTED: u32 = 0xb9ca42ff;

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
        if self.white_move { ChessColor::White} else { ChessColor::Black }
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
            },
        };

        Some(Piece {
            t: piece_type,
            color: if white { ChessColor::White } else { ChessColor::Black},
        })
    }
}


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
    let check_sound = audio.new_sound("assets/capture.mp3").unwrap();
    let promote_sound = audio.new_sound("assets/move-check.mp3").unwrap();
    let castle_sound = audio.new_sound("assets/castle.mp3").unwrap();

    let textures = load_textures(&mut rl, &thread);


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

    let mut selected_square: Option<u32> = None;
    let mut moves = board.get_moves();

    while !rl.window_should_close() {

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let maybe_to = select_move(&mut rl, &board, &mut selected_square);

            if maybe_to.is_some() && selected_square.is_some() {
                let from = selected_square.unwrap();
                let to = maybe_to.unwrap();

                let valid = moves.iter().find(|s| move_squares(s) == (from, to));

                if let Some(m) = valid {
                    println!("Our move: {m}");
                    board.make_move(m.to_string());
                }
                moves = board.get_moves();
                for m in &moves {
                    println!("{m}");
                }

                selected_square = None;
            }
        }

        let mut d = rl.begin_drawing(&thread);

        draw_board(&mut d);
        draw_pieces(&mut d, &board, &textures);

        if let Some(s) = selected_square {
            highlight_movable_squares(&mut d, &moves, s);
        }
    }
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


fn highlight_movable_squares(d: &mut RaylibDrawHandle, moves: &[String], selected_square: u32) {
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

fn select_move(
    rl: &mut RaylibHandle,
    b: &ChessBoard,
    selected_square: &mut Option<u32>,
) -> Option<u32> {
    let x = rl.get_mouse_x() / RECT_WIDTH;
    let y = rl.get_mouse_y() / RECT_WIDTH;
    let clicked_square = (y * 8 + x) as u32;

    if selected_square.is_none() {
        *selected_square = Some(clicked_square);
        return None;
    }

    let maybe_piece_on = b.piece_on(clicked_square);

    if maybe_piece_on.is_none() {
        return Some(clicked_square);
    }

    if maybe_piece_on.unwrap().color == b.current_side() {
        *selected_square = Some(clicked_square);
    } else {
        return Some(clicked_square);
    }


    None
}


fn draw_pieces(d: &mut RaylibDrawHandle, board: &ChessBoard, textures: &[Texture2D]) {
    for y in 0..8 {
        for x in 0..8 {
            let curr_piece = board.board[board.board.len() - 1][y][x];
            if curr_piece != '.' {
                //let curr_piece = if curr_piece as u8 == 145 {
                //    (curr_piece as u8 - 2 * 32) as char
                //} else {
                //curr_piece
                //};

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

fn draw_board(d: &mut RaylibDrawHandle) {
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
