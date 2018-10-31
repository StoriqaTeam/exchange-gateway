#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate exchange_gateway_lib;

use clap::App;

fn main() {
    env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let mut app = App::from_yaml(yaml);
    let matches = app.clone().get_matches();

    if let Some(_) = matches.subcommand_matches("config") {
        exchange_gateway_lib::print_config();
    } else if let Some(_) = matches.subcommand_matches("server") {
        exchange_gateway_lib::start_server();
    } else if let Some(matches) = matches.subcommand_matches("create_user") {
        let name = matches.value_of("name").unwrap();
        exchange_gateway_lib::create_user(&name);
    } else {
        let _ = app.print_help();
        println!("\n")
    }
}
