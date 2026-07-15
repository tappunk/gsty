use color_eyre::Result;

mod theme;

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("gsty {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "gsty {} - Ghostty theme browser and installer",
            env!("CARGO_PKG_VERSION")
        );
        println!();
        println!("Usage:");
        println!("  gsty              Interactive TUI theme picker");
        println!("  gsty --list       Plain text listing of discovered themes");
        println!("  gsty --version    Print version");
        println!("  gsty --help       Print this help");
        println!();
        println!("TUI Keybindings:");
        println!("  j/k     Navigate themes    f     Cycle filter (all/dark/light)");
        println!("  /       Start search       g/G   Jump to first/last");
        println!("  Enter   Apply theme        q/Esc Cancel and restore previous");
        return Ok(());
    }

    let list_only = args.iter().any(|arg| arg == "--list" || arg == "-l");
    let unknown_args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| !matches!(*arg, "--list" | "-l" | "--version" | "-V" | "--help" | "-h"))
        .collect();

    if !unknown_args.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "unknown argument(s): {}",
            unknown_args.join(" ")
        ));
    }

    theme::run(list_only)?;
    Ok(())
}

fn main() {
    if let Err(e) = color_eyre::install() {
        eprintln!("error: failed to install error handler: {e}");
        std::process::exit(1);
    }

    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
