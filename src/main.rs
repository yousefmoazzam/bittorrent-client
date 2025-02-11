use bittorrent_client::{download::download, init::init, metainfo::Metainfo, parse::parse};
use clap::Parser;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
struct Cli {
    /// Filepath to metainfo (`.torrent`) file
    metainfo: PathBuf,

    /// Path to save downloaded file
    output: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    if !cli.metainfo.exists() {
        println!("File doesn't exist: {:?}", cli.metainfo);
        println!("Exiting...");
        std::process::exit(1);
    }

    let mut file = File::open(cli.metainfo.clone()).expect(&format!(
        "Expected to be able to open file: {:?}",
        cli.metainfo
    ));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    let bencoded_data = parse(&bytes);
    let metainfo = match Metainfo::new(bencoded_data) {
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
        Ok(val) => val,
    };
    let torrent = init(metainfo).await;
    let data = download(torrent).await;
    let mut out_file = match File::create(cli.output.clone()) {
        Err(_) => {
            println!("Unable to create output file: {:?}", cli.output);
            std::process::exit(1);
        }
        Ok(val) => val,
    };
    if out_file.write_all(&data).is_err() {
        println!("Unable to save data to output file: {:?}", cli.output);
        std::process::exit(1);
    }
}
