
// Prints a message to standard error and exits the program with exit code 1
#[macro_export]
macro_rules! exit {
    ( $msg:expr ) => {
        eprintln!($msg);
        std::process::exit(1);
    };
}
