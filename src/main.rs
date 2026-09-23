use std::{env, io};

fn main() -> io::Result<()> {
    editor_91to9::cli::run(env::args())
}
