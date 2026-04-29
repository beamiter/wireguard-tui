use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    println!("HOME: {}", home);

    let downloads_dir = PathBuf::from(&home).join("Downloads");
    println!("Downloads dir: {:?}", downloads_dir);
    println!("Exists: {}", downloads_dir.exists());

    if downloads_dir.exists() {
        println!("\nContents:");
        for entry in fs::read_dir(&downloads_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "conf" {
                        println!("  Found: {:?}", path);
                    }
                }
            }
        }
    }
}
