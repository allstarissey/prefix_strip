use std::path::PathBuf;
use clap::Parser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;


#[derive(Debug)]
struct NoFilesRemaining;

impl std::fmt::Display for NoFilesRemaining {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: None of the specified files could be affected")
    }
}

impl std::error::Error for NoFilesRemaining {}


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Manually sets prefix")]
    prefix: Option<String>,

    #[arg(short, long, help = "Use all files in specified directory", default_value = "./")]
    source_directory: PathBuf,

    #[arg(last = true, conflicts_with = "source_directory", help = "Explicit list of files to be changed, mutually exclusive with -d")]
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
            match &file.try_exists() {
                Ok(exists) if *exists => existing.push(file.clone()),
                Ok(_) => eprintln!("File {} doesn't exist", file.display()),
                Err(e) => {
                    eprintln!("Couldn't detect if {} exists: {e}", file.display());
                }
            }
        }

        list = existing;
    } else {
        let read_dir = std::fs::read_dir(&args.source_directory)?;

        // Use size_hint to try to preallocate space, uses upper bound if possible, otherwise uses lower bound
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

    if final_list.is_empty() {
        return Err(Box::new(NoFilesRemaining))
    }

    Ok(final_list)
}
