use std::env;
use std::path::PathBuf;

fn main() {
    let vosk_dir = [
        &env::var("CARGO_MANIFEST_DIR").unwrap(),
        &env::var("VOSK_DIR").unwrap_or(String::from("res/vosk")),
    ]
    .iter()
    .collect::<PathBuf>();

    let libvosk = vosk_dir
        .read_dir()
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let extension = path.extension().unwrap().to_str().unwrap();

            if extension == "dll" || extension == "so" {
                Some(path)
            } else {
                None
            }
        })
        .next()
        .expect(format!("no libraries found in {}", vosk_dir.to_string_lossy()).as_str());

    println!(
        "cargo:rustc-link-search=native={}",
        vosk_dir.to_string_lossy()
    );

    copy_to_output::copy_to_output(libvosk.to_str().unwrap(), &env::var("PROFILE").unwrap())
        .unwrap();
}
