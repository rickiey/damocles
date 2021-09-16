use anyhow::{anyhow, Context, Result};
use async_std::task::block_on;
use clap::{value_t, App, Arg, ArgMatches, SubCommand};

use venus_worker::{
    client::{connect, WorkerClient},
    logging::{debug_field, info},
    Config,
};

pub const SUB_CMD_NAME: &str = "worker";

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let list_cmd = SubCommand::with_name("list");
    let pause_cmd = SubCommand::with_name("pause").arg(
        Arg::with_name("index")
            .long("index")
            .short("i")
            .takes_value(true)
            .required(true)
            .help("index of the worker"),
    );

    let resume_cmd = SubCommand::with_name("resume")
        .arg(
            Arg::with_name("index")
                .long("index")
                .short("i")
                .takes_value(true)
                .required(true)
                .help("index of the worker"),
        )
        .arg(
            Arg::with_name("state")
                .long("state")
                .short("s")
                .takes_value(true)
                .required(false)
                .help("next state"),
        );

    SubCommand::with_name(SUB_CMD_NAME)
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .takes_value(true)
                .required(true)
                .help("path to the config file"),
        )
        .subcommand(list_cmd)
        .subcommand(pause_cmd)
        .subcommand(resume_cmd)
}

pub fn submatch<'a>(subargs: &ArgMatches<'a>) -> Result<()> {
    match subargs.subcommand() {
        ("list", _) => get_client(subargs).and_then(|wcli| {
            let infos = block_on(wcli.worker_list()).map_err(|e| anyhow!("rpc error: {:?}", e))?;
            for wi in infos {
                info!(
                    paused = wi.paused,
                    paused_elapsed = debug_field(wi.paused_elapsed),
                    state = wi.state.as_str(),
                    last_err = debug_field(wi.last_error),
                    "#{}: {:?}",
                    wi.index,
                    wi.location,
                );
            }

            Ok(())
        }),

        ("pause", Some(m)) => {
            let index = value_t!(m, "index", usize)?;
            get_client(subargs).and_then(|wcli| {
                let done = block_on(wcli.worker_pause(index))
                    .map_err(|e| anyhow!("rpc error: {:?}", e))?;

                info!(done, "#{} worker pause", index);
                Ok(())
            })
        }

        ("resume", Some(m)) => {
            let index = value_t!(m, "index", usize)?;
            let state = m.value_of("state").map(|s| s.to_owned());
            get_client(subargs).and_then(|wcli| {
                let done = block_on(wcli.worker_resume(index, state.clone()))
                    .map_err(|e| anyhow!("rpc error: {:?}", e))?;

                info!(done, state = debug_field(state), "#{} worker resume", index);
                Ok(())
            })
        }

        (other, _) => Err(anyhow!("unexpected subcommand `{}` of worker", other)),
    }
}

fn get_client<'a>(m: &ArgMatches<'a>) -> Result<WorkerClient> {
    let cfg_path = value_t!(m, "config", String).context("get config path")?;
    let cfg = Config::load(&cfg_path)?;
    connect(&cfg)
}
