# Otters

AUDio Effects for Rust

## What is this?

Otters is a framework for creating reusable audio effects in Rust.


## Implemented Effects

* Bypass
* Basic Delay
* Modulated Delay Effects
    * Flanger
    * Chorus
    * Vibrato
* Non-linear Processing
    * Bit-Crusher
    * Wave-Shapers
* Phaser

Many of these effects are derived from algorithms presented in Will Pirkle's book: _Designing Audio Effect Plugins in C++_ 2nd Edition.

# Building

## What You Need

To build the otters_rt library:
* Rust (1.46 was used for development)
* A working GCC installation

## Setup

For otters_rt:

* Nothing, once you have Rust, you're good to go. Make sure you cloned the fftw submodule though.

## Build Steps

* In the root directory, run `cargo build --release`

If you'd like to build otters_runner for other architectures, you may need to specify the linker explicitly.

e.g. `RUSTFLAGS="-C linker=/opt/elk/1.0/sysroots/x86_64-pokysdk-linux/usr/bin/aarch64-elk-linux/aarch64-elk-linux-ld" cargo build --release --target=aarch64-unknown-linux-gnu`

# Configuration

Otters is configured with a JSON file describing the effects used, their default parameters, and how data flows between them. Here is an example.

```json
{
    "buffers": ["@SOURCE_0", "@SINK_0", "test_buf1"],
    "effects": [
        {
            "bind_name": "bypass1",
            "effect_name": "Bypass/Mono",
            "config": [],
            "enabled": true
        },
        {
            "bind_name": "delay1",
            "effect_name": "Delay/Basic",
            "config": [
                {"name": "delay_time_ms", "value": { "F": 500.0 }},
                {"name": "feedback_pct", "value": { "F": 0.2 }},
                {"name": "wet_dry_pct", "value": {"F": 0.5 }}
            ],
            "enabled": true
        }
    ],
    "connections": [
        {
            "effect": "bypass1",
            "reads": ["@SOURCE_0"],
            "writes": ["test_buf1"]
        },
        {
            "effect": "delay1",
            "reads": ["test_buf1"],
            "writes": ["@SINK_0"]
        }
    ]
}
```

## What each component is
### buffers
Declares the buffers that will be used to store intermediate data between effects. Buffers will be sized to the maximum block size the plugin will process at a time.

#### Special Buffers

* @SOURCE_N where N is [0,9] are buffers of data that come from outside of Otters.
* @SINK_N where N is [0,9] are buffers that will be passed back to the host for processing.

*It's important to not treat these as scratch area. The host may reuse memory space for source and sink buffers*

### effects
Declares the effects that will be used.

* bind_name: The name that will be used to refer to this effect in the *connections* list
* effect_name: The name of the actual effect. (e.g. bypass, distortion, phaser)
* config: The default parameters for the effect
    * name: the name of the parameter
    * value: the value of the parameter. This is an object containing a single key value pair.
        * Float parameter: Use "F" as the key and the float value as the value
        * Integer parameter: Use "N" as the key
        * String parameter: Use "S" as the key

### connections
Defines how data flows between effects. Each effect specifies which buffers it will read from and which buffers it will write to. **Connections are executed in the order they are provided.**

* effect: the *bind_name* of the effect
* reads: a list of buffers that this effect will read from
* writes: a list of buffers that this effect will write to

# Credits
* FFTW3 (licensed under GPL)
* Treiber Stack Implementation from synthesizer-io: https://github.com/raphlinus/synthesizer-io (used under Apache License)
* WyHash implementation: https://github.com/lemire/testingRNG/blob/master/source/wyhash.h (used under Apache License)
