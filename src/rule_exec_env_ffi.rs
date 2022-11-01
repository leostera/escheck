use crate::rule::{Rule, RuleId};
use dashmap::DashMap;
use deno_core::error::AnyError;
use deno_core::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct InnerState {
    pub id: uuid::Uuid,
    pub rule_map: Arc<DashMap<RuleId, Rule>>,
}

#[op]
pub fn op_escheck_rule_new(state: &mut OpState, rule: Rule) -> Result<(), AnyError> {
    let inner_state = state.try_borrow_mut::<InnerState>().unwrap();
    inner_state.rule_map.insert(RuleId::next(), rule);
    Ok(())
}
