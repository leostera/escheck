mod rule;
mod rule_exec_env_ffi;
mod rule_executor;

extern crate derive_builder;

use crate::rule_executor::*;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut re = RuleExecutor::new()?;

    let args: Vec<String> = std::env::args().collect();
    re.load_file(args.get(1).unwrap().into()).await?;

    dbg!(&re.rule_map);

    Ok(())
}
