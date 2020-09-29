
use crate::utils::async_utils::Receiver;
use crate::conf::{
    AudioConfig, BoardConfig, BoardEffectConfigParameterValue, BoardEffectDeclaration,
};
use crate::context::BoardContext;
use crate::effects::{loaded_set, FactoryExtension, GenericBypass};
use crate::errors::{FactoryErrors, OttersInitError};
use crate::factory::EffectFactory;
use crate::param::{AsyncParamUpdate, ParamNameAndIndex, ParameterMappingManager};
use crate::traits::AudioEffect;
use crate::OttersParamModifierContext;

use std::collections::HashMap;
use std::fs;

// (ordinal or identifier, effect)
// ordinal must be > 0 and < # total effects
pub type IdentifiedEffect = (usize, Box<dyn AudioEffect>, bool);
pub type LoadedEffects = HashMap<String, IdentifiedEffect>;

struct ConfiguredState {
    parsed_config: BoardConfig,
    factory: EffectFactory,
}

pub struct Otters {
    audio_config: AudioConfig,
    context: BoardContext,

    // TODO: it's probably actually faster to keep the following
    // two arrays in one array to exploit locality
    effects: Vec<Box<dyn AudioEffect>>,
    enable_info: Vec<bool>,

    // just so we don't have to reload the file later
    // in case things need to be rebuilt
    configured_state: ConfiguredState,

    global_param_manager: ParameterMappingManager,
    async_param_update_queue: Option<Receiver<AsyncParamUpdate>>,

    disabled_effect_bypass: GenericBypass,
}

impl Otters {
    pub fn get_available_effect_names() -> Vec<String> {
        let mock_ac = AudioConfig {
            sample_rate: 1_f32,
            max_block_size: 1
        };

        let factory = EffectFactory::assemble_factory(mock_ac, loaded_set());
        factory.get_loaded_effect_names()
    }

    pub fn get_effect_info_json(format_prettily: bool) -> String {
        let mock_ac = AudioConfig {
            sample_rate: 1_f32,
            max_block_size: 1,
        };

        let fake_factory = EffectFactory::assemble_factory(mock_ac, loaded_set());
        fake_factory.get_effect_infos_json(format_prettily)
    }

    pub fn create_default(
        audio_config: AudioConfig,
        config_file_name: &str,
    ) -> Result<Otters, OttersInitError> {
        let read_file_result = fs::read_to_string(config_file_name)?;

        Otters::create_default_from_string(audio_config, &read_file_result)
    }

    pub fn create_default_from_string(
        audio_config: AudioConfig,
        config_str: &str,
    ) -> Result<Otters, OttersInitError> {
        Otters::create(audio_config, loaded_set(), &config_str)
    }

    pub fn create(
        audio_config: AudioConfig,
        factory_extensions: Vec<FactoryExtension>,
        config_str: &str,
    ) -> Result<Otters, OttersInitError> {
        let parsed_config: BoardConfig = serde_json::from_str(&config_str)?;

        let factory = EffectFactory::assemble_factory(audio_config, factory_extensions);

        let effects = create_effect_units(&factory, &parsed_config.effects)?;
        debug_print_loaded_effects(&effects);

        let context = BoardContext::initialize_context(&parsed_config, &audio_config, &effects)?;

        let (mut effects_arr, enabled_arr, global_param_manager) = effect_map_to_vec(effects);

        set_initial_config_on_effects(&parsed_config, &global_param_manager, &mut effects_arr);

        println!("Otters is ready to go!");
        Ok(Otters {
            audio_config,
            context,
            effects: effects_arr,
            configured_state: ConfiguredState {
                parsed_config,
                factory,
            },
            enable_info: enabled_arr,
            global_param_manager,
            async_param_update_queue: None,
            disabled_effect_bypass: GenericBypass::new(),
        })
    }

    pub fn update_audio_config(
        &mut self,
        audio_config: AudioConfig,
    ) -> Result<(), OttersInitError> {
        self.audio_config = audio_config;

        // now we gotta rebuild all of our nice data strctures
        self.configured_state
            .factory
            .change_audio_config(audio_config);

        let effects = create_effect_units(
            &self.configured_state.factory,
            &self.configured_state.parsed_config.effects,
        )?;

        self.context = BoardContext::initialize_context(
            &self.configured_state.parsed_config,
            &self.audio_config,
            &effects,
        )?;

        let (effects, _, global_param_manager) = effect_map_to_vec(effects);

        self.effects = effects;
        self.global_param_manager = global_param_manager;

        Ok(())
    }

    // WARNING: this function is usually called from a UI thread!
    pub fn set_effect_parameter(
        &mut self,
        global_idx: usize,
        value: BoardEffectConfigParameterValue,
    ) {
        let (e_idx, p_idx) = self.global_param_manager.effect_and_param_idx(global_idx);
        self.effects[e_idx].set_effect_parameter(p_idx, value);
    }

    pub fn bind_input(&mut self, input_idx: usize, input_ptr: *const f32) {
        self.context.bind_source(input_idx, input_ptr);
    }

    pub fn bind_output(&mut self, output_idx: usize, output_ptr: *mut f32) {
        self.context.bind_sink(output_idx, output_ptr);
    }

    pub fn frolic(&self, num_samples: usize) {
        // any code that runs here must be rt-safe
        // this means heap mem allocation is not allowed

        for (i, connection) in self.context.get_connections().iter().enumerate() {
            if self.enable_info[connection.ordinal] {
                self.effects[connection.ordinal].execute(&self.context, i, num_samples)
            } else {
                self.disabled_effect_bypass.execute(&self.context, i, num_samples);
            }
        }
    }

    pub fn setup_async_param_updater(&mut self) -> OttersParamModifierContext {
        let (ctx, receiver) = self.global_param_manager.create_async_param_update_context();
        self.async_param_update_queue = Some(receiver);

        ctx
    }
}

fn create_effect_units(
    factory: &EffectFactory,
    effect_configs: &Vec<BoardEffectDeclaration>,
) -> Result<LoadedEffects, FactoryErrors> {
    let mut result = HashMap::new();
    let mut errors = Vec::new();
    let mut current_ordinal = 0usize;

    for effect_decl in effect_configs {
        let unit = factory.create_effect_unit(&effect_decl.effect_name);
        if let Some(real_unit) = unit {
            println!(
                "  Binding the shiny {} to name {}",
                &effect_decl.effect_name, &effect_decl.bind_name
            );
            result.insert(
                effect_decl.bind_name.clone(),
                (current_ordinal, real_unit, effect_decl.enabled),
            );
            current_ordinal += 1;
        } else {
            errors.push(format!("No such effect unit {}", &effect_decl.effect_name));
        }
    }

    if errors.len() > 0 {
        Err(FactoryErrors(errors))
    } else {
        Ok(result)
    }
}

fn effect_map_to_vec(
    effects: HashMap<String, IdentifiedEffect>,
) -> (
    Vec<Box<dyn AudioEffect>>,
    Vec<bool>,
    ParameterMappingManager,
) {
    let mut intermediate: Vec<(String, IdentifiedEffect)> =
        effects.into_iter().map(|x| x).collect();
    intermediate
        .sort_by(|(_, (ord_a, _, _)), (_, (ord_b, _, _))| ord_a.partial_cmp(ord_b).unwrap());

    let mut pm = ParameterMappingManager::new();
    let (result_vec, result_enabled_vec) = intermediate
        .into_iter()
        .enumerate()
        .map(|(i, (bind_name, (_, effect, is_enabled)))| {
            let advertised_params = effect.advertise_parameters();
            let mut global_param_idxs: Vec<ParamNameAndIndex> =
                Vec::with_capacity(advertised_params.len());
            for param_idx in 0..advertised_params.len() {
                global_param_idxs.push((
                    advertised_params[param_idx].name,
                    pm.new_parameter((bind_name.clone(), i, param_idx)),
                ));
            }

            pm.set_global_idxs_for_bind_name(bind_name.clone(), global_param_idxs);

            (effect, is_enabled)
        })
        .collect::<Vec<(Box<dyn AudioEffect>, bool)>>()
        .into_iter()
        .unzip();

    (result_vec, result_enabled_vec, pm)
}

fn set_initial_config_on_effects(
    loaded_conf: &BoardConfig,
    param_mgr: &ParameterMappingManager,
    effects: &mut Vec<Box<dyn AudioEffect>>,
) {
    let mut param_name_to_idx = HashMap::<String, usize>::new();
    for effect_decl in &loaded_conf.effects {
        let params_for_effect = param_mgr.get_glob_idxs_for_bind_name(&effect_decl.bind_name);
        for (n, v) in params_for_effect {
            // bleh have to copy this
            param_name_to_idx.insert(n.to_string(), *v);
        }

        for effect_param in &effect_decl.config {
            if !param_name_to_idx.contains_key(&effect_param.name) {
                continue;
            }

            let (eidx, pidx) =
                param_mgr.effect_and_param_idx(param_name_to_idx[&effect_param.name]);
            effects[eidx].set_effect_parameter(pidx, effect_param.value);
        }

        param_name_to_idx.clear();
    }
}

fn debug_print_loaded_effects(effects: &LoadedEffects) {
    for (effect_name, (effect_ordinal, _, effect_enabled)) in effects.iter() {
        println!(
            "Bind Effect {} -> Ordinal {}. Enabled? {}",
            effect_name, effect_ordinal, effect_enabled
        );
    }
}
