// 集成测试：测试导入功能

#[test]
fn test_scan_downloads() {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let downloads_dir = PathBuf::from(&home).join("Downloads");

    println!("Testing download scanner");
    println!("Home: {}", home);
    println!("Downloads dir: {:?}", downloads_dir);
    println!("Exists: {}", downloads_dir.exists());

    assert!(downloads_dir.exists(), "Downloads directory should exist");

    let mut configs = Vec::new();

    if let Ok(entries) = fs::read_dir(&downloads_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "conf" {
                        println!("Found: {:?}", path);
                        configs.push(path);
                    }
                }
            }
        }
    }

    println!("Total configs found: {}", configs.len());

    if configs.is_empty() {
        println!("WARNING: No .conf files in Downloads");
        println!("Expected to find at least str-dub303.conf");
    } else {
        for config in &configs {
            println!("  - {}", config.display());
        }
    }

    // 应该找到至少一个文件
    assert!(!configs.is_empty(), "Should find at least one .conf file");
}
