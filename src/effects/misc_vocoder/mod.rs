mod robotize;
mod whisper;


// neither of these 2 effects are useful for guitar
// and whisper is particularly scary sounding

// Both effects are part of the vocoder example in Bela
pub use robotize::Robotize;
pub use whisper::Whisper;