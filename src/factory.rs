use crate::conf::{AdvertisedParameter, AudioConfig};
use crate::effects::FactoryExtension;
use crate::traits::AudioEffect;

use std::collections::HashMap;

pub struct EffectFactory {
    audio_config: AudioConfig,
    factory_exts: Vec<FactoryExtension>,
}

impl EffectFactory {
    pub fn assemble_factory(
        audio_config: AudioConfig,
        extensions: Vec<FactoryExtension>,
    ) -> EffectFactory {
        EffectFactory {
            audio_config,
            factory_exts: extensions,
        }
    }

    pub fn create_effect_unit(&self, name: &str) -> Option<Box<dyn AudioEffect>> {
        print!("Creating effect {}...", name);
        for factory_ext in &self.factory_exts {
            if factory_ext.factory_fns.contains_key(name) {
                print!("Success!\n");
                return Some((factory_ext.factory_fns[name].constructor)(
                    self.audio_config,
                ));
            }
        }

        print!("FAILED!\n");
        None
    }

    pub fn change_audio_config(&mut self, new_audio_config: AudioConfig) {
        self.audio_config = new_audio_config;
    }

    pub fn get_loaded_effect_names(&self) -> Vec<String> {
        let mut result = Vec::new();

        for factory_ext in &self.factory_exts {
            for (factory_fn_name, _) in &factory_ext.factory_fns {
                result.push(factory_fn_name.to_string());
            }
        }

        result
    }

    pub fn get_effect_infos_json(&self, format_prettily: bool) -> String {
        let mut result_map = HashMap::new();
        for effect_name in self.get_loaded_effect_names() {
            let effect_info = self.get_effect_info(&effect_name).unwrap();
            result_map.insert(effect_name, effect_info);
        }

        if format_prettily {
            serde_json::to_string_pretty(&result_map).unwrap()
        } else {
            serde_json::to_string(&result_map).unwrap()
        }
    }

    fn get_effect_info(&self, name: &str) -> Option<&'static [AdvertisedParameter]> {
        for factory_ext in &self.factory_exts {
            if factory_ext.factory_fns.contains_key(name) {
                return Some((factory_ext.factory_fns[name].info)());
            }
        }

        None
    }
}
