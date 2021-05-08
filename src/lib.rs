
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use serde::{Serialize, Deserialize};

use baseplug::{
    ProcessContext,
    Plugin,
};

const MAX: i32 = 44_100;

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
        let mut buffer = Buffer::new(MAX as usize);
        
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

struct RoundingErrorDelay {
    buffer: Buffer,
    time: i32,
}

impl RoundingErrorDelay {
    pub fn new(time: i32) -> Self {
        let mut buffer = Buffer::new(MAX as usize);
        
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
        self.buffer.output = (self.buffer.output - time).rem_euclid(self.buffer.contents.len() as i32)
    }
}

struct DelayWithFeedback {
    initial_delay: Delay,
    feedback_delay: Delay,
    former_initial: f32,
    former_feedback: f32,
    feedback: f32,
}

impl DelayWithFeedback {
    pub fn new(time: i32, feedback: f32) -> Self {
        Self {
            initial_delay: Delay::new(time),
            feedback_delay: Delay::new(time),
            former_initial: 0f32,
            former_feedback: 0f32,
            feedback: feedback,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.former_initial = self.initial_delay.process(input + (self.feedback * self.former_feedback));
        self.former_feedback = self.feedback_delay.process(self.former_initial);

        self.former_initial
    }

    fn set_time(&mut self, time: i32) {
        self.initial_delay.set_time(time);
        self.feedback_delay.set_time(time);
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }
}

struct RoundingErrorDelayWithFeedback {
    initial_delay: RoundingErrorDelay,
    feedback_delay: RoundingErrorDelay,
    former_initial: f32,
    former_feedback: f32,
    feedback: f32,
}

impl RoundingErrorDelayWithFeedback {
    pub fn new(time: i32) -> Self {
        Self {
            initial_delay: RoundingErrorDelay::new(time),
            feedback_delay: RoundingErrorDelay::new(time),
            former_initial: 0f32,
            former_feedback: 0f32,
            feedback: 0.5f32,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.former_initial = self.initial_delay.process(input + (self.feedback * self.former_feedback));
        self.former_feedback = self.feedback_delay.process(self.former_initial);

        self.former_initial
    }

    fn set_time(&mut self, time: i32) {
        self.initial_delay.set_time(time);
        self.feedback_delay.set_time(time);
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }
}

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ReverbModel {
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "time")]
        time: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "feedback")]
        feedback: f32,
    }
}

impl Default for ReverbModel {
    fn default() -> Self {
        Self {
            time: 1.0,
            feedback: 0.5,
        }
    }
}

struct Reverb {
    delay_left: DelayWithFeedback,
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
            delay_left: DelayWithFeedback::new(_model.time as i32, _model.feedback),
        }
    }

    #[inline]
    fn process(&mut self, model: &ReverbModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        
        for i in 0..ctx.nframes {          
            if model.time.is_smoothing() {
                self.delay_left.set_time((model.time[i] * MAX as f32) as i32);
            }
            if model.feedback.is_smoothing() {
                self.delay_left.set_feedback(model.feedback[i]);
            }
           
            output[0][i] = self.delay_left.process(input[0][i]);
        }
    }            
}

baseplug::vst2!(Reverb, b"sdov");