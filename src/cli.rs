use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use colored::Colorize as _;

use crate::check::{Severity, check};
use crate::error::{Error, Result};
use crate::page::Kind;
use crate::repo::{CreatePageOptions, EditPageOptions, Repo};

/// The one-line description. `--help` prints it and `prime` prints it,
/// from this one place, so the two cannot drift.
pub const ABOUT: &str = "Diataxis documentation sets as typed markdown";

#[derive(Parser)]
#[command(name = "diataxis", version, about = ABOUT, max_term_width = 98)]
pub struct Args {
    /// Documentation set directory. Literal: the directory must hold
    /// diataxis.yml; no walk, no fallback. Without it, the nearest
    /// diataxis.yml at or above the cwd is used.
    #[arg(long, global = true)]
    pub root: Option<Utf8PathBuf>,

    /// Act on the configured root store, wherever the command runs.
    #[arg(long, global = true, conflicts_with = "root")]
    pub home: bool,

    /// Read the user config from this file instead of its fixed path.
    /// A test seam; a flag is visible where an env var is not.
    #[arg(long, global = true, hide = true, overrides_with = "user_config")]
    pub user_config: Option<Utf8PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// Whether a command needs a store at all, and whether it writes.
/// Exhaustive on the leaf action, so a new subcommand does not compile
/// until it is classified.
fn intent(command: &Command) -> Option<mdstore::resolve::Intent> {
    use mdstore::resolve::Intent::{Read, Write};
    Some(match command {
        // Rootless: these never resolve a store.
        Command::Init
        | Command::GenMan { .. }
        | Command::GenCompletions { .. }
        | Command::Prime
        | Command::Docs(_)
        | Command::Store(StoreCommand::Root(_)) => return None,
        // The compass with no page names nothing in the store, but the
        // form is one command and one intent. A read costs nothing.
        Command::Map | Command::Compass(_) | Command::Check | Command::Search(_) => Read,
        Command::Store(StoreCommand::List) => Read,
        // Sync mutates the cache.
        Command::Store(StoreCommand::Sync) => Write,
        Command::Page(cmd) => match **cmd {
            PageCommand::List(_) | PageCommand::Show(_) | PageCommand::Path(_) => Read,
            PageCommand::Create(_) | PageCommand::Edit(_) | PageCommand::Delete(_) => Write,
        },
    })
}

#[derive(Parser)]
pub enum Command {
    /// Initialize a documentation set in the current directory
    Init,

    /// Write the man pages into a directory (the package build uses this)
    #[command(hide = true)]
    GenMan {
        /// Output directory for the section-1 pages
        dir: std::path::PathBuf,
    },

    /// Print a shell completion script (the package build uses this)
    #[command(hide = true)]
    GenCompletions {
        /// Target shell
        shell: clap_complete::Shell,
    },

    /// Print what diataxis is and how to use it, for an agent's context
    Prime,

    /// Manage pages
    #[command(subcommand)]
    Page(Box<PageCommand>),

    /// Show the set as the four Diataxis quadrants
    Map,

    /// Show the two questions that name a page's type
    Compass(CompassArgs),

    /// Check the pages for type faults and broken links
    Check,

    /// Search the pages
    Search(SearchArgs),

    /// Manage the declared documentation sets
    #[command(subcommand)]
    Store(StoreCommand),

    /// Show the bundled documentation
    Docs(DocsArgs),
}

#[derive(Parser)]
pub enum PageCommand {
    /// Create a page of one type
    Create(PageCreateArgs),

    /// List the pages
    List(PageListArgs),

    /// Show one page
    Show(PageShowArgs),

    /// Change a page
    Edit(PageEditArgs),

    /// Remove a page
    Delete(PageDeleteArgs),

    /// Print the file path of a page
    Path(PageShowArgs),
}

#[derive(clap::Args)]
pub struct PageCreateArgs {
    /// The page title. A how-to title states a goal: "How to ...".
    pub title: String,

    /// The Diataxis type: tutorial, how-to, reference, or explanation
    #[arg(short = 'T', long)]
    pub r#type: String,

    /// The one-line description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Tags, separated by commas
    #[arg(short, long)]
    pub tags: Option<String>,

    /// The page body
    #[arg(short, long)]
    pub body: Option<String>,

    /// The file stem, when the slug from the title is not wanted
    #[arg(long)]
    pub id: Option<String>,
}

#[derive(clap::Args)]
pub struct PageListArgs {
    /// Show one type only
    #[arg(short = 'T', long)]
    pub r#type: Option<String>,
}

#[derive(clap::Args)]
pub struct PageShowArgs {
    /// The page id, or a unique prefix of one
    pub page: String,
}

#[derive(clap::Args)]
pub struct PageDeleteArgs {
    /// The page id, or a unique prefix of one
    pub page: String,
}

#[derive(clap::Args)]
pub struct PageEditArgs {
    /// The page id, or a unique prefix of one
    pub page: String,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(short, long)]
    pub description: Option<String>,

    /// Retype the page
    #[arg(short = 'T', long)]
    pub r#type: Option<String>,

    /// Replace the tags
    #[arg(short, long)]
    pub tags: Option<String>,

    /// Replace the body
    #[arg(short, long)]
    pub body: Option<String>,
}

#[derive(clap::Args)]
pub struct CompassArgs {
    /// A page to name alongside the questions
    pub page: Option<String>,
}

#[derive(Parser)]
pub enum StoreCommand {
    /// List the documentation sets this set reads
    List,
    /// Fetch the declared remote sets into the local cache
    Sync,
    /// Show or set the root set that reads fall back to
    Root(StoreRootArgs),
}

#[derive(clap::Args)]
pub struct StoreRootArgs {
    /// The directory of the root set. Without it, print the current
    /// setting.
    pub path: Option<Utf8PathBuf>,

    /// Replace an already-set root with a different one.
    #[arg(long)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct SearchArgs {
    /// A regular expression, matched without regard to case
    pub pattern: String,
}

pub use crate::docs::DocsArgs;

/// This tool's own documentation, compiled in.
///
/// One line, and no list. The build script embedded every page in the
/// directory `diataxis.yml` names, so `diataxis docs` works from any
/// directory with no files present.
static DOCS: &[(&str, &str)] = crate::embedded_docs!();

const VOCAB: mdstore::resolve::Vocabulary<'static> = mdstore::resolve::Vocabulary {
    marker: "diataxis.yml",
    noun: "documentation set",
    tool: "diataxis",
};

pub fn run(args: Args) -> Result<()> {
    let Some(intent) = intent(&args.command) else {
        if args.home {
            return Err(Error::General(
                "--home does not apply to this command".to_string(),
            ));
        }
        if let Command::Store(StoreCommand::Root(a)) = args.command {
            return run_store_root(&a, args.user_config.as_deref());
        }
        let cwd = cwd_utf8()?;
        let root = match &args.root {
            Some(r) if r.is_relative() => cwd.join(r),
            Some(r) => r.clone(),
            None => cwd,
        };
        return run_command(&root, args.command);
    };
    let cwd = cwd_utf8()?;
    let config = load_user_config(args.user_config.as_deref())?;
    let resolved = mdstore::resolve::resolve_root(
        cwd.as_std_path(),
        args.root.as_ref().map(|r| r.as_std_path()),
        args.home,
        intent,
        &config,
        VOCAB,
    )
    .map_err(|e| Error::General(e.to_string()))?;
    let root = Utf8PathBuf::from_path_buf(resolved.root)
        .map_err(|p| Error::General(format!("non-UTF-8 root {}", p.display())))?;
    announce(&root, &cwd, resolved.via, intent);
    run_command(&root, args.command)
}

fn cwd_utf8() -> Result<Utf8PathBuf> {
    let cwd = std::env::current_dir()?;
    Utf8PathBuf::try_from(cwd).map_err(|e| Error::General(e.to_string()))
}

fn load_user_config(path: Option<&Utf8Path>) -> Result<mdstore::userconfig::UserConfig> {
    let loaded = match path {
        Some(p) => mdstore::userconfig::UserConfig::load_from(p.as_std_path()),
        None => mdstore::userconfig::UserConfig::load(),
    };
    loaded.map_err(|e| Error::General(e.to_string()))
}

/// One stderr line whenever the answer to "where did that act" is not
/// visible in the command line itself.
fn announce(
    root: &Utf8Path,
    cwd: &Utf8Path,
    via: mdstore::resolve::Via,
    intent: mdstore::resolve::Intent,
) {
    use mdstore::resolve::{Intent, Via};
    match (intent, via) {
        (_, Via::Flag) => {}
        (Intent::Write, Via::Home) => eprintln!("diataxis: writing to root set {root}"),
        (Intent::Write, _) if root != cwd => eprintln!("diataxis: writing to set {root}"),
        (Intent::Read, Via::Walk) if root != cwd => {
            eprintln!("diataxis: using set at {root} (diataxis.yml above {cwd})");
        }
        (Intent::Read, Via::Config) => {
            eprintln!("diataxis: no diataxis.yml at or above {cwd}; reading root set {root}");
        }
        (Intent::Read | Intent::Write, _) => {}
    }
}

pub fn run_command(root: &Utf8Path, command: Command) -> Result<()> {
    match command {
        Command::Init => Repo::init(root),

        Command::GenMan { dir } => {
            use clap::CommandFactory as _;
            std::fs::create_dir_all(&dir)?;
            crate::mangen::write_man_pages(&Args::command(), &dir)?;
            Ok(())
        }

        Command::GenCompletions { shell } => {
            use clap::CommandFactory as _;
            clap_complete::generate(
                shell,
                &mut Args::command(),
                "diataxis",
                &mut std::io::stdout(),
            );
            Ok(())
        }

        Command::Prime => {
            print!("{}", prime());
            Ok(())
        }

        Command::Docs(args) => crate::docs::DocSet::from_embedded(DOCS)?.run(&args),

        Command::Page(cmd) => run_page(root, *cmd),

        Command::Map => run_map(root),

        Command::Compass(args) => run_compass(root, args.page.as_deref()),

        Command::Check => run_check(root),

        Command::Search(args) => run_search(root, &args.pattern),

        Command::Store(cmd) => run_store(root, cmd),
    }
}

fn run_page(root: &Utf8Path, command: PageCommand) -> Result<()> {
    let repo = Repo::open(root)?;
    match command {
        PageCommand::Create(args) => {
            let kind: Kind = args.r#type.parse()?;
            let page = repo.create(
                &args.title,
                kind,
                CreatePageOptions {
                    description: args.description,
                    tags: args.tags,
                    body: args.body,
                    id: args.id,
                },
            )?;
            println!("{} {}", page.id.bold(), format!("({kind})").dimmed());
            Ok(())
        }
        PageCommand::List(args) => {
            let wanted = args.r#type.map(|t| t.parse::<Kind>()).transpose()?;
            let pages = repo.list()?;
            for page in pages
                .iter()
                .filter(|p| wanted.is_none_or(|k| p.kind() == k))
            {
                println!(
                    "{:<28} {:<12} {}",
                    page.id.bold(),
                    page.kind().to_string().dimmed(),
                    page.title()
                );
            }
            Ok(())
        }
        PageCommand::Show(args) => {
            let page = repo.get(&args.page)?;
            println!("{}", page.id.bold());
            println!();
            println!("  Title:        {}", page.title());
            println!("  Type:         {}", page.kind());
            if !page.frontmatter.description.is_empty() {
                println!("  Description:  {}", page.frontmatter.description);
            }
            if !page.frontmatter.tags.is_empty() {
                println!("  Tags:         {}", page.frontmatter.tags.join(", "));
            }
            println!();
            println!("{}", page.body);
            Ok(())
        }
        PageCommand::Path(args) => {
            let page = repo.get(&args.page)?;
            println!("{}", repo.path_of(&page.id)?);
            Ok(())
        }
        PageCommand::Edit(args) => {
            let kind = args.r#type.map(|t| t.parse::<Kind>()).transpose()?;
            let page = repo.edit(
                &args.page,
                EditPageOptions {
                    title: args.title.as_deref(),
                    description: args.description.as_deref(),
                    kind,
                    tags: args.tags.as_deref(),
                    body: args.body.as_deref(),
                },
            )?;
            println!(
                "{} {}",
                page.id.bold(),
                format!("({})", page.kind()).dimmed()
            );
            Ok(())
        }
        PageCommand::Delete(args) => {
            let page = repo.delete(&args.page)?;
            println!("removed {}", page.id.bold());
            Ok(())
        }
    }
}

/// The set as the four quadrants, in Diataxis order.
///
/// A list hides a gap. A grid shows one: eleven how-to guides and one
/// tutorial, or a product with no explanation at all.
fn run_map(root: &Utf8Path) -> Result<()> {
    let repo = Repo::open(root)?;
    let pages = repo.list()?;
    println!("{}", repo.title().bold());
    for kind in Kind::ALL {
        let held: Vec<_> = pages.iter().filter(|p| p.kind() == kind).collect();
        println!();
        println!(
            "{}  {}",
            kind.to_string().bold(),
            format!(
                "informs {}, serves {} of skill — {} page(s)",
                kind.informs(),
                kind.serves(),
                held.len()
            )
            .dimmed()
        );
        if held.is_empty() {
            println!("  {}", "(none)".dimmed());
        }
        for page in held {
            println!("  {:<28} {}", page.id, page.title().dimmed());
        }
    }
    Ok(())
}

/// The two questions, and the table that turns a pair of answers into a
/// type.
///
/// The compass does not classify a page. Classification needs a reader,
/// and a tool that guessed would be inventing a fact.
fn run_compass(root: &Utf8Path, selector: Option<&str>) -> Result<()> {
    println!("{}", "Two questions name a page's type.".bold());
    println!();
    println!(
        "  1. Does it inform {} or {}?",
        "action".bold(),
        "cognition".bold()
    );
    println!(
        "  2. Does it serve {} of skill, or {}?",
        "acquisition".bold(),
        "application".bold()
    );
    println!();
    for kind in Kind::ALL {
        println!(
            "  {:<12} {:<10} {:<12} {}",
            kind.to_string().bold(),
            kind.informs(),
            kind.serves(),
            kind.brief().dimmed()
        );
    }
    if let Some(selector) = selector {
        let repo = Repo::open(root)?;
        let page = repo.get(selector)?;
        println!();
        println!(
            "{} declares {}: informs {}, serves {}.",
            page.id.bold(),
            page.kind().to_string().bold(),
            page.kind().informs(),
            page.kind().serves()
        );
    }
    Ok(())
}

fn run_check(root: &Utf8Path) -> Result<()> {
    let findings = check(root)?;
    if findings.is_empty() {
        println!("no problems found");
        return Ok(());
    }
    let mut faults = 0;
    for finding in &findings {
        let label = match finding.severity {
            Severity::Fault => {
                faults += 1;
                "fault".red()
            }
            Severity::Advice => "advice".yellow(),
        };
        println!("{label}  {:<28} {}", finding.subject, finding.message);
    }
    if faults > 0 {
        return Err(Error::General(format!("{faults} fault(s)")));
    }
    Ok(())
}

fn run_search(root: &Utf8Path, pattern: &str) -> Result<()> {
    let repo = Repo::open(root)?;
    for hit in repo.search(pattern)? {
        println!(
            "{}:{}  {}  {}",
            hit.id.bold(),
            hit.line_number,
            hit.kind.to_string().dimmed(),
            hit.line
        );
    }
    Ok(())
}

/// `store list` and `store sync`. `store root` never reaches here,
/// because it resolves nothing and runs from any directory.
fn run_store(root: &Utf8Path, command: StoreCommand) -> Result<()> {
    match command {
        StoreCommand::List => {
            let workspace = crate::workspace::Workspace::open(root)?;
            for member in workspace.store_members() {
                let label = if member.alias.is_empty() {
                    "(this set)".to_string()
                } else {
                    member.alias.clone()
                };
                let state = match &member.unavailable {
                    Some(why) => format!("unavailable: {why}"),
                    None => format!("{} page(s)", member.pages),
                };
                let age = match &member.age {
                    Some(age) => format!("  synced {age}"),
                    None => String::new(),
                };
                println!("{}  {}  {state}{age}", label.bold(), member.source);
            }
            Ok(())
        }
        StoreCommand::Sync => {
            let workspace = crate::workspace::Workspace::open_fetching(root)?;
            let results = workspace.sync_all();
            if results.is_empty() {
                println!("no remote sets declared");
            }
            let mut failed = false;
            for (alias, outcome) in results {
                match outcome {
                    Ok(()) => println!("{alias}  synced"),
                    Err(e) => {
                        failed = true;
                        eprintln!("{alias}  failed: {e}");
                    }
                }
            }
            if failed {
                return Err(Error::General(
                    "one or more sets failed to sync".to_string(),
                ));
            }
            Ok(())
        }
        StoreCommand::Root(_) => unreachable!("store root is rootless and handled in run()"),
    }
}

/// `diataxis store root [<path>] [--force]`: show or set the root set
/// in the user config. Rootless, and acts on its literal argument;
/// resolution here would be circular.
fn run_store_root(args: &StoreRootArgs, user_config: Option<&Utf8Path>) -> Result<()> {
    let config = load_user_config(user_config)?;
    let Some(path) = &args.path else {
        match config.root_store {
            Some(root) => println!(
                "root_store: {} (~/.config/mdstore/config.yml)",
                root.display()
            ),
            None => println!("root_store: unset"),
        }
        return Ok(());
    };
    let cwd = cwd_utf8()?;
    let abs = if path.is_relative() {
        cwd.join(path)
    } else {
        path.clone()
    };
    if !abs.join("diataxis.yml").is_file() {
        return Err(Error::General(format!(
            "no diataxis.yml in {abs}; run `diataxis init` there first"
        )));
    }
    if let Some(old) = &config.root_store
        && old != abs.as_std_path()
        && !args.force
    {
        return Err(Error::General(format!(
            "root_store is {}; pass --force to change it to {abs}",
            old.display()
        )));
    }
    if config.root_store.as_deref() == Some(abs.as_std_path()) {
        println!("root_store: {abs} (unchanged)");
        return Ok(());
    }
    let old = config.root_store.clone();
    let target = user_config
        .map(|p| p.as_std_path().to_path_buf())
        .or_else(mdstore::userconfig::config_path)
        .ok_or_else(|| Error::General("no home directory resolves".to_string()))?;
    mdstore::userconfig::UserConfig::save_root_to(&target, abs.as_std_path())
        .map_err(|e| Error::General(e.to_string()))?;
    match old {
        Some(old) => println!(
            "root_store: {} -> {abs} ({})",
            old.display(),
            target.display()
        ),
        None => println!("root_store: {abs} ({})", target.display()),
    }
    Ok(())
}

/// What an agent is told about this tool, in one screen.
#[must_use]
pub fn prime() -> String {
    format!(
        "# diataxis\n\
         {ABOUT}\n\
         A page is one markdown document that declares one Diataxis type: tutorial, \
         how-to, reference, or explanation. A documentation set is a directory with \
         diataxis.yml, found upward from the cwd or named by --root. Pages live in the \
         directory that config names, `docs` by default, and a page id is its file stem.\n\
         Commands:\n  \
         diataxis page create <title> -T <type>\n  \
         diataxis page list [-T <type>]\n  \
         diataxis page show <id>\n  \
         diataxis map\n  \
         diataxis compass [<id>]\n  \
         diataxis check\n\
         More: diataxis --help; diataxis docs\n"
    )
}
