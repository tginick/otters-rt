use crate::conf::BoardEffectConfigParameterValue;
use crate::utils::async_utils::{RTQueue, Receiver, Sender};

use serde::Serialize;
use std::collections::HashMap;

// bind name, idx in effects vec, param idx
type EffectParameterMapping = (String, usize, usize);
pub type ParamNameAndIndex = (&'static str, usize);

// global idx, new value
pub type AsyncParamUpdate = (usize, BoardEffectConfigParameterValue);

#[derive(Serialize)]
pub struct OttersSessionInfoEntry {
    effect_name: String,
    global_idxs: Vec<ParamNameAndIndex>,
}

#[derive(Serialize)]
pub struct OttersSessionInfo {
    // bind name -> info
    infos: HashMap<String, OttersSessionInfoEntry>,
}

pub struct OttersParamModifierContext {
    sender: Sender<AsyncParamUpdate>,

    session_info: OttersSessionInfo,
}

pub struct ParameterMappingManager {
    mappings: Vec<EffectParameterMapping>,
    bind_name_to_glob_idxs: HashMap<String, Vec<ParamNameAndIndex>>,
    bind_name_to_effect_type: HashMap<String, String>,
}

// this is kinda meant to be used in FFI
// so return FFI-friendly types
impl OttersParamModifierContext {
    pub fn get_session_info_json(&self) -> String {
        serde_json::to_string(&self.session_info).unwrap()
    }

    pub fn set_flt_param_value(&self, global_idx: u32, value: f32) {
        self.sender.send((
            global_idx as usize,
            BoardEffectConfigParameterValue::F(value),
        ));
    }

    pub fn set_int_param_value(&self, global_idx: u32, value: i32) {
        self.sender.send((
            global_idx as usize,
            BoardEffectConfigParameterValue::N(value),
        ));
    }
}

impl ParameterMappingManager {
    pub fn new() -> ParameterMappingManager {
        ParameterMappingManager {
            mappings: Vec::new(),
            bind_name_to_glob_idxs: HashMap::new(),
            bind_name_to_effect_type: HashMap::new(),
        }
    }

    pub fn new_parameter(&mut self, m: EffectParameterMapping) -> usize {
        let next_param_idx = self.mappings.len();

        println!(
            "Global Parameter Manager: New Param Index {:?} -> Ordinal {}",
            &m, next_param_idx
        );
        self.mappings.push(m);

        next_param_idx
    }

    pub fn set_global_idxs_for_bind_name(
        &mut self,
        bind_name: String,
        idxs: Vec<ParamNameAndIndex>,
    ) {
        debug_print_global_idxs_info(&bind_name, &idxs);
        self.bind_name_to_glob_idxs.insert(bind_name, idxs);
    }

    pub fn set_effect_type_for_bind_name(&mut self, bind_name: String, effect_name: String) {
        self.bind_name_to_effect_type.insert(bind_name, effect_name);
    }

    pub fn get_effect_type_for_bind_name<'a>(&'a self, bind_name: &str) -> &'a String {
        &self.bind_name_to_effect_type[bind_name]
    }

    pub fn get_glob_idxs_for_bind_name<'a>(
        &'a self,
        bind_name: &str,
    ) -> &'a Vec<ParamNameAndIndex> {
        &self.bind_name_to_glob_idxs[bind_name]
    }

    pub fn effect_and_param_idx(&self, global_idx: usize) -> (usize, usize) {
        let (_, effect_idx, param_idx) = self.mappings[global_idx];

        (effect_idx, param_idx)
    }

    pub fn create_async_param_update_context(
        &self,
    ) -> (OttersParamModifierContext, Receiver<AsyncParamUpdate>) {
        let (sender, receiver) = RTQueue::<AsyncParamUpdate>::new();

        let mut session_info: HashMap<String, OttersSessionInfoEntry> = HashMap::new();

        self.bind_name_to_effect_type
            .iter()
            .map(|(k, _)| k.clone())
            .for_each(|x| {
                let effect_name = self.bind_name_to_effect_type.get(&x).unwrap().clone();
                let global_idxs = self.bind_name_to_glob_idxs.get(&x).unwrap().clone();
                session_info.insert(
                    x,
                    OttersSessionInfoEntry {
                        effect_name,
                        global_idxs,
                    },
                );
            });
        let context = OttersParamModifierContext {
            session_info: OttersSessionInfo {
                infos: session_info,
            },
            sender,
        };

        (context, receiver)
    }
}

fn debug_print_global_idxs_info(bind_name: &str, idxs: &Vec<ParamNameAndIndex>) {
    for e in idxs {
        println!(
            "Global Parameter Manager: New Param Name - {}, {:?}",
            bind_name, e
        );
    }
}
