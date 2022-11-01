mod rule;
mod rule_exec_env_ffi;
mod rule_executor;

extern crate derive_builder;

use crate::rule_executor::*;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut re = RuleExecutor::new()?;

    let args: Vec<String> = std::env::args().collect();

    for arg in args.iter().skip(1) {
        let file: PathBuf = arg.into();
        let _ = re.load_file(&file).await;
    }

    for entry in re.rule_map.iter() {
        let rule = entry.value();
        let docs = rule.meta.docs.as_ref().unwrap();
        let url = &docs.url.as_ref().unwrap().to_string();
        println!("# {} ({})", rule.name, url);
        println!("{}\n", &docs.description);
    }

    Ok(())
}
