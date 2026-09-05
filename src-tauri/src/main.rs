// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 4 && args[1] == "--broker-protocol" {
        let exit_code = ai_switcher_lib::protocol_broker::handle_broker_cli(&args[2], &args[3]);
        std::process::exit(exit_code);
    }
    ai_switcher_lib::run()
}
