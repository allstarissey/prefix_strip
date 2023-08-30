use std::{path::PathBuf, ffi::OsString, io::{stdin, stdout, Write}};
use clap::Parser;
use colored::Colorize;

type GenericResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;


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
    include_directories: bool,

    #[arg(short = 'y', long, help = "Skips confirmation prompt")]
    skip_confirmation: bool,

    #[arg(short, long, help = "Replaces prefix, rather than deleting it. Can be used with empty prefix input to add a prefix")]
    replace: Option<String>,
}


#[derive(Clone)]
struct NamedPath {
    pathbuf: PathBuf,
    name: String,
}

impl NamedPath {
    fn from_pathbuf(pathbuf: PathBuf) -> Option<Self> {
        let name = pathbuf.file_name()?
            .to_string_lossy()
            .to_string();

        Some(Self { pathbuf, name })
    }

    fn pathbuf(&self) -> &PathBuf {
        &self.pathbuf
    }

    fn name(&self) -> &str {
        &self.name
    }
}


fn main() {
    let args = Args::parse();

    let mut named_paths = match get_named_paths(&args) {
        Ok(n_p) => n_p,
        Err(e) => {
            eprintln!("Error getting files: {e}");
            return;
        }
    };

    let prefix: String = match args.prefix {
        Some(p) => match vet_named_paths(&p, named_paths) {
            Ok(vetted) => {
                named_paths = vetted;
                p
            }
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        }
        None => match try_find_prefix(&named_paths) {
            Ok(p_opt) => match p_opt {
                Some(p) => p,
                None => {
                    eprintln!("Couldn't guess a prefix!");
                    return;
                }
            }
            Err(e) => {
                eprintln!("Error guessing prefix: {e}");
                return;
            }
        }
    };

    let prefix_len = prefix.len();

    let new_named_paths: Vec<NamedPath> = get_new_named_paths(&named_paths, &args.replace, &prefix);

    println!("Found prefix: {}", prefix.bold());
    println!("\nAffected files:");
    for named_path in named_paths.iter() {
        let (prefix_name, remainder_name) = named_path.name().split_at(prefix_len);

        println!("{}{remainder_name}", prefix_name.bold().blue());
    }

    println!("\nFiles after changes:");
    for new_named_path in new_named_paths.iter() {
        let new_prefix_len = match &args.replace {
            Some(n_p) => n_p.len(),
            None => 0,
        };

        let (prefix_name, remainder_name) = new_named_path.name().split_at(new_prefix_len);

        println!("{}{remainder_name}", prefix_name.bold().blue());
    }

    println!();
    if !&args.skip_confirmation {
        loop {
            print!("Rename files? [y/N]: ");
            stdout().flush().unwrap();

            let mut response = String::new();
            if let Err(e) = stdin().read_line(&mut response) {
                eprintln!("Failed to read input: {e}");
                return
            }

            match response.trim() {
                "y" | "Y" => break,
                "n" | "N" | "" => return,
                _ => continue
            }
        }
    }

    for (old_path, new_path) in named_paths.into_iter().zip(new_named_paths.into_iter()) {
        if let Err(e) = std::fs::rename(old_path.pathbuf(), new_path.pathbuf()) {
            eprintln!("Failed to rename {}: {e}", old_path.pathbuf().display());
            continue
        }
    }
}

fn get_named_paths(args: &Args) -> GenericResult<Vec<NamedPath>> {
    let named_paths: Vec<NamedPath>;
    if let Some(file_list) = &args.files {
        let mut existing: Vec<NamedPath> = Vec::with_capacity(file_list.len());
        for file in file_list {
            match &file.try_exists() {
                Ok(exists) if *exists => match NamedPath::from_pathbuf(file.clone()) {
                    Some(f) => existing.push(f),
                    None => eprintln!("Couldn't get filename for {}", &file.display()),
                },
                Ok(_) => eprintln!("File {} doesn't exist", &file.display()),
                Err(e) => {
                    eprintln!("Couldn't detect if {} exists: {e}", file.display());
                }
            }
        }

        named_paths = existing;
    } else {
        let read_dir = std::fs::read_dir(&args.source_directory)?;

        // Use size_hint to try to preallocate space, uses upper bound if possible, otherwise uses lower bound
        let size_hint = match read_dir.size_hint() {
            (_, Some(upper_bound)) => upper_bound,
            (lower_bound, None) => lower_bound,
        };
        let mut named_path_list: Vec<NamedPath> = Vec::with_capacity(size_hint);

        for entry in read_dir {
            if let Err(e) = entry {
                eprintln!("Error reading directory entry: {e}");
                continue;
            }

            let path = entry.unwrap().path();
            match NamedPath::from_pathbuf(path.clone()) {
                Some(p) => named_path_list.push(p),
                None => {
                    eprintln!("Couldn't get filename for {}", &path.display());
                    continue;
                }
            }
        }

        named_paths = named_path_list
    }

    if named_paths.is_empty() {
        return Err(Box::new(NoFilesRemaining))
    }

    Ok(named_paths)
}

fn try_find_prefix(named_paths: &[NamedPath]) -> Result<Option<String>, NoFilesRemaining> {
    let names: Vec<&str> = named_paths.iter().map(|p| p.name()).collect();

    let max_length = names.iter()
        .min_by(|&&a, &&b| a.len().cmp(&b.len()))
        .unwrap()
        .len();

    let mut longest_common_prefix: String = String::with_capacity(max_length);

    for index in 0..max_length {
        let first = names[0].as_bytes()
            .get(index)
            .unwrap()
            .to_owned();

        if !names.iter().all(|&n| *n.as_bytes().get(index).unwrap() == first) {
            break
        }

        longest_common_prefix.push(first as char)
    }

    if longest_common_prefix.is_empty() {
        return Ok(None)
    }

    Ok(Some(longest_common_prefix))
}

fn vet_named_paths(prefix: &String, named_paths: Vec<NamedPath>) -> Result<Vec<NamedPath>, NoFilesRemaining> {
    let vetted: Vec<NamedPath> = named_paths.into_iter()
        .filter(|n_p| n_p.name().starts_with(prefix))
        .collect();

    if vetted.is_empty() {
        return Err(NoFilesRemaining)
    }

    Ok(vetted)
}

fn get_new_named_paths(named_paths: &Vec<NamedPath>, replace: &Option<String>, prefix: &str) -> Vec<NamedPath> {
    let mut new_paths: Vec<NamedPath> = Vec::with_capacity(named_paths.len());

    let replace_str = match replace {
        Some(r) => r.to_owned(),
        None => String::new(),
    };

    for named_path in named_paths.iter() {
        let mut new_path = named_path.pathbuf().clone();
        let new_name = named_path.name().replacen(prefix, &replace_str, 1);

        new_path.set_file_name(OsString::from(new_name));

        new_paths.push(NamedPath::from_pathbuf(new_path).unwrap())
    }

    new_paths
}
