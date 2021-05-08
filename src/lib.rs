
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use serde::{Serialize, Deserialize};

use baseplug::{
    ProcessContext,
    Plugin,
};

struct Buffer {
    contents: Vec<f32>,
    input: i32,
    output: i32,
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
        self.contents[self.output as usize]
    }

    fn write(&mut self, input: f32) {
        self.contents[self.input as usize] = input;
    }

    fn increment(&mut self) {
        self.output = (self.output + 1).rem_euclid(self.contents.len() as i32);
        self.input = (self.input + 1).rem_euclid(self.contents.len() as i32);
    }
}

struct Delay {
    buffer: Buffer,
    time: i32,
}

impl Delay {
    pub fn new(time: i32) -> Self {
        let mut buffer = Buffer::new(44_100);
        
        let mut result = Self {
            buffer: buffer,
            time: time,
        };

        result.set_time(time);

        result
    }

    fn process(&mut self, input: f32) -> f32 {
        self.buffer.write(input);
        let out = self.buffer.read();
        self.buffer.increment();
        out
    }

    fn set_time(&mut self, time: i32) {
        self.buffer.output = (self.buffer.input - time).rem_euclid(self.buffer.contents.len() as i32)
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
    feedback_left: Delay,
    delay_one: f32,
    delay_feedback: f32,
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
            feedback_left: Delay::new(44_100),
            delay_one: 0f32,
            delay_feedback: 0f32,
        }
    }

    #[inline]
    fn process(&mut self, model: &ReverbModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        
        for i in 0..ctx.nframes {          
            if model.time.is_smoothing() {
                self.delay_left.set_time((model.time[i] * 44_100f32) as i32);
                //self.delay_left.buffer.contents.iter_mut().map(|x| *x = 0f32).count();
            }

            self.delay_one = self.delay_left.process(input[0][i] + 0.5 * self.delay_feedback);
            self.delay_feedback = self.feedback_left.process(self.delay_one);
            
            output[0][i] = self.delay_one;
            output[1][i] = input[1][i];
        }
    }            
}

baseplug::vst2!(Reverb, b"sdov");