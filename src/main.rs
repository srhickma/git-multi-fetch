extern crate git2;

use {
    git2::{AutotagOption, Cred, FetchOptions, RemoteCallbacks, Repository},
    std::{
        env,
        error::Error,
        fs::File,
        io::{self, BufRead, BufReader, Write},
        process, str,
    },
};

const INDEX_FILE_NAME: &str = ".gmf";

fn main() {
    let index_file = match File::open(INDEX_FILE_NAME) {
        Ok(file) => file,
        Err(err) => fatal(
            &format!("Failed to load configuration '{}'", INDEX_FILE_NAME),
            &err,
        ),
    };

    for line in BufReader::new(index_file).lines() {
        let repo_path = match line {
            Ok(repo_path) => repo_path,
            Err(err) => {
                error(
                    &format!("Failed to read configuration line '{}'", INDEX_FILE_NAME),
                    &err,
                );
                continue;
            }
        };

        let repo = match Repository::open(&repo_path) {
            Ok(repo) => repo,
            Err(err) => {
                error(&format!("Failed to open repository '{}'", &repo_path), &err);
                continue;
            }
        };

        let remotes = match repo.remotes() {
            Ok(remotes) => remotes,
            Err(err) => {
                error(&format!("Failed to list remotes for '{}'", repo_path), &err);
                continue;
            }
        };

        for remote_name_opt in remotes.iter() {
            let remote_name = match remote_name_opt {
                Some(remote_name) => remote_name,
                None => {
                    println!("info: Skipping non-utf8 remote name for '{}'", repo_path);
                    continue;
                }
            };

            println!("Fetching '{}' for '{}'", remote_name, repo_path);

            let remote_result = repo
                .find_remote(remote_name)
                .or_else(|_| repo.remote_anonymous(remote_name));

            let mut remote = match remote_result {
                Ok(remote) => remote,
                Err(err) => {
                    error(&format!("Failed to list remotes for '{}'", repo_path), &err);
                    continue;
                }
            };

            let mut cb = RemoteCallbacks::new();
            cb.sideband_progress(|data| {
                print!("remote: {}", str::from_utf8(data).unwrap());
                io::stdout().flush().unwrap();
                true
            });

            cb.update_tips(|refname, a, b| {
                if a.is_zero() {
                    println!("[new]     {:20} {}", b, refname);
                } else {
                    println!("[updated] {:10}..{:10} {}", a, b, refname);
                }
                true
            });

            cb.transfer_progress(|stats| {
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
            });

            cb.credentials(|_url, username_from_url, _allowed_types| {
                Cred::ssh_key_from_agent(username_from_url.unwrap())
            });

            let mut fo = FetchOptions::new();
            fo.remote_callbacks(cb);
            remote.download(&[] as &[&str], Some(&mut fo)).unwrap();

            {
                // If there are local objects (we got a thin pack), then tell the user
                // how many objects we saved from having to cross the network.
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

            remote.disconnect().unwrap();

            remote
                .update_tips(None, true, AutotagOption::Unspecified, None)
                .unwrap();
        }
    }
}

fn fatal(message: &str, error: &dyn Error) -> ! {
    println!("fatal: {}: {}", message, error);
    process::exit(1);
}

fn error(message: &str, error: &dyn Error) {
    println!("error: {}: {}", message, error);
}
