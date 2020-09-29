use crate::conf::AudioConfig;
use crate::otters::Otters;
use crate::OttersParamModifierContext;
use std::ffi;

pub type OttersString = *mut libc::c_char;

#[no_mangle]
pub extern "C" fn otters_hello(
    sample_rate: libc::c_float,
    max_block_size: libc::c_uint,
    config_file_name: *const libc::c_char,
) -> *mut Otters {
    if sample_rate <= 0f32 || max_block_size <= 0 {
        return 0 as *mut Otters;
    }

    let c_str = unsafe {
        if config_file_name.is_null() {
            return 0 as *mut Otters;
        }

        ffi::CStr::from_ptr(config_file_name)
    };

    let valid_rs_str = c_str.to_str();
    if let Err(_) = valid_rs_str {
        return 0 as *mut Otters;
    }

    let otters = Otters::create_default(
        AudioConfig {
            sample_rate: sample_rate as f32,
            max_block_size: max_block_size as usize,
        },
        valid_rs_str.unwrap(),
    );

    match otters {
        Ok(real_otters) => Box::into_raw(Box::new(real_otters)),
        Err(_) => 0 as *mut Otters,
    }
}

#[no_mangle]
pub extern "C" fn otters_bye(otters: *mut Otters) {
    if otters.is_null() {
        return;
    }

    unsafe {
        // take ownership of the data pointed to
        // and immediately drop the value
        Box::from_raw(otters);
    }
}

#[no_mangle]
pub extern "C" fn otters_update_audio_parameters(
    otters: *mut Otters,
    new_sample_rate: libc::c_float,
    new_max_block_size: libc::c_uint,
) {
    if otters.is_null() {
        return;
    }

    unsafe {
        let mut o: Box<Otters> = Box::from_raw(otters);
        let _ = o.update_audio_config(AudioConfig {
            sample_rate: new_sample_rate as f32,
            max_block_size: new_max_block_size as usize,
        });

        // don't accidentally delete the instance
        Box::into_raw(o);
    }
}

#[no_mangle]
pub extern "C" fn otters_bind_input(otters: *mut Otters, input_num: libc::c_uint, input_ptr: *const f32) {
    if otters.is_null() {
        return;
    }

    unsafe {
        let mut o: Box<Otters> = Box::from_raw(otters);
        o.bind_input(input_num as usize, input_ptr);

        Box::into_raw(o);
    }
}

#[no_mangle]
pub extern "C" fn otters_bind_output(otters: *mut Otters, output_num: libc::c_uint, output_ptr: *mut f32) {
    if otters.is_null() {
        return;
    }

    unsafe {
        let mut o: Box<Otters> = Box::from_raw(otters);
        o.bind_output(output_num as usize, output_ptr);

        Box::into_raw(o);
    }
}

#[no_mangle]
pub extern "C" fn otters_frolic(otters: *mut Otters, block_size: libc::c_uint) {
    if otters.is_null() {
        return;
    }

    unsafe {
        #[allow(unused_mut)]
        let mut o: Box<Otters> = Box::from_raw(otters);

        o.frolic(block_size as usize);

        Box::into_raw(o);
    }
}

// it's totally safe to use an OttersParamModifierContext even if the Otters object it's attached to dies.
// Allocation is also ok if necessary here as these functions will usually be called from a UI thread
#[no_mangle]
pub extern "C" fn otters_setup_async_param_updater(otters: *mut Otters) -> *mut OttersParamModifierContext {
    if otters.is_null() {
        return 0 as *mut OttersParamModifierContext;
    }

    unsafe {
        let mut o: Box<Otters> = Box::from_raw(otters);

        let ctx = o.setup_async_param_updater();

        Box::into_raw(o);

        Box::into_raw(Box::new(ctx))
    }
}

#[no_mangle]
pub extern "C" fn otters_free_async_param_updater(u: *mut OttersParamModifierContext) {
    if u.is_null() {
        return;
    }

    unsafe {
        Box::from_raw(u);
    }
}

#[no_mangle]
pub extern "C" fn otters_free_string(s: OttersString) {
    if s.is_null() {
        return;
    }

    unsafe {
        ffi::CString::from_raw(s);
    }
}

#[no_mangle]
pub extern "C" fn param_get_session_info_json(pu: *mut OttersParamModifierContext) -> OttersString {
    if pu.is_null() {
        return 0 as OttersString;
    }

    unsafe {
        let u: Box<OttersParamModifierContext> = Box::from_raw(pu);
        let result = u.get_session_info_json();
        let cstr_result_ptr = str_ref_to_cstr(&result);
        Box::into_raw(u);

        cstr_result_ptr
    }
}

#[no_mangle]
pub extern "C" fn otters_get_capabilities_json(format_prettily: bool) -> OttersString {
    let json = Otters::get_effect_info_json(format_prettily);
    str_ref_to_cstr(&json)
}

#[no_mangle]
pub extern "C" fn param_set_flt_param_value(pu: *mut OttersParamModifierContext, global_param_idx: u32, value: libc::c_float) {
    if pu.is_null() {
        return;
    }

    unsafe {
        let u = Box::from_raw(pu);

        u.set_flt_param_value(global_param_idx, value as f32);

        Box::into_raw(u);
    }
}

#[no_mangle]
pub extern "C" fn param_set_int_param_value(pu: *mut OttersParamModifierContext, global_param_idx: u32, value: libc::c_int) {
    if pu.is_null() {
        return;
    }

    unsafe {
        let u = Box::from_raw(pu);
        u.set_int_param_value(global_param_idx, value as i32);

        Box::into_raw(u);
    }
}

fn str_ref_to_cstr(s: &str) -> OttersString {
    let cstr_s = ffi::CString::new(s).unwrap();
    cstr_s.into_raw()
}
