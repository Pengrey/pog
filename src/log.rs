/// Print an informational message: `[*]` in yellow.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        println!("[{}] {}", "\x1b[33m*\x1b[0m", format!($($arg)*))
    };
}

/// Print a success message: `[+]` in green.
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
        println!("[{}] {}", "\x1b[32m+\x1b[0m", format!($($arg)*))
    };
}

/// Print a debug message: `[>><]` in yellow.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        eprintln!("[{}] {}", "\x1b[33m>><\x1b[0m", format!($($arg)*))
    };
}

/// Print an error message: `[-]` in red.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("[{}] {}", "\x1b[31m-\x1b[0m", format!($($arg)*))
    };
}
