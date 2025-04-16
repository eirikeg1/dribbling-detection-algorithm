use clap::{command, Parser};



#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Overwrite the config's input path
    #[arg(short, long)]
    pub input: Option<String>,

    /// Overwrite the config's output path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Download dataset
    #[arg(long)]
    pub download: bool,

    /// Review mode
    #[arg(long)]
    pub review: Option<bool>,
}
