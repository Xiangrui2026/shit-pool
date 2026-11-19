use clap::{Args, Parser, Subcommand, ValueEnum, value_parser};

#[derive(Parser)]
#[command(
    name = "shit-pool",
    version,
    about = "Search and manage the SHIT-Pool question bank"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        env = "SHIT_POOL_URL",
        default_value = "http://127.0.0.1:3000"
    )]
    pub url: String,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Find questions by keyword, category and/or tags; returns one page of JSON.
    Search(Search),
    /// Print a single question as JSON, by its persistent ID.
    Get {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Print a standalone UTF-8 XeLaTeX document to stdout.
    Tex {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Download the standalone TeX document and its images as a ZIP file.
    Export {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(short, long, value_name = "ZIP_FILE")]
        output: std::path::PathBuf,
    },
    /// Upload a PNG or JPEG and print its path and \includegraphics snippet as JSON.
    UploadImage {
        #[arg(long, value_name = "FILE")]
        file: std::path::PathBuf,
    },
    /// List category IDs, names and question counts as JSON.
    Categories,
    /// List tag names and question counts as JSON.
    Tags,
    /// Check API and database status.
    Health,
    /// Add a question from a UTF-8 JSON file; use --file - for stdin.
    Add {
        #[arg(long, value_name = "FILE")]
        file: std::path::PathBuf,
    },
    /// Apply a partial JSON update by ID; use --file - for stdin.
    Update {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(long, value_name = "FILE")]
        file: std::path::PathBuf,
    },
    /// Soft-delete a question by ID. Requires --yes; never prompts.
    Delete {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(long)]
        yes: bool,
    },
    /// Manage individual solutions; use get to read them.
    #[command(subcommand)]
    Solution(SolutionCommand),
}

#[derive(Subcommand)]
pub(crate) enum SolutionCommand {
    /// Add a named solution from UTF-8 JSON; use --file - for stdin.
    Add {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        question_id: i32,
        #[arg(long, value_name = "FILE")]
        file: std::path::PathBuf,
    },
    /// Update a solution's title/body or make it the default.
    Update {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        question_id: i32,
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(long, value_name = "FILE")]
        file: std::path::PathBuf,
    },
    /// Soft-delete a solution. Requires --yes; never prompts.
    Delete {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        question_id: i32,
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Args)]
pub(crate) struct Search {
    #[arg(short = 'q', long, help = "Literal match against the stem or source")]
    pub query: Option<String>,
    #[arg(long, value_parser = value_parser!(i32).range(1..), help = "Category ID from categories")]
    pub category_id: Option<i32>,
    #[arg(long = "tag", help = "One exact tag name; repeat for multiple tags")]
    pub tags: Vec<String>,
    #[arg(long, value_enum, default_value = "any")]
    pub tag_match: TagMatch,
    #[arg(long, default_value_t = 1, value_parser = value_parser!(u32).range(1..))]
    pub page: u32,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum TagMatch {
    Any,
    All,
}
