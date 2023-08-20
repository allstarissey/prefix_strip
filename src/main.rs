use std::path::PathBuf;

use clap::Parser;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Manually sets prefix")]
    prefix: Option<String>,

    #[arg(short, long, help = "Use all files in specified directory", default_value = "./")]
    source_directory: Option<PathBuf>,

    #[arg(last = true, conflicts_with = "source_directory")]
    files: Option<Vec<PathBuf>>,

    #[arg(short = 'd', long, help = "Include directories to be stripped")]
    include_directories: bool
}


fn main() {
    let args = Args::parse();

    let files: Vec<PathBuf> = 'files: {
        if args.files.is_some() {
            break 'files Vec::from(args.files.unwrap())
        }

        let dir = args.source_directory.unwrap();
        let read_dir = match std::fs::read_dir(dir.clone()) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                eprintln!("Couldn't open directory {}: {e}", dir.display());
                return;
            }
        };

        let size_hint = match read_dir.size_hint() {
            (_, Some(upper_bound)) => upper_bound,
            (lower_bound, None) => lower_bound,
        };
        let mut file_vec: Vec<PathBuf> = Vec::with_capacity(size_hint);

        for entry_result in read_dir {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("Failed to read directory entry: {e}");
                    continue;
                }
            };

            if !args.include_directories {
                let file_type = match entry.file_type() {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error getting file type: {e}");
                        continue;
                    }
                };

                if !file_type.is_file() {
                    continue;
                }
            }

            file_vec.push(entry.path());
        }

        file_vec
    };
}
