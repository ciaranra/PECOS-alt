use pecos_core::StructMetadata;
use serde_json::Value;
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug)]
pub enum ProcessingStage<M, O> {
    NeedsCoprocessing(M),
    Complete(O),
}

pub trait CoProcessor: Send + Sync + Debug + StructMetadata + Any {
    fn process(&mut self, input: Value) -> Value;
    fn clone_box(&self) -> Box<dyn CoProcessor>;
}

pub trait DrivingProcessor<Input: Debug + 'static, Output: Debug + 'static>:
    Send + Sync + Debug + StructMetadata + Any
{
    fn start(&mut self, input: Input) -> ProcessingStage<Value, Output>;
    fn continue_processing(&mut self, coprocessor_result: Value) -> ProcessingStage<Value, Output>;
    fn clone_box(&self) -> Box<dyn DrivingProcessor<Input, Output>>;
}

impl Clone for Box<dyn CoProcessor> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl<Input: Debug + 'static, Output: Debug + 'static> Clone
    for Box<dyn DrivingProcessor<Input, Output>>
{
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug)]
pub struct ProcessingSystem<D, C, Input, Output>
where
    D: DrivingProcessor<Input, Output> + Clone,
    C: CoProcessor + Clone,
    Input: Debug + 'static,
    Output: Debug + 'static,
{
    driver: D,
    coprocessor: C,
    _marker: PhantomData<(Input, Output)>,
}

impl<D, C, Input, Output> StructMetadata for ProcessingSystem<D, C, Input, Output>
where
    D: DrivingProcessor<Input, Output> + Clone,
    C: CoProcessor + Clone,
    Input: Debug + 'static,
    Output: Debug + 'static,
{
    fn name(&self) -> &str {
        "ProcessingSystem"
    }

    fn description(&self) -> &str {
        "A composed processing system"
    }
}

impl<D, C, Input, Output> Clone for ProcessingSystem<D, C, Input, Output>
where
    D: DrivingProcessor<Input, Output> + Clone,
    C: CoProcessor + Clone,
    Input: Debug + 'static,
    Output: Debug + 'static,
{
    fn clone(&self) -> Self {
        Self {
            driver: self.driver.clone(),
            coprocessor: self.coprocessor.clone(),
            _marker: PhantomData,
        }
    }
}

impl<D, C, Input, Output> ProcessingSystem<D, C, Input, Output>
where
    D: DrivingProcessor<Input, Output> + Debug + Clone + 'static,
    C: CoProcessor + Debug + Clone + 'static,
    Input: Debug,
    Output: Debug,
{
    pub fn new(driver: D, coprocessor: C) -> Self {
        Self {
            driver,
            coprocessor,
            _marker: PhantomData,
        }
    }

    pub fn process(&mut self, input: Input) -> Output {
        let mut stage = self.driver.start(input);

        while let ProcessingStage::NeedsCoprocessing(batch) = stage {
            let processed = self.coprocessor.process(batch);
            stage = self.driver.continue_processing(processed);
        }

        match stage {
            ProcessingStage::Complete(output) => output,
            ProcessingStage::NeedsCoprocessing(_) => unreachable!(),
        }
    }
}

pub type ProcessorStage<D, C> = ProcessingSystem<D, C, Value, Value>;

impl<D, C> CoProcessor for ProcessingSystem<D, C, Value, Value>
where
    D: DrivingProcessor<Value, Value> + Debug + Clone + 'static,
    C: CoProcessor + Debug + Clone + 'static,
{
    fn process(&mut self, input: Value) -> Value {
        self.process(input)
    }

    fn clone_box(&self) -> Box<dyn CoProcessor> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, StructMetadata)]
pub struct DynCoProcessor {
    inner: Box<dyn CoProcessor>,
}

impl DynCoProcessor {
    pub fn new(processor: Box<dyn CoProcessor>) -> Self {
        Self { inner: processor }
    }
}

impl CoProcessor for DynCoProcessor {
    fn process(&mut self, input: Value) -> Value {
        self.inner.process(input)
    }

    fn clone_box(&self) -> Box<dyn CoProcessor> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct DynDrivingProcessor {
    inner: Box<dyn DrivingProcessor<Value, Value>>,
}

impl DynDrivingProcessor {
    pub fn new(processor: Box<dyn DrivingProcessor<Value, Value>>) -> Self {
        Self { inner: processor }
    }
}

impl Clone for DynDrivingProcessor {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

impl StructMetadata for DynDrivingProcessor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }
}

impl DrivingProcessor<Value, Value> for DynDrivingProcessor {
    fn start(&mut self, input: Value) -> ProcessingStage<Value, Value> {
        self.inner.start(input)
    }

    fn continue_processing(&mut self, coprocessor_result: Value) -> ProcessingStage<Value, Value> {
        self.inner.continue_processing(coprocessor_result)
    }

    fn clone_box(&self) -> Box<dyn DrivingProcessor<Value, Value>> {
        Box::new(self.clone())
    }
}
