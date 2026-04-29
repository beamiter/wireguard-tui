// 测试配置文件扫描功能
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("=== Testing Config Scanner ===\n");

    // 模拟 ConfigDownloader::new()
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let downloads_dir = PathBuf::from(&home).join("Downloads");

    println!("HOME: {}", home);
    println!("Downloads dir: {:?}", downloads_dir);
    println!("Exists: {}\n", downloads_dir.exists());

    if !downloads_dir.exists() {
        println!("ERROR: Downloads directory does not exist!");
        return;
    }

    // 模拟 scan_downloads()
    let mut configs = Vec::new();

    println!("Scanning for .conf files...\n");

    match fs::read_dir(&downloads_dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        println!("  Checking: {:?}", path);

                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                println!("    Extension: {:?}", ext);
                                if ext == "conf" {
                                    println!("    ✓ Adding to list");
                                    configs.push(path);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("  Error reading entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("ERROR: Failed to read directory: {}", e);
            return;
        }
    }

    println!("\n=== Results ===");
    println!("Total .conf files found: {}", configs.len());

    if configs.is_empty() {
        println!("\nNo .conf files found!");
        println!("Please check:");
        println!("  1. Files are in {}", downloads_dir.display());
        println!("  2. Files have .conf extension");
        println!("  3. You have read permissions");
    } else {
        println!("\nFound files:");
        for config in &configs {
            println!("  - {}", config.display());
        }
    }
}
