use anyhow::Result;
use ccg::{
    CommandContext,
    commands::{
        Command as CommandTrait, CreateCommand, DiffCommand, InitCommand, ListCommand,
        RestoreCommand, ShowCommand,
        traits::{CreateArgs, DiffArgs, InitArgs, ListArgs, RestoreArgs, ShowArgs},
    },
    i18n::setup_i18n,
};
use clap::{Arg, Command as ClapCommand};
use git2::Repository;
use rust_i18n::t;
use std::process;

rust_i18n::i18n!("locales");

fn build_cli() -> ClapCommand {
    ClapCommand::new("ccg")
        .version("0.1.0")
        .about(t!("app_about"))
        .long_about(t!("app_long_about"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(ClapCommand::new("init").about(t!("init_about")))
        .subcommand(
            ClapCommand::new("create")
                .about(t!("create_about"))
                .arg(Arg::new("message").help(t!("create_message_help")).index(1))
                .arg(
                    Arg::new("tool_input_json")
                        .long("tool-input-json")
                        .help(t!("create_tool_input_json_help"))
                        .long_help(t!("create_tool_input_json_long_help")),
                ),
        )
        .subcommand(
            ClapCommand::new("list").about(t!("list_about")).arg(
                Arg::new("number")
                    .short('n')
                    .long("number")
                    .help(t!("list_number_help"))
                    .default_value("10"),
            ),
        )
        .subcommand(
            ClapCommand::new("restore").about(t!("restore_about")).arg(
                Arg::new("hash")
                    .help(t!("restore_hash_help"))
                    .required(true),
            ),
        )
        .subcommand(
            ClapCommand::new("show")
                .about(t!("show_about"))
                .arg(Arg::new("hash").help(t!("show_hash_help")).required(true))
                .arg(
                    Arg::new("diff")
                        .short('d')
                        .long("diff")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("show_diff_help")),
                ),
        )
        .subcommand(
            ClapCommand::new("diff")
                .about(t!("diff_about"))
                .arg(
                    Arg::new("hash_a")
                        .help(t!("diff_hash_a_help"))
                        .required(true),
                )
                .arg(Arg::new("hash_b").help(t!("diff_hash_b_help"))),
        )
}

fn run() -> Result<()> {
    let matches = build_cli().get_matches();
    let subcommand_name = matches.subcommand_name().unwrap_or("");

    // Check if the current directory is a git repository
    let is_repo = Repository::open(".").is_ok();

    if !is_repo {
        match subcommand_name {
            "init" | "create" => {
                // These commands can proceed as they handle repository initialization
            }
            _ => {
                // For other commands, print a message and exit
                println!("{}", t!("repo_not_initialized_tip"));
                return Ok(());
            }
        }
    }

    let context = CommandContext::new()?;

    match matches.subcommand() {
        Some(("init", _)) => {
            let cmd = InitCommand::new(context);
            let args = InitArgs;
            CommandTrait::execute(&cmd, args)?;
        }
        Some(("create", sub_matches)) => {
            let cmd = CreateCommand::new(context);
            let args = CreateArgs {
                message: sub_matches.get_one::<String>("message").cloned(),
            };
            CommandTrait::execute(&cmd, args)?;
        }
        Some(("list", sub_matches)) => {
            let cmd = ListCommand::new(context);
            let number_str = sub_matches.get_one::<String>("number").unwrap();
            let number = number_str.parse::<usize>()?;
            let args = ListArgs { number };
            CommandTrait::validate_args(&cmd, &args)?;
            CommandTrait::execute(&cmd, args)?;
        }
        Some(("restore", sub_matches)) => {
            let cmd = RestoreCommand::new(context);
            let hash = sub_matches.get_one::<String>("hash").unwrap().clone();
            let args = RestoreArgs { hash };
            CommandTrait::validate_args(&cmd, &args)?;
            CommandTrait::execute(&cmd, args)?;
        }
        Some(("show", sub_matches)) => {
            let cmd = ShowCommand::new(context);
            let hash = sub_matches.get_one::<String>("hash").unwrap().clone();
            let diff = sub_matches.get_flag("diff");
            let args = ShowArgs { hash, diff };
            CommandTrait::validate_args(&cmd, &args)?;
            CommandTrait::execute(&cmd, args)?;
        }
        Some(("diff", sub_matches)) => {
            let cmd = DiffCommand::new(context);
            let hash_a = sub_matches.get_one::<String>("hash_a").unwrap().clone();
            let hash_b = sub_matches.get_one::<String>("hash_b").cloned();
            let args = DiffArgs { hash_a, hash_b };
            CommandTrait::validate_args(&cmd, &args)?;
            CommandTrait::execute(&cmd, args)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    setup_i18n(); // 初始化 i18n

    if let Err(error) = run() {
        eprintln!("{}: {error}", t!("error_prefix"));

        let mut source = error.source();
        let mut level = 1;
        while let Some(err) = source {
            eprintln!(
                "   {} {}: {}",
                "  ".repeat(level),
                t!("error_cause_prefix"),
                err
            );
            source = err.source();
            level += 1;
        }

        eprintln!();
        eprintln!("{}", t!("error_tip"));
        process::exit(1);
    }
}
