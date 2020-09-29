use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;

use super::conf::AudioConfig;
use super::conf::{BoardConfig, BoardConnectionDeclaration};
use super::errors::ContextInitError;
use super::otters::LoadedEffects;
use super::utils::buf_rw::{AudioBufferReader, AudioBufferWriter};
use super::utils::ringbuf::SimpleFloatBuffer;

const MAX_ALLOWABLE_BUF_DECLS: usize = 1024;
pub const MAX_ALLOWABLE_INPUTS: usize = 10;
pub const MAX_ALLOWABLE_OUTPUTS: usize = 10;

const MAX_EXTERNAL_INS: usize = 6;
const MAX_EXTERNAL_OUTS: usize = 6;

const FIRST_INPUT_IDX: usize = 1024;
const FIRST_OUTPUT_IDX: usize = 2048;

enum BufferUsageError<'a> {
    NoError,
    NoSuchBuffer(&'a String),
    BufferAlreadyUsed(&'a String),
}

impl<'a> fmt::Display for BufferUsageError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferUsageError::NoError => write!(f, "ok"),
            BufferUsageError::NoSuchBuffer(buf) => write!(f, "No such buffer {}", buf),
            BufferUsageError::BufferAlreadyUsed(buf) => write!(f, "Buffer already used {}", buf),
        }
    }
}

impl<'a> BufferUsageError<'a> {
    fn is_err(&self) -> bool {
        match self {
            BufferUsageError::NoError => false,
            _ => true,
        }
    }
}

struct BoardContextConstructionState {
    buf_name_to_idx: HashMap<String, usize>,
    num_external_buffers: usize,
}

impl BoardContextConstructionState {
    fn generate_idx_for_buf_name(&mut self, requested_buf_name: &str) -> Option<usize> {
        if requested_buf_name.starts_with("@SOURCE_") && requested_buf_name.len() > 8 {
            // 8 is the length of @SOURCE_
            let source_idx_str = &requested_buf_name[8..];
            if let Ok(parse_result) = source_idx_str.to_string().parse::<usize>() {
                if parse_result >= MAX_ALLOWABLE_INPUTS {
                    // exceeds max # of sources
                    return None;
                }

                self.num_external_buffers += 1;
                return Some(parse_result + FIRST_INPUT_IDX);
            } else {
                return None;
            }
        } else if requested_buf_name.starts_with("@SINK_") && requested_buf_name.len() > 6 {
            let sink_idx_str = &requested_buf_name[6..];
            if let Ok(parse_result) = sink_idx_str.to_string().parse::<usize>() {
                if parse_result >= MAX_ALLOWABLE_OUTPUTS {
                    return None;
                }
                
                self.num_external_buffers += 1;
                return Some(parse_result + FIRST_OUTPUT_IDX);
            } else {
                return None;
            }
        } else {
            return Some(self.buf_name_to_idx.len() - self.num_external_buffers);
        }
    }
}

pub struct BoardConnection {
    pub ordinal: usize,
    pub inputs_idxs: Vec<usize>,
    pub output_idxs: Vec<usize>,
}

pub struct BoardContext {
    buffers: Vec<RefCell<SimpleFloatBuffer>>,
    connections: Vec<BoardConnection>,
    external_ins: Vec<*const f32>,
    external_outs: Vec<*mut f32>,
}

impl BoardContext {
    pub fn initialize_context(
        board_config: &BoardConfig,
        audio_config: &AudioConfig,
        effects: &LoadedEffects,
    ) -> Result<BoardContext, ContextInitError> {
        let mut construction_state = create_construction_intermediate();
        let buffers = create_mem_buffers(
            &mut construction_state,
            &board_config.buffers,
            audio_config.max_block_size,
        )?;

        let connections =
            create_effect_connections(&mut construction_state, &board_config.connections, effects)?;

        let mut external_ins = Vec::new();
        let mut external_outs = Vec::new();

        for _ in 0..MAX_EXTERNAL_INS {
            external_ins.push(0 as *const f32);
        }

        for _ in 0..MAX_EXTERNAL_OUTS {
            external_outs.push(0 as *mut f32);
        }

        Ok(BoardContext {
            buffers,
            connections,
            external_ins,
            external_outs,
        })
    }

    pub fn bind_sink(&mut self, sink_idx: usize, sink_ptr: *mut f32) {
        if sink_idx >= MAX_ALLOWABLE_OUTPUTS {
            return;
        }

        self.external_outs[sink_idx] = sink_ptr;
    }

    pub fn bind_source(&mut self, source_idx: usize, source_ptr: *const f32) {
        if source_idx >= MAX_ALLOWABLE_INPUTS {
            return;
        }

        self.external_ins[source_idx] = source_ptr;
    }

    pub fn get_buffer_for_read<'a>(&'a self, buf_idx: usize) -> AudioBufferReader<'a> {
        if buf_idx >= FIRST_INPUT_IDX {
            if buf_idx >= FIRST_INPUT_IDX + MAX_ALLOWABLE_INPUTS {
                return AudioBufferReader::Null;
            }

            let norm_idx = buf_idx - FIRST_INPUT_IDX;
            if self.external_ins[norm_idx] == (0 as *const f32) {
                return AudioBufferReader::Null;
            }

            return AudioBufferReader::External(self.external_ins[norm_idx]);
        } else {
            if buf_idx >= self.buffers.len() {
                return AudioBufferReader::Null;
            }

            return AudioBufferReader::Internal(self.buffers[buf_idx].borrow());
        }
    }

    pub fn get_buffer_for_write<'a>(&'a self, buf_idx: usize) -> AudioBufferWriter<'a> {
        if buf_idx >= FIRST_OUTPUT_IDX {
            if buf_idx >= FIRST_OUTPUT_IDX + MAX_ALLOWABLE_OUTPUTS {
                return AudioBufferWriter::Null;
            }

            let norm_idx = buf_idx - FIRST_OUTPUT_IDX;
            if self.external_outs[norm_idx] == (0 as *mut f32) {
                return AudioBufferWriter::Null;
            }

            return AudioBufferWriter::External(self.external_outs[buf_idx - FIRST_OUTPUT_IDX]);
        } else {
            if buf_idx >= self.buffers.len() {
                return AudioBufferWriter::Null;
            }

            return AudioBufferWriter::Internal(self.buffers[buf_idx].borrow_mut());
        }
    }

    pub fn get_inputs_for_connection<'a>(&'a self, connection_idx: usize) -> &'a Vec<usize> {
        &self.connections[connection_idx].inputs_idxs
    }

    pub fn get_outputs_for_connection<'a>(&'a self, connection_idx: usize) -> &'a Vec<usize> {
        &self.connections[connection_idx].output_idxs
    }

    pub fn get_connections<'a>(&'a self) -> &'a Vec<BoardConnection> {
        return &self.connections;
    }
}

fn create_construction_intermediate() -> BoardContextConstructionState {
    BoardContextConstructionState {
        buf_name_to_idx: HashMap::new(),
        num_external_buffers: 0,
    }
}

fn create_mem_buffers(
    construction_helper: &mut BoardContextConstructionState,
    buf_names: &Vec<String>,
    max_block_size: usize,
) -> Result<Vec<RefCell<SimpleFloatBuffer>>, ContextInitError> {
    if buf_names.len() > MAX_ALLOWABLE_BUF_DECLS {
        return Err(ContextInitError(vec![format!(
            "Too many buffers. Requested {}. Max {}",
            buf_names.len(),
            MAX_ALLOWABLE_BUF_DECLS
        )]));
    }

    let mut result = Vec::with_capacity(buf_names.len());
    let mut errors: Vec<String> = Vec::new();

    for buf_name in buf_names {
        if construction_helper.buf_name_to_idx.contains_key(buf_name) {
            errors.push(format!("Redeclaration of buffer {}", buf_name));
        }

        let next_idx = construction_helper.generate_idx_for_buf_name(&buf_name);
        if let None = next_idx {
            errors.push(format!("Failed to generate idx for name {}", &buf_name));
            continue;
        }

        let next_idx = next_idx.unwrap();
        
        println!("Buffer Manager: Bind Buffer {} -> Ordinal {}", &buf_name, next_idx);

        construction_helper
            .buf_name_to_idx
            .insert(buf_name.clone(), next_idx);

        // only create a buffer if this is an internal buffer
        // external ones have special indexes and are backed by a buffer unknown to
        // the context
        if next_idx < MAX_ALLOWABLE_BUF_DECLS {
            result.push(RefCell::new(SimpleFloatBuffer::with_max_capacity(
                max_block_size,
            )));
        }
    }

    if errors.len() > 0 {
        Err(ContextInitError(errors))
    } else {
        Ok(result)
    }
}

fn create_effect_connections(
    construction_helper: &mut BoardContextConstructionState,
    connection_infos: &Vec<BoardConnectionDeclaration>,
    effects: &LoadedEffects,
) -> Result<Vec<BoardConnection>, ContextInitError> {
    let mut errors: Vec<String> = Vec::new();
    let mut used_buffer_tracker: HashSet<String> = HashSet::new();
    let mut connections: Vec<BoardConnection> = Vec::new();

    for connection_info in connection_infos {
        if !effects.contains_key(&connection_info.effect) {
            errors.push(format!(
                "Trying to connect nonexistent node {}",
                &connection_info.effect
            ));
            continue;
        }

        let (effect_ordinal, _, _) = &effects[&connection_info.effect];
        if *effect_ordinal >= effects.len() {
            errors.push(format!(
                "Effect {} has ordinal {}, which is > the max allowed ordinal {}",
                &connection_info.effect,
                *effect_ordinal,
                effects.len() - 1
            ));
            continue;
        }

        let mut input_target_idxs: Vec<usize> = Vec::new();
        let mut output_target_idxs: Vec<usize> = Vec::new();

        // find read buffers
        find_buffer_targets(
            &connection_info.reads,
            &mut input_target_idxs,
            &mut used_buffer_tracker,
            &construction_helper,
            &mut errors,
        );

        // find write buffers
        find_buffer_targets(
            &connection_info.writes,
            &mut output_target_idxs,
            &mut used_buffer_tracker,
            &construction_helper,
            &mut errors,
        );

        used_buffer_tracker.clear();

        println!("Connection Manager: Effect ordinal {} refers to {:?} for read, {:?} for write", *effect_ordinal, &input_target_idxs, &output_target_idxs);

        connections.push(BoardConnection {
            ordinal: *effect_ordinal,
            inputs_idxs: input_target_idxs,
            output_idxs: output_target_idxs,
        });
    }

    if errors.len() > 0 {
        Err(ContextInitError(errors))
    } else {
        Ok(connections)
    }
}

fn find_buffer_targets(
    targets: &Vec<String>,
    result_vec: &mut Vec<usize>,
    used_buffer_tracker: &mut HashSet<String>,
    helper: &BoardContextConstructionState,
    errors_acc: &mut Vec<String>,
) {
    for input_target in targets {
        let buffer_usage =
            is_valid_buffer(&helper.buf_name_to_idx, &used_buffer_tracker, input_target);
        if buffer_usage.is_err() {
            errors_acc.push(buffer_usage.to_string());
            continue;
        }

        used_buffer_tracker.insert(input_target.clone());

        result_vec.push(helper.buf_name_to_idx[input_target]);
    }
}

fn is_valid_buffer<'a>(
    buf_name_to_idx: &HashMap<String, usize>,
    used_buffers: &HashSet<String>,
    requested_buf: &'a String,
) -> BufferUsageError<'a> {
    if !buf_name_to_idx.contains_key(requested_buf) {
        BufferUsageError::NoSuchBuffer(requested_buf)
    } else if used_buffers.contains(requested_buf) {
        BufferUsageError::BufferAlreadyUsed(requested_buf)
    } else {
        BufferUsageError::NoError
    }
}
