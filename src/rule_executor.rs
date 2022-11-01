use crate::rule::*;
use crate::rule_exec_env_ffi::*;
use anyhow::bail;
use dashmap::DashMap;
use deno_core::futures::FutureExt;
use deno_core::Extension;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSourceFuture;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::*;
use tokio::fs;

pub struct NetModuleLoader;

impl ModuleLoader for NetModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _is_main: bool,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        Ok(deno_core::resolve_import(specifier, referrer)?)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        async move {
            let scheme = module_specifier.scheme().to_string();
            let string_specifier = module_specifier.to_string();

            let bytes: Vec<u8> = match scheme.clone().as_str() {
                "file" => {
                    let path = match module_specifier.to_file_path() {
                        Ok(path) => path,
                        Err(_) => bail!("Invalid file URL."),
                    };
                    fs::read(path).await?
                }
                schema => bail!("Invalid schema {}", schema),
            };

            // Strip BOM
            let code = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                bytes.as_slice()[3..].to_vec()
            } else {
                bytes
            }
            .into_boxed_slice();

            let module = ModuleSource {
                code,
                module_type: ModuleType::JavaScript,
                module_url_specified: string_specifier.clone(),
                module_url_found: string_specifier.to_string(),
            };

            Ok(module)
        }
        .boxed_local()
    }
}

static JS_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/JS_SNAPSHOT.bin"));

#[derive(Error, Debug)]
pub enum RuleExecutorError {
    #[error(transparent)]
    DenoExecutionError(anyhow::Error),

    #[error("The module name `{module_name}` is invalid: {reason:?}")]
    BadModuleName {
        module_name: String,
        reason: deno_core::url::ParseError,
    },

    #[error("The module `{module_name}` had issues importing some other files: {reason:?}")]
    ModuleResolutionError {
        module_name: String,
        reason: anyhow::Error,
    },

    #[error("The module name `{module_name}` could not be evaluated: {reason:?}")]
    ModuleEvaluationError {
        module_name: String,
        reason: anyhow::Error,
    },

    #[error("Could not read file {file:?} due to {err:?}")]
    CouldNotReadFile { file: PathBuf, err: std::io::Error },
}

pub struct RuleExecutor {
    runtime: deno_core::JsRuntime,
    pub rule_map: Arc<DashMap<RuleId, Rule>>,
}

impl RuleExecutor {
    pub fn new() -> Result<RuleExecutor, RuleExecutorError> {
        let rule_map = Arc::new(DashMap::new());

        let extension: deno_core::Extension = {
            let rule_map = rule_map.clone();
            let inner_state = InnerState {
                id: uuid::Uuid::new_v4(),
                rule_map,
            };

            Extension::builder()
                .ops(vec![crate::rule_exec_env_ffi::op_escheck_rule_new::decl()])
                .state(move |state| {
                    state.put(inner_state.clone());
                    Ok(())
                })
                .build()
        };

        let rt_options = deno_core::RuntimeOptions {
            startup_snapshot: Some(deno_core::Snapshot::Static(JS_SNAPSHOT)),
            module_loader: Some(Rc::new(NetModuleLoader)),
            extensions: vec![extension, deno_console::init()],
            ..Default::default()
        };
        let runtime = deno_core::JsRuntime::new(rt_options);

        let mut rule_executor = Self { runtime, rule_map };

        rule_executor.setup()?;

        Ok(rule_executor)
    }

    pub async fn load_file(&mut self, file: PathBuf) -> Result<(), RuleExecutorError> {
        let module_name = format!("file://{}", file.to_str().unwrap());
        let module_code =
            fs::read_to_string(&file)
                .await
                .map_err(|err| RuleExecutorError::CouldNotReadFile {
                    file: file.clone(),
                    err,
                })?;
        self.load(&module_name, Some(module_code)).await
    }

    pub async fn load(
        &mut self,
        module_name: &str,
        module_code: Option<String>,
    ) -> Result<(), RuleExecutorError> {
        let mod_specifier =
            url::Url::parse(module_name).map_err(|reason| RuleExecutorError::BadModuleName {
                module_name: module_name.to_string(),
                reason,
            })?;

        let mod_id = self
            .runtime
            .load_side_module(&mod_specifier, module_code)
            .await
            .map_err(|reason| RuleExecutorError::ModuleResolutionError {
                module_name: module_name.to_string(),
                reason,
            })?;

        let eval_future = self.runtime.mod_evaluate(mod_id);

        self.runtime.run_event_loop(false).await.map_err(|reason| {
            RuleExecutorError::ModuleEvaluationError {
                module_name: module_name.to_string(),
                reason,
            }
        })?;

        let _ = eval_future.await.unwrap();

        self.runtime
            .get_module_namespace(mod_id)
            .map_err(|reason| RuleExecutorError::ModuleEvaluationError {
                module_name: module_name.to_string(),
                reason,
            })?;

        Ok(())
    }

    pub fn setup(&mut self) -> Result<(), RuleExecutorError> {
        self.runtime
            .execute_script("<prelude>", include_str!("prelude.js"))
            .map_err(RuleExecutorError::DenoExecutionError)?;

        Ok(())
    }
}
