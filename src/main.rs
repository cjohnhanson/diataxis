use clap::Parser as _;

fn main() -> miette::Result<()> {
    // A piped reader that closes early ends the process quietly, the
    // way every other command-line tool does.
    sigpipe::reset();
    let args = diataxis::cli::Args::parse();
    diataxis::cli::run(args)?;
    Ok(())
}
