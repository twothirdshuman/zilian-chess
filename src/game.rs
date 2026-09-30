use crate::board::{Board, Square};
use crate::pieces::{ChessPiece, PieceType};
use crate::moves::Move;

// Improved error handling for the API
#[derive(Debug, PartialEq, Eq)]
pub enum MoveError {
    InvalidNotation, // Notation wrong, like x9
    EmptySquare,     // Tried to move a empty square
    NotYourPiece,    // Tried to move enemy piece
    CantSelfHarm,    // Tried to attack own piece
    IllegalMove,     // Tried an illegal move
    HangsKing,       // Tried a move that self-checks
    InvalidPromotion // Can't promote to specific PieceType
}

#[derive(Clone)]
pub struct ChessGame {
    // Current board layout
    board_state: Board,
    // Whos turn it is
    is_white_turn: bool,
    // 
    movement_logic: Move,
}

impl ChessGame {

    // Starts a game from a given board and player turn.
    pub fn new_game(board_state: Board, is_white_turn: bool) -> Self {
        Self {
            board_state,
            is_white_turn,
            movement_logic: Move::new_move(),
        }
    }

    // Starts a new game with standard layout and whites turn
    pub fn new_standard_game() -> Self {
        Self {
            board_state: Board::new_starting_board(),
            is_white_turn: true,
            movement_logic: Move::new_move(),
        }
    }

    // Get's current board.
    pub fn board(&self) -> &Board {
        &self.board_state
    }

    // Get who's turn it is
    pub fn is_white_turn(&self) -> bool {
        self.is_white_turn
    }

    // Turn text
    pub fn turn_text(&self) -> &str {
        if self.is_white_turn() {
            return "White's turn";
        }
        else {
            return "Black's turn";
        }
    }

    // Current player check
    pub fn in_check(&self) -> bool {
        self.is_in_check(&self.board_state, self.is_white_turn)
    }

    // Current player checkmate
    pub fn in_checkmate(&self) -> bool {
        self.is_checkmate(&self.board_state, self.is_white_turn)
    }

    // Current player stalemate
    pub fn in_stalemate(&self) -> bool {
        self.is_stalemate(&self.board_state, self.is_white_turn)
    }

    // Tie by fifty-move-rule
    pub fn in_tie(&self) -> bool {
        return self.movement_logic.get_fifty_increment() >= 100;
    }

    pub fn make_move_notation(&mut self, start: &str, stop: &str) -> Result<(), MoveError> {
        self.make_move_internal(
            Square::new_square_from_notation_str(&start),
            Square::new_square_from_notation_str(&stop),
            None,
        )
    }

    // Same as make_move_notation, but for after MoveError: InvalidPromotion
    pub fn make_promotion_notation(&mut self, start: &str, stop: &str, promotion_choice: PieceType) -> Result<(), MoveError> {
        self.make_move_internal(
            Square::new_square_from_notation_str(&start),
            Square::new_square_from_notation_str(&stop),
            Some(promotion_choice),
        )
    }

    // Make move with squares (assumes no promotion)
    pub fn make_move(&mut self, start: Option<Square>, stop: Option<Square>) -> Result<(), MoveError> {
        self.make_move_internal(start, stop, None)
    }

    // Promotion move (after make_move returns MoveError: InvalidPromotion)
    pub fn make_promotion_move(&mut self, start: Option<Square>, stop: Option<Square>, promotion_choice: PieceType) -> Result<(), MoveError> {
        self.make_move_internal(start, stop, Some(promotion_choice))
    }

    // Gets a start and stop square
    // Runs checks
    // Prints relevant error depending on where move fails/ why is invalid
    fn make_move_internal(&mut self, start: Option<Square>, stop: Option<Square>, promotion_choice: Option<PieceType>) -> Result<(), MoveError> {

        // For clearing en_passant
        let mut passant_clearing = true;
        // Checks that start square is valid
        let start_square = match start {
            Some(square) => square,
            None => return Err(MoveError::InvalidNotation),
        };
        // Checks that stop square is valid
        let stop_square = match stop {
            Some(square) => square,
            None => return Err(MoveError::InvalidNotation),
        };

        // Checks that we aren't trying to move an empty square
        let mut piece = match self.board_state.get_piece_square(&start_square) {
            Some(piece) => piece,
            None => return Err(MoveError::EmptySquare),
        };

        // Checks that the piece being moved is the same as the color of who's turn it is
        if piece.is_white() != self.is_white_turn {
            return Err(MoveError::NotYourPiece);
        }

        // Checks that we aren't trying to capture own piece
        if self.board_state.is_same_color(&start_square, &stop_square) {
            return Err(MoveError::CantSelfHarm);
        }

        // If move passes the movement logic check
        if !self.movement_logic.check_move(&self.board_state, &start_square, &stop_square) {
            return Err(MoveError::IllegalMove);
        }

        // Check that we aren't exposing king
        if self.hangs_king(&self.board_state, &start_square, &stop_square, self.is_white_turn) {
            return Err(MoveError::HangsKing);
        }

        // FROM HERE ON
        // Since the move is valid by the previous checks we can check how it would effect the fifty move rule
        self.movement_logic.fifty_handler(&self.board_state, &start_square, &stop_square);

        // If the move is a king-castling move
        if self.movement_logic.is_valid_castle(&self.board_state, &start_square, &stop_square) {
            // Determines where the rook is (king/queen-side)
            let step: i8 = if stop_square.file() > start_square.file() { 1 } else { -1 };
            let rook_file = if step > 0 { 7 } else { 0 };

            let rook_start = Square::new_square_from_index(rook_file, start_square.rank()).unwrap();
            let rook_stop = Square::new_square_from_index(stop_square.file() - step, start_square.rank()).unwrap();

            let mut rook = self.board_state.get_piece_square(&rook_start).unwrap();
            rook.has_moved_true();
            self.board_state.set_piece_square(&rook_start, Some(rook));
            self.board_state.move_piece_square(&rook_start, &rook_stop);
        }

        // If the move is a double pawn push, setting up en passant for next move
        else if self.movement_logic.is_valid_en_passant_setup(&self.board_state, &start_square, &stop_square) {
            // Set the en_passant square to the square behind where the move stops at
            let direction: i8 = if piece.is_white() { 1 } else { -1 };
            let passant_square = Square::new_square_from_index(start_square.file(), start_square.rank() + direction).unwrap();
            self.movement_logic.set_en_passant_square(Some(passant_square));
            passant_clearing = false;
        }

        // If the move is a en_passant capture move
        else if self.movement_logic.is_valid_en_passant_capture(&self.board_state, &start_square, &stop_square) {
            // Removes the "captured pawn" since its not on stop_square
            let captured_square = Square::new_square_from_index(stop_square.file(), start_square.rank()).unwrap();
            self.board_state.set_piece_square(&captured_square, None);
            self.movement_logic.set_en_passant_square(None);
        }

        // If the move is a pawn promotion
        else if self.movement_logic.is_valid_promotion(&self.board_state, &start_square, &stop_square) {
            // Determine what pawn should be promoted to
            if promotion_choice == Some(PieceType::Queen)
            || promotion_choice == Some(PieceType::Rook)
            || promotion_choice == Some(PieceType::Bishop)
            || promotion_choice == Some(PieceType::Knight) {
                // Swaps the pawn for the chosen piece; the normal move below
                // then places it on stop_square like any other move.
                piece = ChessPiece::new(promotion_choice.unwrap(), piece.is_white(), false).unwrap();
            }
            else {
                // If no promotion is supplied/invalid
                // Get error and can then prompt user for promotion piece
                return Err(MoveError::InvalidPromotion);
            }
        }

        // Since piece is allowed to move to stop square, and we took care of special cases, we make move as normal
        piece.has_moved_true();
        self.board_state.set_piece_square(&start_square, Some(piece));  // Set start_square to same piece but moved
        self.board_state.move_piece_square(&start_square, &stop_square);            // Actually moves the piece
        // Change whose turn it is
        if passant_clearing {
            self.movement_logic.set_en_passant_square(None);
        }
        self.is_white_turn = !self.is_white_turn;

        return Ok(());
    }

    // Find square with king of given color
    fn find_king(board: &Board, is_white: bool) -> Option<Square> {
        for file in 0..8 {
            for rank in 0..8 {
                match board.get_piece_file_rank(file, rank) {
                    Some(piece) 
                    => if piece.piece_type() == PieceType::King && piece.is_white() == is_white {
                        return Square::new_square_from_index(file, rank);
                    },
                    None => {}
                }
            }
        }
        return  None;
    }

    // Given a square and a color
    // Find out if the enemy is attacking that square (has LOS)
    fn is_attacked(&self,board: &Board, attacked_square: &Square, is_white: bool) -> bool {
        for file in 0..8 {
            for rank in 0..8 {
                // The square we want to see if it has LOS to attacked_square
                let checking_square = Square::new_square_from_index(file, rank).unwrap();

                match board.get_piece_file_rank(file, rank) {
                    Some(piece) 
                    => if piece.is_white() == is_white {
                        continue; // Skip if same color
                    },
                    None => continue, // Skip if empty
                }
                if self.movement_logic.check_move(board, &checking_square, attacked_square) {
                    return true;
                }
            }
        }
        return  false;
    }

    // Combine two previous to find out if in check
    fn is_in_check(&self, board: &Board, is_white: bool) -> bool {
        match Self::find_king(board, is_white) {
            Some(king_square) =>
            return self.is_attacked(board, &king_square, is_white),
            None => return false,
        }
    }

    // Checks if move from square to square would expose king
    fn hangs_king(&self, board: &Board, start: &Square, stop: &Square, is_white: bool) -> bool {
        let mut test_board = board.clone();
        test_board.move_piece_square(start, stop);
        self.is_in_check(&test_board, is_white)
    }

    // Checks if there are any legal moves
    // If not then game over
    fn has_legal_moves(&self, board: &Board, is_white: bool) -> bool {
        // Can lowkirkenuaneliey copy is_attacked
        for file in 0..8 {
            for rank in 0..8 {
                // The square we want to see if it has LOS to attacked_square
                let checking_square = Square::new_square_from_index(file, rank).unwrap();

                match board.get_piece_file_rank(file, rank) {
                    Some(piece) 
                    => if piece.is_white() == !is_white {
                        continue; // If checking_piece isn't same color as who's turn it is
                    },
                    None => continue, // Skip if empty
                }
                for dest_file in 0..8 {
                    for dest_rank in 0..8 {
                        let destination_square = Square::new_square_from_index(dest_file, dest_rank).unwrap();
                        // If there is a move from checking_square to destination_square
                        if self.movement_logic.check_move(board, &checking_square, &destination_square) {
                            if !self.hangs_king(board, &checking_square, &destination_square, is_white) {
                                return true;
                            }
                        }
                    }
                } // Holy nesting!!! I would make less awful if i had the time
            }
        }
        return  false;
    }

    // In check with no legal moves
    fn is_checkmate(&self, board: &Board, is_white: bool) -> bool {
        self.is_in_check(board, is_white) && !self.has_legal_moves(board, is_white)
    }

    // Not in check but no legal moves
    fn is_stalemate(&self, board: &Board, is_white: bool) -> bool {
        !self.is_in_check(board, is_white) && !self.has_legal_moves(board, is_white)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_pieces() {
        // Try to see if we can move a pinned piece
        let board = Board::board_from_strings([
        "....r...",
        "........",
        "........",
        "........",
        "........",
        "........",
        "....B...",
        "....K...",
        ]);

        let mut game = ChessGame::new_game(board, true);
        assert!(!game.make_move_notation("e2", "d3").is_ok());
    }

    #[test]
    fn checkmate_detection() {
        // Try to see if checkmate is detected
        let board = Board::board_from_strings([
        "........",
        "........",
        "........",
        "........",
        "........",
        "........",
        ".....PPP",
        "r......K",
        ]);
        let test_game = ChessGame::new_game(board, true);
        assert!(test_game.is_checkmate(&board, true));
        assert!(!test_game.is_stalemate(&board, true));
    }

    #[test]
    fn stalemate_detection() {
        // Try to see if stalemate is detected
        let board = Board::board_from_strings([
        "k.......",
        "..Q.....",
        "........",
        "........",
        "..K.....",
        "........",
        "........",
        "........",
        ]);
        let test_game = ChessGame::new_game(board, true);
        assert!(test_game.is_stalemate(&board, false));
        assert!(!test_game.is_checkmate(&board, false));
    }

    #[test]
    fn en_passant() {
        // Try to see if en_passant capture works
        let board = Board::board_from_strings([
        "........",
        "........",
        "........",
        "........",
        "...p....",
        "........",
        "....P...",
        "........",
        ]);
        let mut game = ChessGame::new_game(board, true);
        assert!(game.make_move_notation("e2", "e4").is_ok());
        assert!(game.make_move_notation("d4", "e3").is_ok());
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("e3").unwrap()).is_some());
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("e4").unwrap()).is_none());
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("d4").unwrap()).is_none());
    }

    #[test]
    fn fifty_move_rule_tie() {
        // Moves knight back and forth 50 times to see if counts as tie
        let board = Board::board_from_strings([
        ".n......",
        "........",
        "........",
        "........",
        "........",
        "........",
        "........",
        ".N......",
        ]);
        let mut game = ChessGame::new_game(board, true);
        assert!(!game.in_tie());

        // Makes 49 moves aka 98 turns
        let mut forward_turn = true;
        for _ in 0..49 {
            let (white_from, white_to) = if forward_turn { ("b1", "c3") } else { ("c3", "b1") };
            let (black_from, black_to) = if forward_turn { ("b8", "c6") } else { ("c6", "b8") };

            assert!(game.make_move_notation(white_from, white_to).is_ok());
            assert!(game.make_move_notation(black_from, black_to).is_ok());

            forward_turn = !forward_turn;
        }
        // Checks that game is not in tie after 49 moves
        assert!(!game.in_tie());

        // Makes 50th move
        let (white_from, white_to) = if forward_turn { ("b1", "c3") } else { ("c3", "b1") };
        let (black_from, black_to) = if forward_turn { ("b8", "c6") } else { ("c6", "b8") };

        assert!(game.make_move_notation(white_from, white_to).is_ok());
        assert!(game.make_move_notation(black_from, black_to).is_ok());

        // Checks that game IS tie after 50 moves
        assert!(game.in_tie());
    }

    #[test]
    fn castle_kingside() {
        // Try to see if kingside castling works
        let board = Board::board_from_strings([
        "....k..r",
        "........",
        "......R.",
        "........",
        "........",
        "........",
        "........",
        "....K..R",
        ]);
        let mut game = ChessGame::new_game(board, true);
        // Moves are valid
        assert!(game.make_move_notation("e1", "g1").is_ok());
        assert!(!game.make_move_notation("e8", "g8").is_ok());
        
        // Makes castling move
        let _ = game.make_move_notation("e1", "g1");
        let _ = game.make_move_notation("e8", "g8");

        // White
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("g1").unwrap()).is_some()); // King 
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("f1").unwrap()).is_some()); // Rook
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("e1").unwrap()).is_none()); // Empty
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("h1").unwrap()).is_none()); // Empty
        // Black
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("g8").unwrap()).is_some()); // King 
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("f8").unwrap()).is_some()); // Rook
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("e8").unwrap()).is_none()); // Empty
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("h8").unwrap()).is_none()); // Empty
    }

    #[test]
    fn castle_queenside() {
        // Try to see if queenside castling works
        let board = Board::board_from_strings([
        "r...k...",
        "........",
        "........",
        "........",
        "........",
        "..r.....",
        "........",
        "R...K...",
        ]);
        let mut game = ChessGame::new_game(board, false);
        // Moves are valid
        assert!(game.make_move_notation("e8", "c8").is_ok());
        assert!(!game.make_move_notation("e1", "c1").is_ok());

        // Makes castling move
        let _ = game.make_move_notation("e8", "c8");
        let _ = game.make_move_notation("e1", "c1");

        // Black
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("c8").unwrap()).is_some()); // King
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("d8").unwrap()).is_some()); // Rook
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("e8").unwrap()).is_none()); // Empty
        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("a8").unwrap()).is_none()); // Empty
        // White
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("c1").unwrap()).is_some()); // King
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("d1").unwrap()).is_some()); // Rook
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("e1").unwrap()).is_none()); // Empty
        assert!(!game.board().get_piece_square(&Square::new_square_from_notation_str("a1").unwrap()).is_none()); // Empty
    }

    #[test]
    fn white_queen_promotion() {
        // Try to see if pawn promotion works
        let board = Board::board_from_strings([
        "........",
        "....P...",
        "........",
        "........",
        "........",
        "........",
        "........",
        "........",
        ]);
        let mut game = ChessGame::new_game(board, true);
        // Can't make promotion move if promotion piece is not given
        assert!(!game.make_move_notation("e7", "e8").is_ok());
        // Works if promotion given
        assert!(game.make_promotion_notation("e7", "e8", PieceType::Queen).is_ok());

        assert!(game.board().get_piece_square(&Square::new_square_from_notation_str("e7").unwrap()).is_none()); // Empty
        assert_eq!(game.board().get_piece_file_rank(4, 7).unwrap().piece_type(), PieceType::Queen); // Queen
        assert!(game.board().get_piece_file_rank(4, 7).unwrap().is_white()); // Same color
    }

    #[test]
    fn black_knight_promotion() {
        // Try to see if pawn promotion works
        let board = Board::board_from_strings([
        "........",
        "........",
        "........",
        "........",
        "........",
        "........",
        "....p...",
        "........",
        ]);
        let mut game = ChessGame::new_game(board, false);
        // Can't make promotion move if promotion piece is not given
        assert!(!game.make_move_notation("e2", "e1").is_ok());
        // Works if promotion given
        assert!(game.make_promotion_notation("e2", "e1", PieceType::Knight).is_ok());

        assert!(game.board().get_piece_file_rank(4, 1).is_none()); // Empty
        assert_eq!(game.board().get_piece_file_rank(4, 0).unwrap().piece_type(), PieceType::Knight); // Knight
        assert!(!game.board().get_piece_file_rank(4, 0).unwrap().is_white()); // Same color
    }

    #[test]
    fn illegal_promotion() {
        let mut game = ChessGame::new_standard_game();

        game.make_move_notation("g1", "h3").unwrap();
        game.make_move_notation("e7", "e5").unwrap();
        let res = game.make_promotion_move(Some(Square::new_square_from_notation_str("h3").unwrap()), Some(Square::new_square_from_notation_str("g5").unwrap()), PieceType::Queen);

        match res {
            Ok(()) => panic!("THIS MOVE NOT OKAY"),
            Err(_) => ()
        };

    }
}