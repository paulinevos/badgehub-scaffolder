use anyhow::Result;
use badgehub::cli::Cli;
use badgehub::wizard::Wizard;
use clap::Parser;

fn main() -> Result<()> {
    Cli::parse().run(&Wizard::on_this_terminal())
}
