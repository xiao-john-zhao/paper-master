#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--queue-missing-local")
    {
        if let Err(error) = paper_master_lib::archive_cli::queue_missing_local() {
            eprintln!("{}: {}", error.code_string(), error);
            std::process::exit(1);
        }
        return;
    }
    if args.first().is_some_and(|arg| arg == "--sync-cache") {
        if let Err(error) = paper_master_lib::archive_cli::sync_library_cache() {
            eprintln!("{}: {}", error.code_string(), error);
            std::process::exit(1);
        }
        return;
    }
    if args.first().is_some_and(|arg| arg == "--archive") {
        if args.len() < 2 {
            eprintln!("Usage: paper-master --archive <folder> [local-report.json]");
            std::process::exit(2);
        }
        if let Err(error) = paper_master_lib::archive_cli::execute(&args) {
            eprintln!("{}: {}", error.code_string(), error);
            std::process::exit(1);
        }
        return;
    }
    if args.first().is_some_and(|arg| arg == "--analyze") {
        let Some(id) = args.get(1) else {
            std::process::exit(2)
        };
        if let Err(error) = paper_master_lib::archive_cli::analyze_one(id) {
            eprintln!("{}: {}", error.code_string(), error);
            std::process::exit(1);
        }
        return;
    }
    paper_master_lib::run();
}
