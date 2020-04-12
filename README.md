[![Build Status](https://travis-ci.com/srhickma/git-multi-fetch.svg?branch=master)](https://travis-ci.com/srhickma/git-multi-fetch)

# git-multi-fetch
This is a command-line tool which can fetch updates to multiple git repositories using a single command. This is useful,
for example, to automatically keep many local backups of remote git repositories up to date.

## Building
To build from source, just run `cargo build --release` in the root of this repo. The resulting executable will be located
at `target/release/gmf`.

## Usage
To use this tool, write a configuration file named `.gmf` containing the relative or absolute paths to the repo's which 
should be fetched, each located on a separate line. Then simply run `./gmf` in the same directory as the configuration 
file, and all refspecs from every remote of each repo listed in the configuration will be fetched.

## Authentication
Currently, the only supported method of git authentication is using an ssh agent, where the username of the ssh agent is
determined by the remote url being fetched.
