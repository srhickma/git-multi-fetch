extern crate git2;

use {
    git2::{
        AutotagOption, Cred, CredentialHelper, FetchOptions, Remote, RemoteCallbacks, Repository,
    },
    std::{
        error, fmt,
        fs::File,
        io::{self, BufRead, BufReader, Write},
        process, result, str,
    },
};

type Result<T> = result::Result<T, Box<dyn error::Error>>;

const INDEX_FILE_NAME: &str = ".gmf";

#[derive(Debug)]
enum Error {
    Nested(String, Box<dyn error::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Nested(ref msg, ref err) => write!(f, "{}: {}", msg, err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

fn main() {
    let index_file = match File::open(INDEX_FILE_NAME) {
        Ok(file) => file,
        Err(err) => fatal(
            &format!("failed to load configuration '{}'", INDEX_FILE_NAME),
            err.into(),
        ),
    };

    for line in BufReader::new(index_file).lines() {
        let repo_path = match line {
            Ok(repo_path) => repo_path,
            Err(err) => {
                error(
                    &format!("failed to read configuration line '{}'", INDEX_FILE_NAME),
                    err.into(),
                );
                continue;
            }
        };

        if let Err(err) = fetch_all(&repo_path) {
            error("fetch failed", err);
        }
    }
}

fn fetch_all(repo_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path).map_err(|err| {
        Error::Nested(
            format!("failed to open repository '{}'", repo_path),
            err.into(),
        )
    })?;

    let remotes = repo.remotes().map_err(|err| {
        Error::Nested(
            format!("failed to list remotes for '{}'", repo_path),
            err.into(),
        )
    })?;

    for remote_name in remotes.iter().filter(Option::is_some).map(Option::unwrap) {
        println!("Fetching remote '{}' for '{}'", remote_name, repo_path);

        if let Err(err) = fetch_remote(&repo, remote_name) {
            error("fetch failed", err);
        }
    }

    Ok(())
}

fn fetch_remote(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .or_else(|_| repo.remote_anonymous(remote_name))
        .map_err(|err| Error::Nested("failed to list remotes".to_string(), err.into()))?;

    let mut options = get_fetch_options();

    remote
        .download(&[] as &[&str], Some(&mut options))
        .map_err(|err| Error::Nested("failed downloading from remote".to_string(), err.into()))?;

    print_remote_stats(&remote);

    remote
        .disconnect()
        .map_err(|err| Error::Nested("failed disconnecting from remote".to_string(), err.into()))?;

    remote
        .update_tips(None, true, AutotagOption::Unspecified, None)
        .map_err(|err| Error::Nested("failed updating remote tips".to_string(), err.into()))?;

    Ok(())
}

fn get_fetch_options<'scope>() -> FetchOptions<'scope> {
    let mut options = FetchOptions::new();
    options.remote_callbacks(get_remote_callbacks());

    options
}

fn get_remote_callbacks<'scope>() -> RemoteCallbacks<'scope> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks
        .sideband_progress(|data| {
            print!("remote: {}", str::from_utf8(data).unwrap());
            io::stdout().flush().unwrap();
            true
        })
        .update_tips(|refname, a, b| {
            if a.is_zero() {
                println!("[new]     {:20} {}", b, refname);
            } else {
                println!("[updated] {:10}..{:10} {}", a, b, refname);
            }
            true
        })
        .transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                print!(
                    "Resolving deltas {}/{}\r",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                );
            } else if stats.total_objects() > 0 {
                print!(
                    "Received {}/{} objects ({}) in {} bytes\r",
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.indexed_objects(),
                    stats.received_bytes()
                );
            }
            io::stdout().flush().unwrap();
            true
        })
        .credentials(|url, username_from_url, _allowed_types| {
            let username = CredentialHelper::new(url)
                .username
                .or_else(|| username_from_url.map(|s| s.to_string()));

            match username {
                Some(username) => Cred::ssh_key_from_agent(&username),
                None => Cred::default(),
            }
        });

    callbacks
}

fn print_remote_stats(remote: &Remote) {
    let stats = remote.stats();

    if stats.local_objects() > 0 {
        println!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        println!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }
}

fn fatal(message: &str, error: Box<dyn error::Error>) -> ! {
    println!("fatal: {}: {}", message, error);
    process::exit(1);
}

fn error(message: &str, error: Box<dyn error::Error>) {
    println!("error: {}: {}", message, error);
}
