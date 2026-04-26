use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about = "Acer PH18-72 lighting control daemon")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print detected backend capabilities without changing hardware.
    Inventory,
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Inventory => inventory(),
    }
}

fn inventory() {
    println!("ph18-lighting-daemon inventory");
    println!("hid.jingmold=05af:866a");
    println!("hid.darfon=0d62:ba51");
    println!("wmi=todo-read-only-triage");
}
