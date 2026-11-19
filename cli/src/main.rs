mod api;
mod args;
mod document;
mod error;
mod files;
mod payload;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use args::{Cli, Command, SolutionCommand};
use clap::{Parser, error::ErrorKind};
use error::{CliError, Result};
use serde::Serialize;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = serde_json::to_writer(&mut stderr, &error);
            let _ = writeln!(stderr);
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<()> {
    let arguments = match Cli::try_parse() {
        Ok(arguments) => arguments,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return io::stdout()
                .lock()
                .write_all(error.to_string().as_bytes())
                .map_err(CliError::output);
        }
        Err(error) => {
            return Err(CliError::usage(format!(
                "Invalid arguments ({}). Run the subcommand with --help.",
                error.kind().as_str().unwrap_or("argument error")
            )));
        }
    };
    let client = api::Api::new(&arguments.url)?;
    match arguments.command {
        Command::Search(filters) => json(&client.search(&filters)?),
        Command::Get { id } => json(&client.get(id)?),
        Command::Tex { id } => {
            let question = client.get(id)?;
            io::stdout()
                .lock()
                .write_all(document::tex(&question).as_bytes())
                .map_err(CliError::output)
        }
        Command::Export { id, output } => {
            let archive = client.export(id)?;
            files::write_zip(&output, &archive)?;
            json(&serde_json::json!({ "id": id, "output": output, "bytes": archive.len() }))
        }
        Command::UploadImage { file } => {
            let auth = client.authorization()?;
            json(&client.upload_image(files::read_image(&file)?, auth)?)
        }
        Command::Categories => json(&client.categories()?),
        Command::Tags => json(&client.tags()?),
        Command::Health => json(&client.health()?),
        Command::Add { file } => {
            let auth = client.authorization()?;
            json(&client.add(&payload::read(&file)?, auth)?)
        }
        Command::Update { id, file } => {
            let auth = client.authorization()?;
            let patch: payload::QuestionPatch = payload::read(&file)?;
            if patch.is_empty() {
                return Err(CliError::usage("Supply at least one field to update."));
            }
            json(&client.update(id, &patch, auth)?)
        }
        Command::Delete { id, yes } => {
            if !yes {
                return Err(CliError::usage("Deleting a question requires --yes."));
            }
            json(&client.delete(id, client.authorization()?)?)
        }
        Command::Solution(command) => match command {
            SolutionCommand::Add { question_id, file } => {
                let auth = client.authorization()?;
                json(&client.add_solution(question_id, &payload::read(&file)?, auth)?)
            }
            SolutionCommand::Update {
                question_id,
                id,
                file,
            } => {
                let auth = client.authorization()?;
                let patch: payload::SolutionPatch = payload::read(&file)?;
                if patch.is_empty() {
                    return Err(CliError::usage(
                        "Supply a title, body or make_default: true.",
                    ));
                }
                json(&client.update_solution(question_id, id, &patch, auth)?)
            }
            SolutionCommand::Delete {
                question_id,
                id,
                yes,
            } => {
                if !yes {
                    return Err(CliError::usage("Deleting a solution requires --yes."));
                }
                json(&client.delete_solution(question_id, id, client.authorization()?)?)
            }
        },
    }
}

fn json(value: &impl Serialize) -> Result<()> {
    let mut buffer = serde_json::to_vec(value)
        .map_err(|_| CliError::io("Could not serialize the command output."))?;
    buffer.push(b'\n');
    io::stdout()
        .lock()
        .write_all(&buffer)
        .map_err(CliError::output)
}
