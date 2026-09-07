use clap::Parser as _;

fn main() -> miette::Result<()> {
    // A piped reader that closes early ends the process quietly, the
    // way every other command-line tool does.
    sigpipe::reset();

    // miette colours its error line whatever the stream is, so a piped
    // or redirected run carried escapes even under NO_COLOR. anstream
    // answers the same question for every other line this tool writes,
    // and this hands miette that answer.
    let colour = anstream::stdout().current_choice() == anstream::ColorChoice::Always
        || anstream::stderr().current_choice() == anstream::ColorChoice::Always;
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
