use async_process::{ChildStdin, ChildStdout, Command, Stdio};
use futures_lite::{future::block_on, io::BufReader, prelude::*};
use std::process::exit;

mod template;

// Lines of output to be read from the child process, assume 6 for board, one for prompt
const LINES_OF_OUTPUT: usize = 7;

// Offset for how many lines of output are printed before the board
const UNIVERSAL_OFFSET: usize = 0;

const PLAYER_1: &'static str = "R";
const PLAYER_2: &'static str = "Y";

const P1_WIN_TEST_MOVES_1: [usize; 12] = [5, 4, 4, 3, 1, 2, 1, 4, 1, 4, 1, 4];
const P1_WIN_TEST_MOVES_2: [usize; 31] = [
    4, 4, 5, 3, 3, 5, 5, 4, 6, 7, 6, 6, 4, 5, 7, 4, 5, 3, 3, 3, 3, 2, 4, 7, 5, 7, 6, 7, 7, 6, 6,
];
const P2_WIN_TEST_MOVES_1: [usize; 34] = [
    4, 4, 4, 5, 5, 5, 6, 3, 4, 6, 5, 3, 3, 3, 5, 4, 4, 5, 7, 7, 7, 7, 1, 1, 1, 1, 3, 1, 7, 7, 3, 1,
    6, 6,
];
const P2_WIN_TEST_MOVES_2: [usize; 16] = [4, 4, 3, 3, 6, 5, 7, 3, 5, 2, 6, 2, 1, 2, 7, 2];
const P2_WIN_TEST_MOVES_3: [usize; 34] = [
    4, 4, 4, 5, 5, 5, 6, 3, 4, 6, 5, 3, 3, 3, 5, 4, 4, 5, 7, 7, 7, 7, 1, 1, 1, 1, 3, 1, 7, 7, 3, 1,
    6, 6,
];
const DRAW_TEST_1: [usize; 42] = [
    4, 5, 3, 4, 2, 1, 4, 3, 5, 6, 2, 2, 3, 3, 5, 4, 5, 5, 4, 4, 2, 1, 7, 7, 6, 6, 6, 6, 1, 3, 3, 1,
    1, 7, 7, 2, 7, 5, 6, 7, 2, 1,
];

fn main() {
    // attempts to compile the connect4 program, on the event of failure, logs the error and
    // provides the score to input
    let Ok(_) = Command::new("javac").arg("connect4.java").spawn() else {
        println!("Failed to compile");

        // if the program fails to compile, the score should be 0;
        println!("score = 0");
        exit(0);
    };

    // Creates a "Future" object which contains the async code. This code is not run until it is
    // provided with an async context

    let mut scores = [0; 7];

    // let fmoves = test_moves(&mut scores[0], /* unimplemented */);
    if let [ref mut score0, ref mut score1, ref mut score2, ref mut score3, ref mut score4, ref mut score5, ref mut score6] =
        scores
    {
        // let fmoves_test = test_moves(score0, );
        // TODO: Refactor to support more words: drew, win, etc
        let fp1w1 = test_outcome(score1, P1_WIN_TEST_MOVES_1, ["win", "won"]);
        let fp1w2 = test_outcome(score2, P1_WIN_TEST_MOVES_2, ["win", "won"]);
        let fp2w1 = test_outcome(score3, P2_WIN_TEST_MOVES_1, ["won", "win"]);
        let fp2w2 = test_outcome(score4, P2_WIN_TEST_MOVES_2, ["won", "win"]);
        // Removed one test case for an even 5 cases
        // let fp2w3 = test_outcome(score6, P2_WIN_TEST_MOVES_3, ["win", "won"]);
        let fd1 = test_outcome(score5, DRAW_TEST_1, ["draw", "drew"]);

        // This blocks the current thread, so only the async thread runs
        block_on(fp1w1);
        block_on(fp1w2);
        block_on(fp2w1);
        block_on(fp2w2);
        // Removed one test case for an even 5 cases
        // block_on(fp2w3);
        block_on(fd1);

        // scores can be between 0 and 5 based on the number of test cases that are passed
        // multiply score by 2 for total 
        dbg!(scores.iter().sum::<usize>() * 2);

    }
}

// This is the bulk of the logic, it tests a certain amount of moves around an outcome 
// moves are defined as a constant above, and output tests is the "win, won" arrays to allow us to
// check if the message is correct
async fn test_outcome<const N: usize, const M: usize>(
    score: &mut usize,
    input: [usize; N],
    output_tests: [&'static str; M]
) {
    // Board for testing
    let mut connect4_board = [[" "; 7]; 6];

    // This starts the child process(connect 4 with java), and sets up pipes to the process's
    // standard in and standard out
    let (mut stdin, mut stdout_reader) = set_up_process().await;

    // The loop continuously sends input to standard in, and reads the standard output
    // It assumes a constant input pattern eg "Input the column you want to place your piece in"
    // and then printing out the board
    // for a non constant input pattern, you can hardcode some statements before the loop
    for i in 0..N {
        // Reads the output from the child process: reads exactly 7 lines, 6 for the board, 1 for
        // the input prompt, must change if the output pattern changes
        // Must read before writing, because the program prompts before asking for input
        let mut buffer: [String; LINES_OF_OUTPUT] = Default::default();
        for output_line in 0..LINES_OF_OUTPUT {
            stdout_reader.read_line(&mut buffer[output_line]).await;
        }

        assert!(matches(&buffer, &connect4_board));
        if within(output_tests, buffer[LINES_OF_OUTPUT - 1].clone()) || within(output_tests, buffer[0].clone()) {
            *score = 1;
        }
        // Writes "i" to stdin of the child process
        stdin.write_all(format!("{}\n", input[i]).as_bytes()).await;
        drop_piece(
            &mut connect4_board,
            input[i],
            if i % 2 == 0 { PLAYER_1 } else { PLAYER_2 },
        );
    }
    let mut buffer: [String; LINES_OF_OUTPUT] = Default::default();
    for output_line in 0..LINES_OF_OUTPUT {
        stdout_reader.read_line(&mut buffer[output_line]).await;
    }

    if within(output_tests, buffer[LINES_OF_OUTPUT - 1].clone()) || within(output_tests, buffer[0].clone()) {
        *score = 1;
    }
}

fn within<const N: usize>(tests: [&str; N], on: String) -> bool {
    for test in tests {
        if on.to_lowercase().contains(test) {
            return true;
        }
    }
    false
}

async fn set_up_process() -> (ChildStdin, BufReader<ChildStdout>) {
    // Spawns the connect 4 child process with pipes to the programs standard io
    // Stdio::piped() sets up a connection between the two program's standard io through a kernel
    // buffer without the need for a file system. It functions as a queue, you put things in on one
    // end, and the child takes them out the other, where the queue is held in memory by the kernel
    let mut connect_4_process = Command::new("java")
        .arg("connect4")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run the connect 4 program");

    // This sets up a writer to write bytes to the program's standard in
    let stdin = connect_4_process
        .stdin
        .take()
        .expect("Failed to open stdin");

    // sets up a reader to read from the program's standard out
    let stdout_reader = BufReader::new(
        connect_4_process
            .stdout
            .take()
            .expect("Failed to read stdout"),
    );

    (stdin, stdout_reader)
}

fn debug_board(board: &[[&str; 7]; 6]) {
    for row in board {
        for column in row {
            print!("{column} ");
        }
        println!();
    }
}

// Checks that the state of the board is the same as the state of the buffer. Because it is
// impossible to know how the buffer is formatted, it justs checks that there is the right amount
// of p1 or p2 pieces
fn matches(buffer: &[String; LINES_OF_OUTPUT], board: &[[&str; 7]; 6]) -> bool {
    let mut p1 = [0; 6];
    let mut p2 = [0; 6];
    for i in 0..6 {
        for column in board[i] {
            if column == PLAYER_1 {
                p1[i] += 1;
            } else if column == PLAYER_2 {
                p2[i] += 1;
            }
        }
    }

    for i in 0..6 {
        if count_matches(&buffer[i + UNIVERSAL_OFFSET] as &str, PLAYER_1) != p1[i]
            || count_matches(&buffer[i + UNIVERSAL_OFFSET] as &str, PLAYER_2) != p2[i]
        {
            return false;
        }
    }

    return true;
}

fn debug(buffer: &[String; LINES_OF_OUTPUT]) {
    for line in buffer {
        print!("{line}");
    }
}

// returns the number of occurences of the substring in the buffer
fn count_matches(buffer: &str, substring: &str) -> usize {
    buffer.matches(substring).count()
}

// This just drops a piece according to the rows of connect 4
fn drop_piece(board: &mut [[&str; 7]; 6], column: usize, player: &'static str) {
    let mut row = 5;
    // This will by default panic if row goes below 0, because of the type bounds on unsigned
    // integers
    while board[row][column - 1] != " " {
        row -= 1;
    }

    board[row][column - 1] = player;
}
