#![feature(debug_closure_helpers)]

mod feed;
mod config;
mod database;

fn main() {

    let mut args = std::env::args();
    let prg_name = args.next().unwrap();

    match real_main() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{prg_name}: {e}");

            std::process::exit(1)
        }
    }
}

fn real_main() -> std::io::Result<()> {
    let mut config = config::load_config()?;

    println!("{config:#?}"); // This is unsafe but just for testing purposes

    let feeds = feed::query_metadata(&mut config)?;

    println!("{feeds:#?}");


    feed::download_feeds(&feeds)?;



    Ok(())
}
