use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    created: SystemTime
}

fn get_current_directory() -> String {
    env::current_dir()
        .unwrap()
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

fn normalize_filename(filename: &str) -> String {
    // separate name and extension
    let (stem, extension) = match filename.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (filename, None),
    };
    
    // patterns to strip (order matters - check longer regex patterns first)
    let patterns = [
        r" copy \d+$",       // "file copy 2"
        r" copy$",           // "file copy"
        r" - Copy \(\d+\)$", // "file - Copy (2)"
        r" - Copy$",         // "file - Copy"
        r" \(\d+\)$",        // "file (1)"
        r"\(\d+\)$",         // "file(1)"
    ];
    
    let mut normalized = stem.to_string();
    
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(&normalized) {
            normalized = re.replace(&normalized, "").to_string();
            break;
        }
    }
    
    // reconstruct with extension
    match extension {
        Some(ext) => format!("{}.{}", normalized, ext),
        None => normalized,
    }
}

fn find_and_delete_duplicate_files(directory: String, dry_run: bool) {
    // step 1: group files by normalized filename
    let mut hashmap_name: HashMap<String, Vec<FileInfo>> = HashMap::new();

    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", directory, e);
            return;
        }
    };

    for file in entries {
        let file = match file {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error reading directory entry: {}", e);
                continue;
            }
        };

        let path = file.path();

        // skip directories, only process files
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error reading metadata for '{}': {}", path.display(), e);
                continue;
            }
        };

        if !metadata.is_file() {
            continue;
        }

        // get filename
        let filename = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => {
                eprintln!("Warning: Could not extract filename from path '{}'", path.display());
                continue;
            }
        };

        let normalized_filename = normalize_filename(&filename);
        let size = metadata.len();

        // try to get creation time, use modified time as fallback
        let created = match metadata.created() {
            Ok(time) => time,
            Err(_) => {
                match metadata.modified() {
                    Ok(time) => time,
                    Err(e) => {
                        eprintln!("Warning: Could not get creation or modified time for '{}': {}", path.display(), e);
                        continue;
                    }
                }
            }
        };

        let file_info = FileInfo {
            path: path.clone(),
            size,
            created,
        };
        hashmap_name.entry(normalized_filename).or_insert(vec![]).push(file_info);
    }

    // step 2: for each normalized filename group, sub-group by size and find duplicates
    let mut total_duplicates_found = 0;
    let mut total_files_to_delete = 0;

    for (normalized_filename, file_infos) in &hashmap_name {
        // only process if there are multiple files with this normalized name
        if file_infos.len() > 1 {
            // sub-group by size within this filename group
            let mut hashmap_size: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
            for file_info in file_infos {
                hashmap_size.entry(file_info.size).or_insert(vec![]).push(file_info);
            }

            // check each size group for duplicates
            for (size, size_group) in &hashmap_size {
                if size_group.len() > 1 {
                    total_duplicates_found += 1;
                    total_files_to_delete += size_group.len() - 1;

                    // find one specific file to keep (first one with earliest timestamp)
                    let file_to_keep = match size_group.iter().min_by_key(|f| f.created) {
                        Some(file) => file,
                        None => continue,
                    };

                    println!("\n--- Duplicate Set ---");
                    println!("Normalized filename: {}", normalized_filename);
                    println!("Size: {} bytes", size);
                    println!("Keeping: {}", file_to_keep.path.display());

                    // list files to delete
                    for file_info in size_group {
                        if file_info.path != file_to_keep.path {
                            if dry_run {
                                println!("Would delete: {}", file_info.path.display());
                            } else {
                                println!("Will delete: {}", file_info.path.display());
                            }
                        }
                    }
                }
            }
        }
    }

    if total_duplicates_found == 0 {
        println!("\nNo duplicates found!");
        return;
    }

    println!("\n================================");
    println!("Summary: Found {} duplicate set(s)", total_duplicates_found);
    println!("Total files to delete: {}", total_files_to_delete);

    if dry_run {
        println!("\n[DRY RUN MODE] No files were deleted.");
        println!("Run without --dry-run to actually delete files.");
        return;
    }

    print!("\nProceed with deletion? (y/N): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        println!("Deletion cancelled.");
        return;
    }

    println!("\nDeleting files...");
    let mut deleted_count = 0;
    let mut error_count = 0;

    for (_normalized_filename, file_infos) in &hashmap_name {
        if file_infos.len() > 1 {
            let mut hashmap_size: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
            for file_info in file_infos {
                hashmap_size.entry(file_info.size).or_insert(vec![]).push(file_info);
            }

            for (_size, size_group) in &hashmap_size {
                if size_group.len() > 1 {
                    let file_to_keep = match size_group.iter().min_by_key(|f| f.created) {
                        Some(file) => file,
                        None => continue,
                    };

                    for file_info in size_group {
                        if file_info.path != file_to_keep.path {
                            match fs::remove_file(&file_info.path) {
                                Ok(_) => {
                                    println!("Deleted: {}", file_info.path.display());
                                    deleted_count += 1;
                                }
                                Err(e) => {
                                    eprintln!("Error deleting '{}': {}", file_info.path.display(), e);
                                    error_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n================================");
    println!("Deletion complete!");
    println!("Files deleted: {}", deleted_count);
    if error_count > 0 {
        println!("Errors encountered: {}", error_count);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // check for --dry-run flag
    let dry_run = args.iter().any(|arg| arg == "--dry-run");

    if dry_run {
        println!("Running in DRY RUN mode - no files will be deleted\n");
    }

    find_and_delete_duplicate_files(get_current_directory(), dry_run);
}