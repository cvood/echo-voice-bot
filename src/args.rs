use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author = "Me", version = "0.1")]
#[command(about = "echo an voice from a text", long_about = None)]
pub struct Args {
    #[arg(short, long, value_name="PATH")]
    pub data_path: Option<PathBuf>
}
