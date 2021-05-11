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

    fn set_pos(&mut self, index: i32) {
        self.input = index.rem_euclid(self.contents.len() as i32);
        self.output = index.rem_euclid(self.contents.len() as i32);
    }

    fn increment(&mut self) {
        self.output = (self.output + 1).rem_euclid(self.contents.len() as i32);
        self.input = (self.input + 1).rem_euclid(self.contents.len() as i32);
    }

    fn increment_out(&mut self) {
        self.output = (self.output + 1).rem_euclid(self.contents.len() as i32);
    }
}

struct Delay {
    buffer: Buffer,
}

impl Delay {
    pub fn new(time: i32, sample_rate: f32) -> Self {
        let mut buffer = Buffer::new(sample_rate as usize);
        
        let mut result = Self {
            buffer: buffer,
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
    pub fn new(time: i32, sample_rate: f32) -> Self {
        let mut buffer = Buffer::new(sample_rate as usize);
        
        let mut result = Self {
            buffer: buffer,
            time: time,
        };

        for x in 0..50 {
            result.set_time(time);
        }

        result
    }

    fn process(&mut self, input: f32) -> f32 {
        self.buffer.write(input);
        let out = self.buffer.read();
        self.buffer.increment();
        out
    }

    fn set_time(&mut self, time: i32) {
        self.buffer.output = (self.buffer.output - time).rem_euclid(self.buffer.contents.len() as i32);
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
    pub fn new(time: i32, feedback: f32, sample_rate: f32) -> Self {
        Self {
            initial_delay: Delay::new(time, sample_rate),
            feedback_delay: Delay::new(time, sample_rate),
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

    fn clear(&mut self) {
        self.initial_delay.buffer.contents.iter_mut().map(|x| *x = 0f32);
        self.feedback_delay.buffer.contents.iter_mut().map(|x| *x = 0f32);
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
    pub fn new(time: i32, feedback: f32, sample_rate: f32) -> Self {
        Self {
            initial_delay: RoundingErrorDelay::new(time, sample_rate),
            feedback_delay: RoundingErrorDelay::new(time, sample_rate),
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

enum DelayMode {
    Normal,
    DoubleInitial,
    DoubleInitialAndFeedback,
}

struct Lowpass {
    former_input: f32,
    former_output: f32,
    a: f32,
}

impl Lowpass{
    pub fn new(a: f32) -> Self {
        Self {
            former_input: 0f32,
            former_output: 0f32,
            a: a,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let out: f32 = ((1f32 - self.a) * self.former_output) + (self.a * ((input + self.former_input) / 2f32));
        self.former_input = input;
        self.former_output = out;

        out
    }

    fn set_a(&mut self, a: f32) {
        self.a = a;
    }
}

struct Granularverb {
    delay: DelayWithFeedback,
    filter: Lowpass,
}

impl Granularverb {
    pub fn new(length: f32, sample_rate: f32) -> Self {
        Self {
            delay: DelayWithFeedback::new((0.04109589f32 * (sample_rate - 1f32)) as i32, (length * 0.04f32) + 0.9f32, sample_rate),
            filter: Lowpass::new(0.04109589),
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.filter.process(self.delay.process(input)) + input
    }

    fn set_length(&mut self, length: f32) {
        self.delay.set_feedback((length * 0.09f32) + 0.9f32);
    }
}

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ReverbModel {
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "delay_time")]
        delay_time: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "delay_feedback")]
        delay_feedback: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "delay_wet_dry")]
        delay_wet_dry: f32,

        #[model(min = 0.0, max = 2.0)]
        #[parameter(name = "delay_mode")]
        delay_mode: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "reverb_length")]
        reverb_length: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "reverb_wet_dry")]
        reverb_wet_dry: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "final_cutoff")]
        final_cutoff: f32,
    }
}

impl Default for ReverbModel {
    fn default() -> Self {
        Self {
            delay_time: 1.0,
            delay_feedback: 0.5,
            delay_wet_dry: 0.5,
            delay_mode: 0.0,
            reverb_length: 0.5,
            reverb_wet_dry: 0.5,
            final_cutoff: 1.0,
        }
    }
}

struct Reverb {
    delay_left: DelayWithFeedback,
    reverb_left: Granularverb,
    filter_left: Lowpass,
    delay_right: DelayWithFeedback,
    reverb_right: Granularverb,
    filter_right: Lowpass,
    sample_rate: f32,
    mode: DelayMode,
}

impl Plugin for Reverb {
    const NAME: &'static str = "satin_demoverb";
    const PRODUCT: &'static str = "satin_demoverb";
    const VENDOR: &'static str = "audiodog301";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = ReverbModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &ReverbModel) -> Self {
        Self {
            delay_left: DelayWithFeedback::new((_model.delay_time * (_sample_rate - 1f32) as f32) as i32, _model.delay_feedback, _sample_rate),
            reverb_left: Granularverb::new(_model.reverb_length, _sample_rate),
            filter_left: Lowpass::new(_model.final_cutoff),
            delay_right: DelayWithFeedback::new((_model.delay_time * (_sample_rate - 1f32) as f32) as i32, _model.delay_feedback, _sample_rate),
            reverb_right: Granularverb::new(_model.reverb_length, _sample_rate),
            filter_right: Lowpass::new(_model.final_cutoff),
            sample_rate: _sample_rate,
            mode: DelayMode::Normal,
        }
    }

    #[inline]
    fn process(&mut self, model: &ReverbModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        
        for i in 0..ctx.nframes {          
            if model.delay_time.is_smoothing() {
                self.delay_left.set_time((model.delay_time[i] * (self.sample_rate - 1f32) as f32) as i32);
                self.delay_right.set_time((model.delay_time[i] * (self.sample_rate - 1f32) as f32) as i32);
            }
            if model.delay_feedback.is_smoothing() {
                self.delay_left.set_feedback(model.delay_feedback[i]);
                self.delay_right.set_feedback(model.delay_feedback[i]);
            }
            if model.reverb_length.is_smoothing() {
                self.reverb_left.set_length(model.reverb_length[i]);
                self.reverb_right.set_length(model.reverb_length[i]);
            }
            if model.final_cutoff.is_smoothing() {
                self.filter_left.set_a(model.final_cutoff[i]);
                self.filter_right.set_a(model.final_cutoff[i]);
            }

            let delay_left_current: f32 = (self.delay_left.process(input[0][i]) * model.delay_wet_dry[i]) + ((1f32 - model.delay_wet_dry[i]) * input[0][i]);
            let delay_right_current: f32 = (self.delay_right.process(input[1][i]) * model.delay_wet_dry[i]) + ((1f32 - model.delay_wet_dry[i]) * input[1][i]);

            let reverb_left_current: f32 = (model.reverb_wet_dry[i] * self.reverb_left.process(delay_left_current)) + ((1f32 - model.reverb_wet_dry[i]) * delay_left_current);
            let reverb_right_current: f32 = (model.reverb_wet_dry[i] * self.reverb_right.process(delay_right_current)) + ((1f32 - model.reverb_wet_dry[i]) * delay_right_current);
           
            output[0][i] = self.filter_left.process(reverb_left_current);
            output[1][i] = self.filter_right.process(reverb_right_current);
            
            if model.delay_mode[i] < 1f32 {
                if !matches!(self.mode, DelayMode::Normal) {
                    self.mode = DelayMode::Normal;
                    self.delay_left.clear();
                    self.delay_right.clear();

                    self.delay_left.initial_delay.buffer.set_pos(0);
                    self.delay_left.feedback_delay.buffer.set_pos(0);

                    self.delay_right.initial_delay.buffer.set_pos(0);
                    self.delay_right.feedback_delay.buffer.set_pos(0);

                    self.delay_left.set_time((model.delay_time[i] * (self.sample_rate - 1f32) as f32) as i32);
                    self.delay_right.set_time((model.delay_time[i] * (self.sample_rate - 1f32) as f32) as i32);
                }
            }            
            if model.delay_mode[i] >= 1f32 && model.delay_mode[i] < 2f32{
                self.mode = DelayMode::DoubleInitial;
            
                self.delay_left.initial_delay.buffer.increment_out();
                self.delay_right.initial_delay.buffer.increment_out();
            }
            if model.delay_mode[i] >= 2f32 && model.delay_mode[i] < 3f32 {
                self.delay_left.initial_delay.buffer.increment_out();
                self.delay_left.feedback_delay.buffer.increment_out();
                
                self.delay_right.initial_delay.buffer.increment_out();
                self.delay_right.feedback_delay.buffer.increment_out();
            }
        }
    }            
}

baseplug::vst2!(Reverb, b"sdov");