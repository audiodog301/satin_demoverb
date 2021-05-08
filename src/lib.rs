
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use serde::{Serialize, Deserialize};

use baseplug::{
    ProcessContext,
    Plugin,
};

struct Buffer {
    contents: Vec<f32>,
    input: usize,
    output: usize,
}

impl Buffer {
    pub fn new(size: usize) -> Self {
        Self {
            contents: vec![0f32; size],
            input: 0,
            output: 0,
        }
    }

    fn read(&self) -> f32 {
        self.contents[self.output]
    }

    fn write(&mut self, input: f32) {
        self.contents[self.input] = input;
    }

    fn increment(&mut self) {
        self.output = (self.output + 1).rem_euclid(self.contents.len());
        self.input = (self.input + 1).rem_euclid(self.contents.len());
    }
}

struct Delay {
    buffer: Buffer,
    time: u32,
}

impl Delay {
    pub fn new(time: u32) -> Self {
        let mut buffer = Buffer::new(44_100);
        buffer.output = (buffer.output as i32 - time as i32).rem_euclid(buffer.contents.len() as i32) as usize;
        
        Self {
            buffer: buffer,
            time: time,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.buffer.write(input);
        let out = self.buffer.read();
        self.buffer.increment();
        out
    }

    fn set_time(&mut self, time: usize) {
        self.buffer.output = (self.buffer.output as i32 - time as i32).rem_euclid(self.buffer.contents.len() as i32) as usize
    }
}

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ReverbModel {
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "amount")]
        time: f32
    }
}

impl Default for ReverbModel {
    fn default() -> Self {
        Self {
            time: 1.0
        }
    }
}

struct Reverb {
    delay_left: Delay,
}

impl Plugin for Reverb {
    const NAME: &'static str = "Plugin";
    const PRODUCT: &'static str = "Plugin";
    const VENDOR: &'static str = "audiodog301";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = ReverbModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &ReverbModel) -> Self {
        Self {
            delay_left: Delay::new(44_100),
        }
    }

    #[inline]
    fn process(&mut self, model: &ReverbModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        
        for i in 0..ctx.nframes {          
            if model.time.is_smoothing() {
                self.delay_left.set_time((model.time[i] * 44_100f32) as usize);
                self.delay_left.buffer.contents.iter_mut().map(|x| *x = 0f32).count();
            }
            
            output[0][i] = (input[0][i] + self.delay_left.process(input[0][i])) / 2f32;
            output[1][i] = input[1][i];
        }
    }            
}

baseplug::vst2!(Reverb, b"sdov");