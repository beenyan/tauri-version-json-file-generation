#[macro_use]
extern crate maplit;

pub mod error;
pub mod platform;
pub mod release;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    match release::get_release_latest().await {
        Ok(release) => {
            // println!("{:#?}", release);
            if let Err(e) = release.summon().await {
                eprintln!("Error while summon versions.json: {e}")
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    };
}
