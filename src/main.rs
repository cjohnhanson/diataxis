use clap::Parser as _;

fn main() -> miette::Result<()> {
    // A piped reader that closes early ends the process quietly, the
    // way every other command-line tool does.
    sigpipe::reset();

    // miette colours its error line whatever the stream is, so a piped
    // or redirected run carried escapes even under NO_COLOR. anstream
    // answers the same question for every other line this tool writes,
    // and this hands miette that answer.
    // `current_choice() == Always` was wrong: anstream returns Always
    // only from its Windows console arm, so this read false on every
    // unix terminal and miette rendered monochrome where it should not.
    // `ColorChoice::global()` is the resolved answer, and it already
    // honours NO_COLOR, CLICOLOR and CLICOLOR_FORCE.
    let colour = match anstream::ColorChoice::global() {
        anstream::ColorChoice::Always | anstream::ColorChoice::AlwaysAnsi => true,
        anstream::ColorChoice::Never => false,
        anstream::ColorChoice::Auto => std::io::IsTerminal::is_terminal(&std::io::stderr()),
    };
    let _ = miette::set_hook(Box::new(move |_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .color(colour)
                .unicode(colour)
                .build(),
        )
    }));

    let args = diataxis::cli::Args::parse();
    diataxis::cli::run(args)?;
    Ok(())
}
