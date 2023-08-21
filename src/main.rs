use std::path::PathBuf;
use clap::Parser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Manually sets prefix")]
    prefix: Option<String>,

    #[arg(short, long, help = "Use all files in specified directory", default_value = "./")]
    source_directory: PathBuf,

    #[arg(last = true, conflicts_with = "source_directory")]
    files: Option<Vec<PathBuf>>,

    #[arg(short = 'd', long, help = "Include directories to be stripped")]
    include_directories: bool
}


fn main() {
    let args = Args::parse();

    let paths = match get_paths(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error getting files: {e}");
            return;
        }
    };
}

fn get_paths(args: &Args) -> Result<Vec<PathBuf>> {
    let list: Vec<PathBuf>;
    if let Some(file_list) = &args.files {
        let mut existing: Vec<PathBuf> = Vec::with_capacity(file_list.len());
        for file in file_list {
            match file.as_path().try_exists() {
                Ok(exists) => if exists {
                    existing.push(file.clone());
                } else {
                    eprintln!("File {} doesn't exist", file.display());
                }
                Err(e) => {
                    eprintln!("Couldn't detect if {} exists: {e}", file.display());
                }
            }
        }

        list = existing;
    } else {
        let read_dir = std::fs::read_dir(&args.source_directory)?;

        let size_hint = match read_dir.size_hint() {
            (_, Some(upper_bound)) => upper_bound,
            (lower_bound, None) => lower_bound,
        };
        let mut paths: Vec<PathBuf> = Vec::with_capacity(size_hint);

        for entry in read_dir {
            if let Err(e) = entry {
                eprintln!("Error reading directory entry: {e}");
                continue;
            }

            paths.push(entry.unwrap().path());
        }

        list = Vec::from(paths)
    }

    let final_list: Vec<PathBuf> = list.iter() // I wanna use drain_filter so bad, but the stable branch beckons.
        .filter(|pathbuf| {
            if pathbuf.is_dir() && !args.include_directories {
                return false;
            }

            true
        })
        .cloned()
        .collect();

    Ok(final_list)
}
